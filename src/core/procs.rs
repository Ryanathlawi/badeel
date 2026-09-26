use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, RefreshKind, System};

use super::catalog::{Close, Locator, Platform};
use super::{paths, registry};

const GRACEFUL_WAIT: Duration = Duration::from_secs(8);
const FORCE_WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(150);

fn system() -> System {
    System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()))
}

pub fn running(names: &[&str]) -> Vec<String> {
    let mut sys = system();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut found: Vec<String> = Vec::new();
    for p in sys.processes().values() {
        let n = p.name().to_string_lossy().to_string();
        if names.iter().any(|w| w.eq_ignore_ascii_case(&n)) && !found.contains(&n) {
            found.push(n);
        }
    }
    found
}

fn taskkill(name: &str, force: bool) {
    let mut cmd = Command::new("taskkill.exe");
    if force {
        cmd.arg("/F");
    }
    cmd.args(["/T", "/IM", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = cmd.status();
}

fn wait_gone(names: &[&str], limit: Duration) -> bool {
    let start = Instant::now();
    loop {
        if running(names).is_empty() {
            return true;
        }
        if start.elapsed() >= limit {
            return false;
        }
        std::thread::sleep(POLL);
    }
}

pub fn close_all(names: &[&str], mode: Close) -> anyhow::Result<()> {
    if running(names).is_empty() {
        return Ok(());
    }
    if mode == Close::Graceful {
        for n in names {
            taskkill(n, false);
        }
        if wait_gone(names, GRACEFUL_WAIT) {
            std::thread::sleep(Duration::from_millis(250));
            return Ok(());
        }
    }
    for n in names {
        taskkill(n, true);
    }
    if !wait_gone(names, FORCE_WAIT) {
        let left = running(names).join("، ");
        anyhow::bail!("ما زال يعمل بعد محاولة الإغلاق: {left}");
    }
    std::thread::sleep(Duration::from_millis(250));
    Ok(())
}

fn clean_exe_value(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .split(',')
        .next()
        .unwrap_or("")
        .replace('/', "\\")
        .trim()
        .to_string()
}

pub fn resolve_exe(p: &Platform) -> Option<PathBuf> {
    for loc in p.locate {
        let candidate = match loc {
            Locator::Path(raw) => Some(paths::expand(raw)),
            Locator::RegExe { key, value } => registry::read_string(key, value)
                .map(|v| PathBuf::from(clean_exe_value(&v))),
            Locator::RegDir { key, value, exe } => registry::read_string(key, value)
                .map(|v| PathBuf::from(clean_exe_value(&v)).join(exe)),
            Locator::JsonExe { file, key } => std::fs::read_to_string(paths::expand(file))
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|json| super::jsonpath::get(&json, key).and_then(|v| v.as_str()).map(String::from))
                .map(|v| PathBuf::from(clean_exe_value(&v))),
        };
        if let Some(path) = candidate {
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

pub fn installed(p: &Platform) -> bool {
    resolve_exe(p).is_some()
}

pub fn launch(p: &Platform, extra: &[String]) -> anyhow::Result<()> {
    let path = resolve_exe(p).ok_or_else(|| {
        anyhow::anyhow!("لم أجد ملف تشغيل {} على هذا الجهاز", p.name_ar)
    })?;
    let mut cmd = Command::new(&path);
    cmd.args(p.launch_args);
    cmd.args(extra);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    cmd.spawn()?;
    Ok(())
}
