use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub fn expand(input: &str) -> PathBuf {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
                break;
            }
        }
    }
    out.push_str(rest);
    PathBuf::from(out)
}

pub fn data_root() -> PathBuf {
    if let Some(portable) = portable_root() {
        return portable;
    }
    expand("%APPDATA%").join("badeel")
}

fn portable_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("UserData");
    candidate.is_dir().then_some(candidate)
}

static ACTIVE: RwLock<String> = RwLock::new(String::new());

pub fn set_profile(id: &str) {
    if let Ok(mut w) = ACTIVE.write() {
        *w = id.to_string();
    }
}

pub fn active_profile() -> String {
    ACTIVE.read().map(|r| r.clone()).unwrap_or_default()
}

pub fn root_of(id: &str) -> PathBuf {
    if id.is_empty() || id == "main" {
        data_root()
    } else {
        data_root().join("profiles").join(sanitize(id))
    }
}

pub fn profile_root() -> PathBuf {
    root_of(&active_profile())
}

pub fn accounts_root(platform: &str) -> PathBuf {
    profile_root().join("accounts").join(platform)
}

pub fn account_dir(platform: &str, account_id: &str) -> PathBuf {
    accounts_root(platform).join(sanitize(account_id))
}

pub fn work_dir(platform: &str) -> PathBuf {
    profile_root().join(".work").join(platform)
}

pub fn sanitize(name: &str) -> String {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    s = s.trim().trim_end_matches('.').to_string();
    if s.is_empty() {
        s = "account".to_string();
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&s)) {
        s.push('_');
    }
    s.chars().take(120).collect()
}

pub fn temp_sibling(target: &Path, tag: &str) -> PathBuf {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "badeel".to_string());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    target.with_file_name(format!("{name}.badeel-{tag}-{stamp}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_keeps_unknown_tokens() {
        let p = expand("%NOT_A_REAL_VAR_12345%\\x");
        assert!(p.to_string_lossy().starts_with("%NOT_A_REAL_VAR_12345%"));
    }

    #[test]
    fn sanitize_handles_reserved_and_separators() {
        assert_eq!(sanitize("a/b\\c"), "a_b_c");
        assert_eq!(sanitize("con"), "con_");
        assert_eq!(sanitize("  "), "account");
        assert_eq!(sanitize("trailing."), "trailing");
    }
}
