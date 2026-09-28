use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, RefreshKind, System};

use super::catalog::{Close, Game, Locator, Platform};
use super::{paths, registry};

const GRACEFUL_WAIT: Duration = Duration::from_secs(8);
const FORCE_WAIT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(150);

/// ويندوز يفتح نافذة طرفية سوداء لكل برنامج طرفية يُشغَّل من برنامج
/// رسومي، فنمنعها عن taskkill حين يُغلق المنصّة
#[cfg(windows)]
pub fn hidden(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(windows))]
pub fn hidden(cmd: &mut Command) -> &mut Command {
    cmd
}

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

/// أوّل لعبة تعمل الآن من ألعاب هذه المنصّة
///
/// رايوت مثلًا يُغلق بالقوة وقائمة عملياته تضمّ فالورانت نفسها، فتبديل
/// في وسط مباراة كان يقتل اللعبة ويجلب للاعب عقوبة خروج
pub fn game_running(p: &Platform) -> Option<&'static Game> {
    if p.games.is_empty() {
        return None;
    }
    let names: Vec<&str> = p.games.iter().map(|g| g.exe).collect();
    match_game(p.games, &running(&names))
}

fn match_game<'a>(games: &'a [Game], live: &[String]) -> Option<&'a Game> {
    games
        .iter()
        .find(|g| live.iter().any(|n| n.eq_ignore_ascii_case(g.exe)))
}

fn taskkill(name: &str, force: bool) {
    let mut cmd = Command::new("taskkill.exe");
    if force {
        cmd.arg("/F");
    }
    cmd.args(["/T", "/IM", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hidden(&mut cmd);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::catalog::{PLATFORMS, index_of};

    #[test]
    fn a_running_game_is_caught_whatever_its_letter_case() {
        let riot = &PLATFORMS[index_of("riot")];
        assert!(match_game(riot.games, &[]).is_none());
        assert!(match_game(riot.games, &["discord.exe".into()]).is_none());

        let hit = match_game(riot.games, &["valorant-win64-shipping.EXE".into()])
            .expect("caught despite the casing");
        assert_eq!(hit.ar, "فالورانت");

        // المنصّات التي بلا ألعاب لا تُوقف التبديل
        assert!(PLATFORMS.iter().all(|p| !p.games.is_empty()));
    }

    #[test]
    fn running_sees_a_process_that_always_exists() {
        assert!(!running(&["explorer.exe"]).is_empty(), "explorer دائمًا يعمل");
        assert!(running(&["definitely-not-a-real-process.exe"]).is_empty());
    }
}
