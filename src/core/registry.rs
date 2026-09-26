use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum RegValue {
    Str(String),
    Dword(u32),
    Bytes(String),
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};

    fn split(path: &str) -> Result<(RegKey, String)> {
        let (root, rest) = path
            .split_once('\\')
            .ok_or_else(|| anyhow::anyhow!("مسار ريجستري غير صالح: {path}"))?;
        let hive = match root.to_ascii_uppercase().as_str() {
            "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
            "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
            other => bail!("جذر ريجستري غير مدعوم: {other}"),
        };
        Ok((RegKey::predef(hive), rest.to_string()))
    }

    pub fn read(path: &str, name: &str) -> Result<Option<RegValue>> {
        let (hive, sub) = split(path)?;
        let Ok(key) = hive.open_subkey_with_flags(&sub, KEY_READ) else {
            return Ok(None);
        };
        let Ok(value) = key.get_raw_value(name) else {
            return Ok(None);
        };
        Ok(Some(match value.vtype {
            winreg::enums::RegType::REG_DWORD => {
                let n: u32 = key.get_value(name).unwrap_or(0);
                RegValue::Dword(n)
            }
            winreg::enums::RegType::REG_SZ | winreg::enums::RegType::REG_EXPAND_SZ => {
                let s: String = key.get_value(name).unwrap_or_default();
                RegValue::Str(s)
            }
            _ => RegValue::Bytes(super::super::vault::b64_encode(&value.bytes)),
        }))
    }

    pub fn write(path: &str, name: &str, value: &RegValue) -> Result<()> {
        let (hive, sub) = split(path)?;
        let key = match hive.open_subkey_with_flags(&sub, KEY_WRITE) {
            Ok(k) => k,
            Err(_) => hive.create_subkey(&sub)?.0,
        };
        match value {
            RegValue::Str(s) => key.set_value(name, s)?,
            RegValue::Dword(n) => key.set_value(name, n)?,
            RegValue::Bytes(b64) => {
                let bytes = super::super::vault::b64_decode(b64)?;
                let raw = winreg::RegValue {
                    vtype: winreg::enums::RegType::REG_BINARY,
                    bytes: std::borrow::Cow::Owned(bytes),
                };
                key.set_raw_value(name, &raw)?;
            }
        }
        Ok(())
    }

    pub fn read_string(path: &str, name: &str) -> Option<String> {
        match read(path, name).ok()? {
            Some(RegValue::Str(s)) => Some(s),
            _ => None,
        }
    }

    pub fn write_string(path: &str, name: &str, value: &str) -> Result<()> {
        write(path, name, &RegValue::Str(value.to_string()))
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;

    pub fn read(_path: &str, _name: &str) -> Result<Option<RegValue>> {
        Ok(None)
    }
    pub fn write(_path: &str, _name: &str, _value: &RegValue) -> Result<()> {
        Ok(())
    }
    pub fn read_string(_path: &str, _name: &str) -> Option<String> {
        None
    }
    pub fn write_string(_path: &str, _name: &str, _value: &str) -> Result<()> {
        Ok(())
    }
}

pub use imp::{read, read_string, write, write_string};
