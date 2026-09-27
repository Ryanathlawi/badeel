use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::catalog::{Identity, Item, Platform};
use super::vault::VaultKey;
use super::{bnet, fsops, paths, procs, steam, store, vault};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Closing,
    Saving,
    Restoring,
    Launching,
    Done,
}

pub fn current_id(p: &Platform) -> Option<String> {
    match p.identity {
        Identity::Marker(path) => {
            let s = fs::read_to_string(paths::expand(path)).ok()?;
            let s = s.trim().to_string();
            (!s.is_empty()).then_some(s)
        }
        Identity::Steam => steam::current_account().ok().flatten(),
        Identity::Bnet => bnet::current_account(),
    }
}

/// هل في المنصّة جلسة قائمة الآن؟ ملفّات الجلسة موجودة وفيها شيء
/// تُسأل عن المنصّات التي لا تحفظ قائمة حسابات، فهي لا تُعرف إلا بملفّاتها
pub fn session_live(p: &Platform) -> bool {
    p.items.iter().any(|item| match item {
        Item::File(path) => paths::expand(path)
            .metadata()
            .is_ok_and(|m| m.len() > 0),
        Item::Dir(path) => paths::expand(path).is_dir(),
        Item::Reg(key, value) => super::registry::read(key, value)
            .ok()
            .flatten()
            .is_some(),
    })
}

fn account_slot(p: &Platform, account_id: &str, index: usize, item: &Item) -> PathBuf {
    paths::account_dir(p.id, account_id).join(item.slot(index))
}

pub fn capture(p: &Platform, account_id: &str, key: &VaultKey) -> Result<()> {
    let dir = paths::account_dir(p.id, account_id);
    fs::create_dir_all(&dir)?;

    for (i, item) in p.items.iter().enumerate() {
        let dest = account_slot(p, account_id, i, item);
        match item {
            Item::File(path) | Item::Dir(path) => {
                let live = paths::expand(path);
                if !live.exists() {

                    fsops::remove_any(&dest)?;
                    continue;
                }
                let staged = paths::temp_sibling(&dest, "cap");
                vault::seal_path(key, &live, &staged)
                    .with_context(|| format!("حفظ {}", live.display()))?;
                fsops::swap_in(&staged, &dest)?;
            }
            Item::Reg(reg_key, reg_value) => {
                match super::registry::read(reg_key, reg_value)? {
                    Some(v) => key.write_file(&dest, &serde_json::to_vec(&v)?)?,
                    None => fsops::remove_any(&dest)?,
                }
            }
        }
    }

    write_marker(p, account_id)?;
    Ok(())
}

pub fn apply(p: &Platform, account_id: &str, key: &VaultKey) -> Result<()> {
    let work = paths::work_dir(p.id).join(format!("rollback-{}", store::now_secs()));
    fs::create_dir_all(&work)?;

    let mut backups = Vec::new();
    for (i, item) in p.items.iter().enumerate() {
        if let Some(raw) = item.raw_path() {
            let live = paths::expand(raw);
            backups.push(fsops::Backup::capture(&live, &work, &item.slot(i))?);
        }
    }
    if let Identity::Marker(path) = p.identity {
        let live = paths::expand(path);
        backups.push(fsops::Backup::capture(&live, &work, "marker")?);
    }

    let result = (|| -> Result<()> {
        for (i, item) in p.items.iter().enumerate() {
            let src = account_slot(p, account_id, i, item);
            match item {
                Item::File(path) | Item::Dir(path) => {
                    let live = paths::expand(path);
                    if !src.exists() {
                        fsops::remove_any(&live)?;
                        continue;
                    }
                    let staged = paths::temp_sibling(&live, "new");
                    vault::unseal_path(key, &src, &staged)
                        .with_context(|| format!("تحضير {}", live.display()))?;
                    fsops::swap_in(&staged, &live)
                        .with_context(|| format!("تركيب {}", live.display()))?;
                }
                Item::Reg(reg_key, reg_value) => {
                    if !src.exists() {
                        continue;
                    }
                    let bytes = key.read_file(&src)?;
                    let v: super::registry::RegValue = serde_json::from_slice(&bytes)?;
                    super::registry::write(reg_key, reg_value, &v)?;
                }
            }
        }
        write_marker(p, account_id)
    })();

    match result {
        Ok(()) => {
            let _ = fs::remove_dir_all(&work);
            Ok(())
        }
        Err(e) => {
            log::error!("فشل التركيب ({e}) — أرجع الحالة السابقة");
            let mut restore_errors = Vec::new();
            for b in backups.iter().rev() {
                if let Err(re) = b.restore() {
                    restore_errors.push(format!("{}: {re}", b.target.display()));
                }
            }
            if restore_errors.is_empty() {
                Err(e.context("فشل التبديل — أُرجعت الحالة السابقة كما كانت"))
            } else {

                Err(e.context(format!(
                    "فشل التبديل وتعذّر الإرجاع {}، ونسخة الحالة السابقة في {}",
                    restore_errors.join("، "),
                    work.display()
                )))
            }
        }
    }
}

fn write_marker(p: &Platform, account_id: &str) -> Result<()> {
    if let Identity::Marker(path) = p.identity {
        let live = paths::expand(path);
        if let Some(parent) = live.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&live, account_id.as_bytes())?;
    }
    Ok(())
}

pub fn switch_to(
    p: &Platform,
    target_id: &str,
    launch_after: bool,
    skip_steam_chooser: bool,
    key: &VaultKey,
    mut on_step: impl FnMut(Step),
) -> Result<()> {
    let mut index = store::load(p.id);
    anyhow::ensure!(index.get(target_id).is_some(), "حساب غير معروف");

    let current = current_id(p);
    if current.as_deref() == Some(target_id) && !launch_after {
        on_step(Step::Done);
        return Ok(());
    }

    on_step(Step::Closing);
    procs::close_all(p.exes, p.close)?;

    if p.identity.own_list() {
        on_step(Step::Restoring);
        match p.identity {
            Identity::Bnet => bnet::select_account(target_id)?,
            _ => steam::select_account(target_id, skip_steam_chooser)?,
        }
    } else {

        if let Some(cur) = current.as_deref() {
            if cur != target_id && index.get(cur).is_some() {
                on_step(Step::Saving);
                capture(p, cur, key)?;
            }
        }
        on_step(Step::Restoring);
        apply(p, target_id, key)?;
    }

    if let Some(acc) = index.accounts.iter_mut().find(|a| a.id == target_id) {
        acc.last_used = store::now_secs();
        acc.uses = acc.uses.saturating_add(1);
    }
    store::save(p.id, &index)?;

    if launch_after {
        on_step(Step::Launching);
        procs::launch(p, &[])?;
    }
    on_step(Step::Done);
    Ok(())
}

pub fn add_current(p: &Platform, name: &str, key: &VaultKey) -> Result<store::Account> {
    let id = current_id(p)
        .or_else(|| matches!(p.identity, Identity::Marker(_)).then(new_marker_id))
        .context("لا يوجد حساب مسجَّل دخوله الآن")?;

    let mut index = store::load(p.id);
    let acc = store::Account {
        id: id.clone(),
        name: name.trim().to_string(),
        note: String::new(),
        avatar: None,
        last_used: store::now_secs(),
        uses: 0,
    };
    index.upsert(acc.clone());
    store::save(p.id, &index)?;

    if !p.identity.own_list() {
        write_marker(p, &id)?;
        capture(p, &id, key)?;
    }
    Ok(acc)
}

fn new_marker_id() -> String {

    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{t:x}-{:x}", N.fetch_add(1, Ordering::Relaxed))
}

pub fn forget(p: &Platform, account_id: &str) -> Result<()> {
    let mut index = store::load(p.id);
    index.remove(account_id);
    store::save(p.id, &index)?;
    fsops::remove_any(&paths::account_dir(p.id, account_id))?;
    Ok(())
}
