use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::paths;

fn is_locked(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(32) | Some(33))
}

const LOCK_TIMEOUT: Duration = Duration::from_secs(12);

fn retry_locked<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let start = Instant::now();
    let mut wait = Duration::from_millis(30);
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) if is_locked(&e) && start.elapsed() < LOCK_TIMEOUT => {
                std::thread::sleep(wait);
                wait = (wait * 2).min(Duration::from_millis(400));
            }
            Err(e) => return Err(e),
        }
    }
}

pub fn copy_file(src: &Path, dst: &Path) -> io::Result<u64> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let n = retry_locked(|| fs::copy(src, dst))?;

    let want = fs::metadata(src)?.len();
    let got = fs::metadata(dst)?.len();
    if want != got {
        return Err(io::Error::other(format!(
            "نسخ غير مكتمل: {} ({got} من {want} بايت)",
            src.display()
        )));
    }
    Ok(n)
}

pub fn copy_tree(src: &Path, dst: &Path) -> io::Result<()> {
    let mut failures: Vec<String> = Vec::new();
    copy_tree_inner(src, dst, &mut failures)?;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "تعذّر نسخ {} ملف: {}",
            failures.len(),
            failures.join("، ")
        )))
    }
}

fn copy_tree_inner(src: &Path, dst: &Path, failures: &mut Vec<String>) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in retry_locked(|| fs::read_dir(src))? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_tree_inner(&from, &to, failures)?;
        } else if let Err(e) = copy_file(&from, &to) {
            failures.push(format!("{} ({e})", from.display()));
        }
    }
    Ok(())
}

pub fn copy_any(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        copy_tree(src, dst)
    } else {
        copy_file(src, dst).map(|_| ())
    }
}

pub fn remove_any(path: &Path) -> io::Result<()> {
    let r = if path.is_dir() {
        retry_locked(|| fs::remove_dir_all(path))
    } else {
        retry_locked(|| fs::remove_file(path))
    };
    match r {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub fn swap_in(staged: &Path, target: &Path) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = target.exists();
    let aside = paths::temp_sibling(target, "old");
    if existing {
        retry_locked(|| fs::rename(target, &aside))?;
    }
    match retry_locked(|| fs::rename(staged, target)) {
        Ok(()) => {
            if existing {
                let _ = remove_any(&aside);
            }
            Ok(())
        }
        Err(e) => {
            if existing {
                let _ = fs::rename(&aside, target);
            }
            Err(e)
        }
    }
}

pub struct Backup {
    pub target: PathBuf,
    pub saved: Option<PathBuf>,
}

impl Backup {
    pub fn capture(target: &Path, into: &Path, tag: &str) -> io::Result<Self> {
        if !target.exists() {
            return Ok(Self {
                target: target.to_path_buf(),
                saved: None,
            });
        }
        let saved = into.join(tag);
        copy_any(target, &saved)?;
        Ok(Self {
            target: target.to_path_buf(),
            saved: Some(saved),
        })
    }

    pub fn restore(&self) -> io::Result<()> {
        match &self.saved {
            Some(saved) => {
                let staged = paths::temp_sibling(&self.target, "rb");
                copy_any(saved, &staged)?;
                swap_in(&staged, &self.target)
            }
            None => remove_any(&self.target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("badeel-test-{name}"));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn swap_in_replaces_and_keeps_content() {
        let d = tmp("swap");
        let target = d.join("live.txt");
        fs::write(&target, b"old").unwrap();
        let staged = d.join("staged.txt");
        fs::write(&staged, b"new").unwrap();

        swap_in(&staged, &target).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(!staged.exists());
    }

    #[test]
    fn backup_restores_absence() {
        let d = tmp("absence");
        let store = d.join("store");
        fs::create_dir_all(&store).unwrap();
        let target = d.join("missing.txt");

        let b = Backup::capture(&target, &store, "x").unwrap();
        fs::write(&target, b"appeared").unwrap();
        b.restore().unwrap();

        assert!(!target.exists(), "ملف لم يكن موجودًا يجب ألّا يبقى");
    }

    #[test]
    fn backup_restores_tree() {
        let d = tmp("tree");
        let store = d.join("store");
        fs::create_dir_all(&store).unwrap();
        let target = d.join("data");
        fs::create_dir_all(target.join("sub")).unwrap();
        fs::write(target.join("sub/a.txt"), b"one").unwrap();

        let b = Backup::capture(&target, &store, "data").unwrap();
        fs::write(target.join("sub/a.txt"), b"two").unwrap();
        fs::write(target.join("sub/b.txt"), b"extra").unwrap();
        b.restore().unwrap();

        assert_eq!(fs::read(target.join("sub/a.txt")).unwrap(), b"one");
        assert!(!target.join("sub/b.txt").exists());
    }
}
