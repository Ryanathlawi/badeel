#![allow(deprecated)]

use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::paths;

const MAGIC: &[u8; 4] = b"BDL1";
const NONCE_LEN: usize = 12;

fn fill_random(buf: &mut [u8]) {
    getrandom::fill(buf).expect("random source");
}

#[derive(Serialize, Deserialize, Default)]
struct VaultFile {

    wrapped_key: String,

    #[serde(default)]
    kdf: Option<Kdf>,

    check: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Kdf {
    salt: String,
    mem_kib: u32,
    passes: u32,
    lanes: u32,
}

impl Default for Kdf {
    fn default() -> Self {

        Self {
            salt: String::new(),
            mem_kib: 64 * 1024,
            passes: 3,
            lanes: 1,
        }
    }
}

fn vault_path() -> PathBuf {
    paths::data_root().join("vault.json")
}

pub struct VaultKey(Zeroizing<[u8; 32]>);

impl VaultKey {
    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.0.as_ref()))
    }

    pub fn encrypt(&self, plain: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        fill_random(&mut nonce);
        let ct = self
            .cipher()
            .encrypt(Nonce::from_slice(&nonce), plain)
            .map_err(|_| anyhow::anyhow!("فشل التشفير"))?;
        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + ct.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn decrypt(&self, blob: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        if blob.len() < MAGIC.len() + NONCE_LEN || &blob[..4] != MAGIC {
            bail!("ملف غير معروف أو تالف");
        }
        let nonce = &blob[4..4 + NONCE_LEN];
        let ct = &blob[4 + NONCE_LEN..];
        let plain = self
            .cipher()
            .decrypt(Nonce::from_slice(nonce), ct)

            .map_err(|_| anyhow::anyhow!("تعذّر فك التشفير — الملف تالف أو مُعدَّل"))?;
        Ok(Zeroizing::new(plain))
    }

    pub fn write_file(&self, path: &Path, plain: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.encrypt(plain)?)?;
        Ok(())
    }

    pub fn read_file(&self, path: &Path) -> Result<Zeroizing<Vec<u8>>> {
        let blob = fs::read(path).with_context(|| format!("قراءة {}", path.display()))?;
        self.decrypt(&blob)
    }
}

pub fn seal_path(key: &VaultKey, live: &Path, dest: &Path) -> Result<()> {
    if live.is_dir() {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(live)? {
            let entry = entry?;
            seal_path(key, &entry.path(), &dest.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        let plain = Zeroizing::new(fs::read(live)?);
        key.write_file(dest, &plain)
    }
}

pub fn unseal_path(key: &VaultKey, src: &Path, staged: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(staged)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            unseal_path(key, &entry.path(), &staged.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        let plain = key.read_file(src)?;
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(staged, plain.as_slice())?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {

    Fresh,

    Auto,

    Locked,
}

pub fn state() -> State {
    match read_vault() {
        None => State::Fresh,
        Some(v) if v.kdf.is_some() => State::Locked,
        Some(_) => State::Auto,
    }
}

fn read_vault() -> Option<VaultFile> {
    let raw = fs::read(vault_path()).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn write_vault(v: &VaultFile) -> Result<()> {
    let path = vault_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = paths::temp_sibling(&path, "vault");
    fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    super::fsops::swap_in(&tmp, &path)?;
    Ok(())
}

pub fn open(password: Option<&str>) -> Result<VaultKey> {
    match read_vault() {
        None => create(password),
        Some(v) => {
            let mut wrapped = b64_decode(&v.wrapped_key)?;
            let raw = match (&v.kdf, password) {
                (Some(kdf), Some(pw)) => {
                    let pk = derive(pw, kdf)?;
                    let unwrapped = aes_open(pk.as_ref(), &wrapped)?;
                    dpapi_unprotect(&unwrapped)?
                }
                (Some(_), None) => bail!("الخزنة مقفلة — أدخل كلمة السر"),
                (None, _) => dpapi_unprotect(&wrapped)?,
            };
            wrapped.zeroize();
            let mut key = [0u8; 32];
            if raw.len() != 32 {
                bail!("مفتاح الخزنة تالف");
            }
            key.copy_from_slice(&raw);
            let vk = VaultKey(Zeroizing::new(key));

            let check = b64_decode(&v.check)?;
            vk.decrypt(&check).context("كلمة السر غير صحيحة")?;
            Ok(vk)
        }
    }
}

fn create(password: Option<&str>) -> Result<VaultKey> {
    let mut key = [0u8; 32];
    fill_random(&mut key);
    let vk = VaultKey(Zeroizing::new(key));

    let protected = dpapi_protect(&key)?;
    let (wrapped, kdf) = match password {
        Some(pw) => {
            let kdf = Kdf {
                salt: random_b64(16),
                ..Kdf::default()
            };
            let pk = derive(pw, &kdf)?;
            let w = aes_seal(pk.as_ref(), &protected)?;
            (w, Some(kdf))
        }
        None => (protected, None),
    };

    let file = VaultFile {
        wrapped_key: b64_encode(&wrapped),
        kdf,
        check: b64_encode(&vk.encrypt(b"badeel-check")?),
    };
    write_vault(&file)?;
    Ok(vk)
}

pub fn set_password(current: Option<&str>, new: Option<&str>) -> Result<()> {
    let vk = open(current)?;
    let protected = dpapi_protect(vk.0.as_ref())?;
    let (wrapped, kdf) = match new {
        Some(pw) => {
            let kdf = Kdf {
                salt: random_b64(16),
                ..Kdf::default()
            };
            (aes_seal(derive(pw, &kdf)?.as_ref(), &protected)?, Some(kdf))
        }
        None => (protected, None),
    };
    let mut file = read_vault().unwrap_or_default();
    file.wrapped_key = b64_encode(&wrapped);
    file.kdf = kdf;
    file.check = b64_encode(&vk.encrypt(b"badeel-check")?);
    write_vault(&file)
}

fn derive(password: &str, kdf: &Kdf) -> Result<Zeroizing<[u8; 32]>> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let salt = b64_decode(&kdf.salt)?;
    let params = Params::new(kdf.mem_kib, kdf.passes, kdf.lanes, Some(32))
        .map_err(|e| anyhow::anyhow!("معطيات اشتقاق غير صالحة: {e}"))?;
    let a2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    a2.hash_password_into(password.as_bytes(), &salt, out.as_mut())
        .map_err(|e| anyhow::anyhow!("فشل اشتقاق المفتاح: {e}"))?;
    Ok(out)
}

fn aes_seal(key: &[u8], plain: &[u8]) -> Result<Vec<u8>> {
    let c = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow::anyhow!("مفتاح غير صالح"))?;
    let mut nonce = [0u8; NONCE_LEN];
    fill_random(&mut nonce);
    let ct = c
        .encrypt(Nonce::from_slice(&nonce), plain)
        .map_err(|_| anyhow::anyhow!("فشل التغليف"))?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ct);
    Ok(out)
}

fn aes_open(key: &[u8], blob: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if blob.len() < NONCE_LEN {
        bail!("بيانات مفتاح تالفة");
    }
    let c = Aes256Gcm::new_from_slice(key).map_err(|_| anyhow::anyhow!("مفتاح غير صالح"))?;
    let plain = c
        .decrypt(Nonce::from_slice(&blob[..NONCE_LEN]), &blob[NONCE_LEN..])
        .map_err(|_| anyhow::anyhow!("كلمة السر غير صحيحة"))?;
    Ok(Zeroizing::new(plain))
}

fn random_b64(n: usize) -> String {
    let mut buf = vec![0u8; n];
    fill_random(&mut buf);
    b64_encode(&buf)
}

#[cfg(target_os = "windows")]
mod dpapi {
    use anyhow::{Result, bail};
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData,
    };
    use zeroize::Zeroizing;

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        unsafe { let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _))); }
        v
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>> {
        let mut input = blob(data);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            if CryptProtectData(&mut input, None, None, None, None, 0, &mut out).is_err() {
                bail!("تعذّر حفظ المفتاح عبر حماية ويندوز");
            }
            Ok(take(out))
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        let mut input = blob(data);
        let mut out = CRYPT_INTEGER_BLOB::default();
        unsafe {
            if CryptUnprotectData(&mut input, None, None, None, None, 0, &mut out).is_err() {
                bail!("هذي الخزنة تخص حساب ويندوز أو جهازًا آخر");
            }
            Ok(Zeroizing::new(take(out)))
        }
    }
}

#[cfg(target_os = "windows")]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>> {
    dpapi::protect(data)
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect(data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    dpapi::unprotect(data)
}

#[cfg(not(target_os = "windows"))]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

#[cfg(not(target_os = "windows"))]
fn dpapi_unprotect(data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    Ok(Zeroizing::new(data.to_vec()))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn b64_decode(s: &str) -> Result<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for c in s.bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' {
            continue;
        }
        let v = B64
            .iter()
            .position(|&x| x == c)
            .context("حرف غير صالح في الترميز")? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrips_every_length() {
        for n in 0..40usize {
            let data: Vec<u8> = (0..n).map(|i| (i * 7 + 1) as u8).collect();
            let enc = b64_encode(&data);
            assert_eq!(b64_decode(&enc).unwrap(), data, "طول {n}");
        }
    }

    #[test]
    fn encrypted_blob_is_not_plaintext_and_rejects_tampering() {
        let key = VaultKey(Zeroizing::new([7u8; 32]));
        let secret = b"session-token-do-not-leak";
        let blob = key.encrypt(secret).unwrap();

        assert!(
            !blob.windows(secret.len()).any(|w| w == secret),
            "النص الصريح ظاهر في الملف المشفّر"
        );
        assert_eq!(key.decrypt(&blob).unwrap().as_slice(), secret);

        let mut bad = blob.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(key.decrypt(&bad).is_err(), "عبث بالملف يجب أن يُرفض");

        let other = VaultKey(Zeroizing::new([9u8; 32]));
        assert!(other.decrypt(&blob).is_err(), "مفتاح آخر يجب ألّا يفكّه");
    }
}
