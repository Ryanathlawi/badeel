use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const REPO: &str = "Ryanathlawi/badeel";

#[derive(Clone, Debug, PartialEq)]
pub struct Available {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Progress {
    Checking,
    UpToDate,
    Found(Available),
    Downloading { done: u64, total: u64 },
    Installing,

    Ready,
    Failed(String),
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u32> {
        v.trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map_while(|p| p.parse::<u32>().ok())
            .collect()
    }
    let (a, b) = (parts(candidate), parts(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

fn http_get(url: &str) -> Result<Vec<u8>> {

    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "$ProgressPreference='SilentlyContinue'; \
                 [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12; \
                 $r=Invoke-WebRequest -UseBasicParsing -Headers @{{'User-Agent'='badeel'}} -Uri '{url}'; \
                 [Convert]::ToBase64String($r.Content)"
            ),
        ])
        .output()
        .context("تعذّر تنفيذ طلب الشبكة")?;
    if !out.status.success() {
        bail!("فشل الاتصال بالخادم");
    }
    let b64: String = String::from_utf8_lossy(&out.stdout)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    super::vault::b64_decode(&b64)
}

pub fn check() -> Result<Option<Available>> {
    let raw = http_get(&format!(
        "https://api.github.com/repos/{REPO}/releases/latest"
    ))?;
    let release: Release = serde_json::from_slice(&raw).context("رد غير متوقع من GitHub")?;
    if release.prerelease || !is_newer(&release.tag_name, current_version()) {
        return Ok(None);
    }
    let asset = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".exe"))
        .context("الإصدار الجديد بلا ملف تشغيل")?;
    Ok(Some(Available {
        version: release.tag_name.trim_start_matches('v').to_string(),
        notes: release.body.clone(),
        url: asset.browser_download_url.clone(),
        size: asset.size,
    }))
}

fn exe_path() -> Result<PathBuf> {
    std::env::current_exe().context("لم أعرف مكان البرنامج نفسه")
}

pub fn cleanup_previous() {
    if let Ok(exe) = exe_path() {
        let old = exe.with_extension("old");
        if old.exists() {

            let _ = std::fs::remove_file(&old);
        }
    }
}

pub fn install(update: &Available, report: &Sender<Progress>) -> Result<()> {
    let _ = report.send(Progress::Downloading {
        done: 0,
        total: update.size,
    });
    let bytes = http_get(&update.url)?;
    anyhow::ensure!(bytes.len() > 1024, "الملف المنزَّل غير صالح");
    let _ = report.send(Progress::Downloading {
        done: bytes.len() as u64,
        total: update.size.max(bytes.len() as u64),
    });

    let _ = report.send(Progress::Installing);
    let exe = exe_path()?;
    let new = exe.with_extension("new");
    std::fs::write(&new, &bytes)?;

    let old = exe.with_extension("old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).context("تعذّر تنحية النسخة الحالية")?;
    match std::fs::rename(&new, &exe) {
        Ok(()) => Ok(()),
        Err(e) => {

            let _ = std::fs::rename(&old, &exe);
            Err(e).context("تعذّر تركيب النسخة الجديدة")
        }
    }
}

pub fn restart() -> Result<()> {
    let exe = exe_path()?;
    std::process::Command::new(exe).spawn()?;
    Ok(())
}

pub fn spawn_check() -> Receiver<Progress> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(Progress::Checking);
        let msg = match check() {
            Ok(Some(u)) => Progress::Found(u),
            Ok(None) => Progress::UpToDate,
            Err(e) => Progress::Failed(e.to_string()),
        };
        let _ = tx.send(msg);
    });
    rx
}

pub fn spawn_install(update: Available) -> Receiver<Progress> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let msg = match install(&update, &tx) {
            Ok(()) => Progress::Ready,
            Err(e) => Progress::Failed(e.to_string()),
        };
        let _ = tx.send(msg);
    });
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(is_newer("v1.2.0", "1.1.9"));
        assert!(is_newer("2.0.0", "1.99.99"));
        assert!(is_newer("1.0.1", "1.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.9", "1.0.0"));
        assert!(!is_newer("1.0.0-beta", "1.0.0"));
    }
}
