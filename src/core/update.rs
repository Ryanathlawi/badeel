use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use anyhow::{Context, Result};
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

/// وصف الإصدار يُكتب بماركداون على GitHub، والنافذة تعرض نصًّا عاديًّا،
/// فكانت العلامات تظهر كما هي. هذه تنزعها وتترك الكلام وحده.
pub fn plain_notes(md: &str) -> String {
    let mut out = String::new();
    for line in md.lines() {
        let t = line.trim();
        if t.starts_with("---") || t.starts_with("```") {
            continue;
        }
        let t = t.trim_start_matches('#').trim_start();
        let t = match t.strip_prefix("- ") {
            Some(rest) => format!("\u{2022} {rest}"),
            None => t.to_owned(),
        };
        let t = t.replace("**", "").replace('`', "");
        if t.is_empty() && out.ends_with("\n\n") {
            continue;
        }
        out.push_str(&t);
        out.push('\n');
    }
    out.trim().to_owned()
}

#[cfg(test)]
mod notes_tests {
    #[test]
    fn markdown_marks_are_stripped() {
        let out = super::plain_notes("### عنوان\n\n- **أمر** مهم\n```\ncode\n```\n");
        assert!(!out.contains('#'), "{out}");
        assert!(!out.contains("**"), "{out}");
        assert!(!out.contains("```"), "{out}");
        assert!(out.contains('\u{2022}'), "{out}");
    }
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

/// ويندوز باورشيل ٥ يرجّع نصًّا لا بايتات حين يكون الرد نصًّا، وتحويل
/// النص إلى base64 يرمي استثناءً فيفشل الطلب كله. RawContentStream
/// يعطي البايتات نفسها في الحالتين، نصًّا كان الرد أو ملفًّا.
/// طلب GET بمكتبة ويندوز نفسها WinHTTP، ويمرّ منه فحص التحديث واللوحة وتنزيل
/// النسخة الجديدة كلها
///
/// كان يمرّ عبر باورشيل مخفي يحمّل ثم يحوّل ما حمّله إلى base64، ثم يستبدل
/// البرنامج نفسه بما نزل، وهذه السلسلة بعينها نمط برامج التنزيل الخبيثة، فكانت
/// حماية ويندوز تحذف بديل فجأة عند من سحابتها مفعّلة، وWinHTTP هو ما يستعمله
/// أي برنامج عادي، ويحترم بروكسي الجهاز ويتبع تحويلات GitHub وحده
#[cfg(target_os = "windows")]
pub fn http_get(url: &str) -> Result<Vec<u8>> {
    use std::ffi::c_void;
    use windows::Win32::Networking::WinHttp::*;
    use windows::core::{HSTRING, PCWSTR, w};

    /// المقبض يُغلق متى خرج من مداه، ولو خرجنا بخطأ في منتصف الطريق
    struct Handle(*mut c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
    fn held(h: *mut c_void) -> Result<Handle> {
        if h.is_null() {
            Err(windows::core::Error::from_thread()).context("تعذّر الاتصال بالخادم")
        } else {
            Ok(Handle(h))
        }
    }

    let rest = url.strip_prefix("https://").context("رابط غير مدعوم")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };

    unsafe {
        let session = held(WinHttpOpen(
            w!("badeel"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        WinHttpSetTimeouts(session.0, 15_000, 15_000, 30_000, 60_000)?;
        let connect = held(WinHttpConnect(
            session.0,
            &HSTRING::from(host),
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ))?;
        let request = held(WinHttpOpenRequest(
            connect.0,
            w!("GET"),
            &HSTRING::from(path),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        WinHttpSendRequest(request.0, None, None, 0, 0, 0).context("فشل الاتصال بالخادم")?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).context("فشل الاتصال بالخادم")?;

        let mut status = 0u32;
        let mut len = std::mem::size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut status as *mut u32 as *mut c_void),
            &mut len,
            std::ptr::null_mut(),
        )?;
        anyhow::ensure!(status == 200, "رد الخادم {status}");

        let mut out = Vec::new();
        loop {
            let mut avail = 0u32;
            WinHttpQueryDataAvailable(request.0, &mut avail)?;
            if avail == 0 {
                break;
            }
            let start = out.len();
            out.resize(start + avail as usize, 0);
            let mut read = 0u32;
            WinHttpReadData(
                request.0,
                out[start..].as_mut_ptr() as *mut c_void,
                avail,
                &mut read,
            )?;
            out.truncate(start + read as usize);
            if read == 0 {
                break;
            }
        }
        Ok(out)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn http_get(_url: &str) -> Result<Vec<u8>> {
    anyhow::bail!("الشبكة غير مدعومة على هذا النظام")
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
