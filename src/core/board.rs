//! لوحة يكتب فيها صاحب البرنامج ما يريد أن يقوله لمستخدميه: كلمة تشدّ
//! العزم أو إشارة إلى عمل آخر له
//!
//! تُقرأ من ملف على موقعه فيغيّرها بلا أن يُصدر نسخة جديدة، ومعها نسخة
//! مدمجة في البرنامج تظهر قبل أن يصل الملف وحين لا تكون هناك شبكة
//! ولا شيء يُرسل من الجهاز، إنما يُطلب الملف كما يُطلب التحديث

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::paths;

const URL: &str = "https://ryanathlawi.github.io/badeel-site/board.json";
const BUNDLED: &str = include_str!("../../assets/board.json");

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Say {
    #[serde(default)]
    pub ar: String,
    #[serde(default)]
    pub en: String,
}

impl Say {
    pub fn t(&self, rtl: bool) -> &str {
        if rtl { &self.ar } else { &self.en }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Card {
    #[serde(default)]
    pub id: String,
    /// المنصّات التي تظهر فيها هذه البطاقة، وفراغها يعني كل مكان
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub badge: Say,
    #[serde(default)]
    pub title: Say,
    #[serde(default)]
    pub body: Say,
    #[serde(default)]
    pub cta: Say,
    #[serde(default)]
    pub url: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Board {
    #[serde(default)]
    pub cards: Vec<Card>,
}

fn cache() -> PathBuf {
    paths::data_root().join("board.json")
}

/// ما بين أيدينا الآن: آخر ما وصل، وإلا المدمج
pub fn load() -> Board {
    fs::read(cache())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .or_else(|| serde_json::from_str(BUNDLED).ok())
        .unwrap_or_default()
}

/// يجلب اللوحة ويخزّنها لما بعد، ويُنادى في خيط مستقل فلا يُبطئ الفتح
pub fn refresh() -> Result<Board> {
    let raw = super::update::http_get(URL)?;
    let board: Board = serde_json::from_slice(&raw).context("لوحة غير مفهومة")?;
    let path = cache();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, &raw)?;
    Ok(board)
}

/// بطاقات هذه المنصّة أوّلًا ثم العامّة، فمن فتح باتل نت رأى بطاقته قبل
/// غيرها ثم دارت عليه البقية، والمنصّة الفارغة تعني الشاشة الأولى فلها العامّة
pub fn pool<'a>(board: &'a Board, platform: &str) -> Vec<&'a Card> {
    let titled = |c: &&Card| !c.title.ar.trim().is_empty() || !c.title.en.trim().is_empty();
    let general = board.cards.iter().filter(|c| c.platforms.is_empty());
    if platform.is_empty() {
        return general.filter(titled).collect();
    }
    board
        .cards
        .iter()
        .filter(|c| c.platforms.iter().any(|p| p == platform))
        .chain(general)
        .filter(titled)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_board_parses_and_carries_cards() {
        let b: Board = serde_json::from_str(BUNDLED).expect("اللوحة المدمجة سليمة");
        assert!(b.cards.len() >= 2);
        assert!(b.cards.iter().all(|c| !c.id.trim().is_empty()));
        assert!(
            b.cards
                .iter()
                .all(|c| !c.title.ar.trim().is_empty() && !c.title.en.trim().is_empty())
        );
        // من كان له رابط فله زرّ، ومن لا رابط له فلا زرّ
        assert!(
            b.cards
                .iter()
                .all(|c| c.url.is_empty() == c.cta.ar.trim().is_empty())
        );
    }

    #[test]
    fn a_platform_sees_its_own_card_first_then_the_general_ones() {
        let b = load_test_board();
        let ids = |p: &str| pool(&b, p).iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("battlenet"), vec!["bnet", "all"]);
        // منصّة لا بطاقة لها تأخذ العامّ وحده
        assert_eq!(ids("riot"), vec!["all"]);
        // الشاشة الأولى لا ترى بطاقات المنصّات
        assert_eq!(ids(""), vec!["all"]);
        assert!(pool(&Board::default(), "steam").is_empty());
    }

    #[test]
    fn a_card_without_a_title_never_shows() {
        let mut b = load_test_board();
        b.cards.push(Card {
            id: "blank".into(),
            ..Default::default()
        });
        assert!(pool(&b, "").iter().all(|c| c.id != "blank"));
    }

    fn load_test_board() -> Board {
        Board {
            cards: vec![
                Card {
                    id: "bnet".into(),
                    platforms: vec!["battlenet".into()],
                    title: Say { ar: "أ".into(), en: "a".into() },
                    ..Default::default()
                },
                Card {
                    id: "all".into(),
                    title: Say { ar: "ج".into(), en: "c".into() },
                    ..Default::default()
                },
            ],
        }
    }
}
