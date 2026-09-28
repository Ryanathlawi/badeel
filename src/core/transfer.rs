//! نقل الحسابات إلى جهاز آخر أو إلى ويندوز جديد بعد الفرمتة
//!
//! خزنة بديل مربوطة بحساب ويندوز وبالجهاز فلا تُفتح في غيرهما، وهذا ما يحميها،
//! فالنقل يفكّ كل ملف بمفتاح الخزنة ويجمع الكل في ملف واحد مقفل بكلمة سر
//! يختارها المستخدم، والاستيراد يعكس ذلك ثم يقفل كل ملف بخزنة الجهاز الجديد

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::vault::{self, VaultKey};
use super::{catalog, fsops, paths, store};

const FORMAT: &str = "badeel-transfer";
const VERSION: u32 = 1;
const PACK: &[u8; 4] = b"BDLX";

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    sealed: vault::Sealed,
}

pub struct Moved {
    pub added: usize,
    pub skipped: usize,
}

/// ملفّات بمساراتها في كتلة واحدة: طول المسار ثم المسار ثم طول البيانات ثم البيانات
struct Pack(Zeroizing<Vec<u8>>);

impl Pack {
    fn new() -> Self {
        let mut v = Zeroizing::new(Vec::new());
        v.extend_from_slice(PACK);
        Pack(v)
    }

    fn put(&mut self, path: &str, data: &[u8]) {
        for part in [path.as_bytes(), data] {
            self.0.extend_from_slice(&(part.len() as u32).to_le_bytes());
            self.0.extend_from_slice(part);
        }
    }
}

fn unpack(bytes: &[u8]) -> Result<Vec<(String, Zeroizing<Vec<u8>>)>> {
    fn take<'a>(rest: &mut &'a [u8]) -> Result<&'a [u8]> {
        anyhow::ensure!(rest.len() >= 4, "ملف النقل ناقص");
        let n = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        anyhow::ensure!(rest.len() - 4 >= n, "ملف النقل ناقص");
        let (chunk, tail) = rest[4..].split_at(n);
        *rest = tail;
        Ok(chunk)
    }
    let mut rest = bytes
        .strip_prefix(PACK.as_slice())
        .context("محتوى ملف النقل غير معروف")?;
    let mut out = Vec::new();
    while !rest.is_empty() {
        let path = std::str::from_utf8(take(&mut rest)?)
            .context("مسار غير صالح في ملف النقل")?
            .to_string();
        out.push((path, Zeroizing::new(take(&mut rest)?.to_vec())));
    }
    Ok(out)
}

/// مسار جاء من ملف لا يُوثق به حتى يُفحص: أجزاؤه أسماء عادية لا تصعد ولا تقفز
/// إلى قرص آخر، ولولا هذا لكتب ملف مصنوع حيث يشاء على الجهاز
fn safe_rel(path: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." || paths::sanitize(part) != part {
            return None;
        }
        out.push(part);
    }
    Some(out)
}

/// يفكّ ملفّات حساب واحد من الخزنة ويضعها في الكتلة بمساراتها
fn unseal_into(key: &VaultKey, dir: &Path, base: &str, pack: &mut Pack) -> Result<()> {
    // حساب بلا ملفّات، كحسابات ستيم وباتل نت التي تحفظها منصّتها بنفسها
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        // بقايا كتابة لم تكتمل
        if name.contains(".badeel-") {
            continue;
        }
        let path = entry.path();
        let rel = format!("{base}/{name}");
        if path.is_dir() {
            unseal_into(key, &path, &rel, pack)?;
        } else {
            pack.put(&rel, &key.read_file(&path)?);
        }
    }
    Ok(())
}

/// يجمع حسابات الملف الشخصي الحالي كلها في ملف واحد، ويرجع عددها
pub fn export(key: &VaultKey, password: &str, dest: &Path) -> Result<usize> {
    let mut pack = Pack::new();
    let mut count = 0;
    let avatars = paths::profile_root().join("avatars");
    for p in catalog::PLATFORMS {
        let index = store::load(p.id);
        if index.accounts.is_empty() {
            continue;
        }
        pack.put(&format!("{}/index.json", p.id), &serde_json::to_vec(&index)?);
        for acc in &index.accounts {
            count += 1;
            // ستيم وباتل نت يحفظان جلساتهما بنفسيهما، فلا يُنقل لهما إلا الاسم
            if !p.identity.own_list() {
                let base = format!("{}/{}", p.id, paths::sanitize(&acc.id));
                unseal_into(key, &paths::account_dir(p.id, &acc.id), &base, &mut pack)?;
            }
            // الصور التي اختارها المستخدم بنفسه، أما صور ستيم فتعود وحدها
            if let Some(file) = acc
                .avatar
                .as_deref()
                .map(Path::new)
                .filter(|f| f.starts_with(&avatars))
            {
                if let (Some(name), Ok(bytes)) = (file.file_name(), fs::read(file)) {
                    pack.put(&format!("@avatars/{}", name.to_string_lossy()), &bytes);
                }
            }
        }
    }
    anyhow::ensure!(count > 0, "ما فيه حسابات محفوظة تنتقل");

    let env = Envelope {
        format: FORMAT.into(),
        version: VERSION,
        sealed: vault::seal_with(password, &pack.0)?,
    };
    let staged = paths::temp_sibling(dest, "transfer");
    fs::write(&staged, serde_json::to_vec_pretty(&env)?)?;
    fsops::swap_in(&staged, dest)?;
    Ok(count)
}

/// يضيف ما في الملف إلى الملف الشخصي الحالي، وما كان موجودًا هنا لا يُمسّ
pub fn import(key: &VaultKey, password: &str, src: &Path) -> Result<Moved> {
    let raw = fs::read(src).context("تعذّرت قراءة الملف")?;
    let env: Envelope = serde_json::from_slice(&raw)
        .ok()
        .filter(|e: &Envelope| e.format == FORMAT)
        .context("هذا مو ملف نقل من بديل")?;
    anyhow::ensure!(
        env.version <= VERSION,
        "الملف من نسخة أحدث من بديل، حدّث البرنامج أولًا"
    );
    let plain = vault::open_with(password, &env.sealed)?;
    let records = unpack(&plain)?;
    let find = |path: &str| records.iter().find(|(p, _)| p == path).map(|(_, d)| d);

    let avatars = paths::profile_root().join("avatars");
    let mut moved = Moved {
        added: 0,
        skipped: 0,
    };
    for p in catalog::PLATFORMS {
        let Some(raw_index) = find(&format!("{}/index.json", p.id)) else {
            continue;
        };
        let incoming: store::Index =
            serde_json::from_slice(raw_index).context("سجلّ حسابات تالف في ملف النقل")?;
        let mut local = store::load(p.id);
        let before = moved.added;
        for mut acc in incoming.accounts {
            if local.get(&acc.id).is_some() {
                moved.skipped += 1;
                continue;
            }
            let base = format!("{}/{}/", p.id, paths::sanitize(&acc.id));
            for (path, data) in records.iter().filter(|(path, _)| path.starts_with(&base)) {
                let rel = safe_rel(&path[p.id.len() + 1..])
                    .context("ملف النقل فيه مسار غير آمن، فلم يُستورد شيء منه")?;
                key.write_file(&paths::accounts_root(p.id).join(rel), data)?;
            }
            acc.avatar = acc
                .avatar
                .as_deref()
                .and_then(|a| Path::new(a).file_name())
                .map(|n| n.to_string_lossy().to_string())
                .and_then(|name| {
                    let data = find(&format!("@avatars/{name}"))?;
                    let dest = avatars.join(safe_rel(&name)?);
                    fs::create_dir_all(&avatars).ok()?;
                    fs::write(&dest, data.as_slice()).ok()?;
                    Some(dest.to_string_lossy().to_string())
                });
            local.upsert(acc);
            moved.added += 1;
        }
        if moved.added > before {
            store::save(p.id, &local)?;
        }
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_gives_back_exactly_what_went_in() {
        let mut pack = Pack::new();
        pack.put("riot/abc/00-settings.yaml", b"cookie: 1");
        pack.put("@avatars/x.png", &[0u8, 255, 7]);
        pack.put("steam/index.json", b"");
        let got = unpack(&pack.0).unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].0, "riot/abc/00-settings.yaml");
        assert_eq!(got[0].1.as_slice(), b"cookie: 1");
        assert_eq!(got[1].1.as_slice(), &[0u8, 255, 7]);
        assert!(got[2].1.is_empty());
    }

    #[test]
    fn a_truncated_or_foreign_pack_is_refused() {
        let mut pack = Pack::new();
        pack.put("a/b", b"data");
        let bytes = pack.0.to_vec();
        assert!(unpack(&bytes[..bytes.len() - 1]).is_err());
        assert!(unpack(b"XXXX").is_err());
    }

    #[test]
    fn a_path_from_a_file_cannot_climb_out_or_jump_drives() {
        assert!(safe_rel("abc/00-x.yaml").is_some());
        for bad in ["../x", "a/../b", "a/./b", "a//b", "C:/x", "a\\b", "", "a/"] {
            assert!(safe_rel(bad).is_none(), "{bad}");
        }
    }
}
