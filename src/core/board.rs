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

/// تتغيّر البطاقة المعروضة كل هذه المدّة، فلا تجمد واحدة أمام العين
const ROTATE: u64 = 300;

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

/// بطاقة تناسب هذه المنصّة، وبطاقات الكل لمن لا بطاقة خاصّة له
///
/// المنصّة الفارغة تعني الشاشة الأولى، فتأخذ بطاقات الكل وحدها
pub fn pick<'a>(board: &'a Board, platform: &str, now: u64) -> Option<&'a Card> {
    let general = |c: &&Card| c.platforms.is_empty();
    let mut pool: Vec<&Card> = if platform.is_empty() {
        board.cards.iter().filter(general).collect()
    } else {
        board
            .cards
            .iter()
            .filter(|c| c.platforms.iter().any(|p| p == platform))
            .collect()
    };
    if pool.is_empty() {
        pool = board.cards.iter().filter(general).collect();
    }
    let usable: Vec<&Card> = pool
        .into_iter()
        .filter(|c| !c.title.ar.trim().is_empty() || !c.title.en.trim().is_empty())
        .collect();
    if usable.is_empty() {
        return None;
    }
    Some(usable[(now / ROTATE) as usize % usable.len()])
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
    fn a_platform_gets_its_own_card_and_others_fall_back_to_the_general_ones() {
        let b = load_test_board();
        assert_eq!(pick(&b, "battlenet", 0).map(|c| c.id.as_str()), Some("bnet"));
        // منصّة لا بطاقة لها تأخذ من العامّ
        assert_eq!(pick(&b, "riot", 0).map(|c| c.id.as_str()), Some("all"));
        // الشاشة الأولى تأخذ العامّ ولو كانت هناك بطاقات منصّات
        assert_eq!(pick(&b, "", 0).map(|c| c.id.as_str()), Some("all"));
        assert!(pick(&Board::default(), "steam", 0).is_none());
    }

    #[test]
    fn the_shown_card_rotates_with_time() {
        let mut b = load_test_board();
        b.cards.push(Card {
            id: "all2".into(),
            title: Say { ar: "ب".into(), en: "b".into() },
            ..Default::default()
        });
        assert_eq!(pick(&b, "", 0).map(|c| c.id.as_str()), Some("all"));
        assert_eq!(pick(&b, "", ROTATE).map(|c| c.id.as_str()), Some("all2"));
        assert_eq!(pick(&b, "", ROTATE * 2).map(|c| c.id.as_str()), Some("all"));
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
