use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::{fsops, paths, registry, vdf};

const STEAM_REG: &str = r"HKCU\Software\Valve\Steam";

#[derive(Clone, Debug)]
pub struct SteamAccount {
    pub id64: String,
    pub login: String,
    pub persona: String,
    pub most_recent: bool,
}

pub fn root() -> Option<PathBuf> {
    if let Some(p) = registry::read_string(STEAM_REG, "SteamPath") {
        let path = PathBuf::from(p.replace('/', "\\"));
        if path.exists() {
            return Some(path);
        }
    }
    for c in [
        r"%ProgramFiles(x86)%\Steam",
        r"%ProgramFiles%\Steam",
        r"C:\Steam",
    ] {
        let p = paths::expand(c);
        if p.join("steam.exe").exists() {
            return Some(p);
        }
    }
    None
}

fn login_users_path() -> Option<PathBuf> {
    Some(root()?.join("config").join("loginusers.vdf"))
}

pub fn accounts() -> Result<Vec<SteamAccount>> {
    let Some(path) = login_users_path() else {
        return Ok(Vec::new());
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let root = vdf::parse(&text);
    let Some(users) = root.get("users") else {
        return Ok(Vec::new());
    };
    Ok(users
        .entries()
        .iter()
        .map(|(id64, node)| SteamAccount {
            id64: id64.clone(),
            login: node
                .get("AccountName")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            persona: node
                .get("PersonaName")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            most_recent: node
                .get("MostRecent")
                .and_then(|v| v.as_str())
                .is_some_and(|v| v == "1"),
        })
        .collect())
}

pub fn current_account() -> Result<Option<String>> {
    let list = accounts()?;
    if let Some(a) = list.iter().find(|a| a.most_recent) {
        return Ok(Some(a.id64.clone()));
    }

    let auto = registry::read_string(STEAM_REG, "AutoLoginUser").unwrap_or_default();
    Ok(list
        .iter()
        .find(|a| !auto.is_empty() && a.login.eq_ignore_ascii_case(&auto))
        .map(|a| a.id64.clone()))
}

pub fn select_account(id64: &str, skip_chooser: bool) -> Result<()> {
    let path = login_users_path().context("لم أجد مجلد ستيم")?;
    let text = fs::read_to_string(&path).context("تعذّرت قراءة loginusers.vdf")?;
    let mut tree = vdf::parse(&text);

    let mut login_name = String::new();
    {
        let users = tree
            .get_mut("users")
            .context("ملف حسابات ستيم بصيغة غير متوقعة")?;
        let mut found = false;
        for (id, node) in users.entries().to_vec() {
            let _ = node;
            let selected = id == id64;
            if let Some(entry) = users.get_mut(&id) {
                entry.set("MostRecent", if selected { "1" } else { "0" });
                entry.set("AllowAutoLogin", if selected { "1" } else { "0" });
                if selected {
                    entry.set("RememberPassword", "1");
                    login_name = entry
                        .get("AccountName")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    found = true;
                }
            }
        }
        anyhow::ensure!(found, "ستيم لا يعرف هذا الحساب على هذا الجهاز");
    }

    let backup = path.with_extension("vdf.badeel-bak");
    let _ = fs::copy(&path, &backup);

    let staged = paths::temp_sibling(&path, "new");
    fs::write(&staged, vdf::write(&tree))?;
    fsops::swap_in(&staged, &path)?;

    registry::write_string(STEAM_REG, "AutoLoginUser", &login_name)?;
    if skip_chooser {
        if let Some(r) = root() {
            let _ = set_user_chooser(&r, false);
        }
    }

    let _ = registry::write(STEAM_REG, "RememberPassword", &registry::RegValue::Dword(1));
    Ok(())
}

pub fn set_user_chooser(root_dir: &std::path::Path, show: bool) -> Result<bool> {
    let path = root_dir.join("config").join("config.vdf");
    let text = fs::read_to_string(&path).context("config.vdf")?;
    let want = if show { "1" } else { "0" };
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };

    let mut out = String::with_capacity(text.len() + 8);
    let mut changed = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if !changed && trimmed.contains("\"AlwaysShowUserChooser\"") {
            if trimmed.rsplit('\"').nth(1) == Some(want) {
                return Ok(false);
            }
            let indent: String = trimmed.chars().take_while(|c| c.is_whitespace()).collect();
            out.push_str(&format!(
                "{indent}\"AlwaysShowUserChooser\"\t\t\"{want}\"{nl}"
            ));
            changed = true;
        } else {
            out.push_str(line);
        }
    }
    if !changed {
        return Ok(false);
    }

    let backup = path.with_extension("vdf.badeel-bak");
    let _ = fs::copy(&path, &backup);
    let staged = paths::temp_sibling(&path, "cfg");
    fs::write(&staged, out)?;
    fsops::swap_in(&staged, &path)?;
    Ok(true)
}

pub fn avatar_path(id64: &str) -> Option<PathBuf> {
    let p = root()?
        .join("config")
        .join("avatarcache")
        .join(format!("{id64}.png"));
    p.exists().then_some(p)
}
