use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::paths;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub accent: u8,
    #[serde(default)]
    pub created: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Book {
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub active: String,
}

impl Book {
    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn current(&self) -> &Profile {
        self.get(&self.active)
            .or_else(|| self.profiles.first())
            .expect("book always holds one profile")
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.profiles.iter().position(|p| p.id == id)
    }
}

fn file() -> PathBuf {
    paths::data_root().join("profiles.json")
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load() -> Book {
    let mut book: Book = fs::read(file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let mut born = false;
    if book.profiles.is_empty() {
        book.profiles.push(Profile {
            id: "main".into(),
            name: super::clock::machine_name(),
            accent: 0,
            created: now(),
        });
        born = true;
    }
    if let Some(first) = book.profiles.first_mut() {
        if first.name.trim().is_empty() {
            first.name = super::clock::machine_name();
            born = true;
        }
    }
    if book.get(&book.active.clone()).is_none() {
        book.active = book.profiles[0].id.clone();
    }
    paths::set_profile(&book.active);
    if born {
        let _ = save(&book);
    }
    book
}

pub fn save(book: &Book) -> Result<()> {
    let path = file();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = paths::temp_sibling(&path, "profiles");
    fs::write(&tmp, serde_json::to_vec_pretty(book)?)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn fresh_id(book: &Book) -> String {
    let mut n = book.profiles.len() + 1;
    loop {
        let id = format!("p{n}");
        if book.get(&id).is_none() {
            return id;
        }
        n += 1;
    }
}

pub fn create(book: &mut Book, name: &str, accent: u8) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("empty name");
    }
    if book.profiles.len() >= 12 {
        bail!("too many profiles");
    }
    let id = fresh_id(book);
    book.profiles.push(Profile {
        id: id.clone(),
        name: name.to_string(),
        accent,
        created: now(),
    });
    save(book)?;
    Ok(id)
}

pub fn remove(book: &mut Book, id: &str) -> Result<()> {
    if book.profiles.len() <= 1 {
        bail!("last profile");
    }
    let Some(i) = book.index_of(id) else {
        bail!("unknown profile");
    };
    let root = paths::root_of(id);
    book.profiles.remove(i);
    if book.active == id {
        book.active = book.profiles[0].id.clone();
        paths::set_profile(&book.active);
    }
    save(book)?;
    if root != paths::data_root() && root.is_dir() {
        let _ = fs::remove_dir_all(root);
    }
    Ok(())
}

pub fn activate(book: &mut Book, id: &str) -> Result<()> {
    if book.get(id).is_none() {
        bail!("unknown profile");
    }
    book.active = id.to_string();
    paths::set_profile(id);
    save(book)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_profile_keeps_the_legacy_root() {
        assert_eq!(paths::root_of("main"), paths::data_root());
        assert_eq!(paths::root_of(""), paths::data_root());
    }

    #[test]
    fn extra_profiles_get_their_own_root() {
        let r = paths::root_of("p2");
        assert_ne!(r, paths::data_root());
        assert!(r.ends_with("p2"));
        assert!(r.starts_with(paths::data_root()));
    }

    #[test]
    fn a_book_always_has_a_current_profile() {
        let book = Book {
            profiles: vec![Profile {
                id: "x".into(),
                name: "n".into(),
                accent: 3,
                created: 0,
            }],
            active: "missing".into(),
        };
        assert_eq!(book.current().id, "x");
    }
}
