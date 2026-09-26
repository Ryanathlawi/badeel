use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::paths;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {

    pub id: String,

    pub name: String,

    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub avatar: Option<String>,

    #[serde(default)]
    pub last_used: u64,
    #[serde(default)]
    pub uses: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Index {
    #[serde(default)]
    pub accounts: Vec<Account>,

    #[serde(default)]
    pub order: Vec<String>,
}

impl Index {
    pub fn get(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }

    pub fn upsert(&mut self, acc: Account) {
        match self.accounts.iter_mut().find(|a| a.id == acc.id) {
            Some(existing) => *existing = acc,
            None => self.accounts.push(acc),
        }
    }

    pub fn remove(&mut self, id: &str) -> Option<Account> {
        let pos = self.accounts.iter().position(|a| a.id == id)?;
        self.order.retain(|o| o != id);
        Some(self.accounts.remove(pos))
    }

    pub fn sorted(&self) -> Vec<&Account> {
        let rank: BTreeMap<&str, usize> = self
            .order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        let mut v: Vec<&Account> = self.accounts.iter().collect();
        v.sort_by_key(|a| {
            (
                rank.get(a.id.as_str()).copied().unwrap_or(usize::MAX),
                std::cmp::Reverse(a.last_used),
            )
        });
        v
    }
}

fn index_path(platform: &str) -> PathBuf {
    paths::accounts_root(platform).join("index.json")
}

pub fn load(platform: &str) -> Index {
    let path = index_path(platform);
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            log::warn!("سجل {platform} غير صالح ({e}) — أبدأ بسجل فارغ");
            Index::default()
        }),
        Err(_) => Index::default(),
    }
}

pub fn save(platform: &str, index: &Index) -> anyhow::Result<()> {
    let path = index_path(platform);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = paths::temp_sibling(&path, "idx");
    fs::write(&tmp, serde_json::to_vec_pretty(index)?)?;
    super::fsops::swap_in(&tmp, &path)?;
    Ok(())
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(id: &str, last: u64) -> Account {
        Account {
            id: id.into(),
            name: id.into(),
            note: String::new(),
            avatar: None,
            last_used: last,
            uses: 0,
        }
    }

    #[test]
    fn sorted_prefers_manual_order_then_recency() {
        let mut idx = Index::default();
        idx.upsert(acc("a", 10));
        idx.upsert(acc("b", 30));
        idx.upsert(acc("c", 20));
        assert_eq!(
            idx.sorted().iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            vec!["b", "c", "a"]
        );

        idx.order = vec!["a".into()];
        assert_eq!(idx.sorted()[0].id, "a");
    }

    #[test]
    fn remove_drops_order_entry() {
        let mut idx = Index::default();
        idx.upsert(acc("a", 1));
        idx.order = vec!["a".into()];
        idx.remove("a");
        assert!(idx.accounts.is_empty() && idx.order.is_empty());
    }
}
