//! باتل نت يحفظ كل الحسابات التي يتذكّرها في قائمة واحدة مفصولة بفواصل،
//! وأوّل اسم فيها هو الحساب الذي يفتح به، فيقرأها بديل كما يقرأ قائمة ستيم
//! ويبدّل بأن يقدّم المطلوب إلى أوّلها ويترك البقية كما هي

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::Value;

use super::{fsops, jsonpath, paths};

const CONFIG: &str = r"%APPDATA%\Battle.net\Battle.net.config";
const NAMES: &str = "Client.SavedAccountNames";
const REMEMBER: &str = "Client.RememberAccountName";

fn config() -> PathBuf {
    paths::expand(CONFIG)
}

/// ملف حفظه أحد بمفكّرة ويندوز القديمة يبدأ بعلامة BOM، وJSON الصارم يرفضها
/// فكانت القائمة ترجع فارغة بلا كلمة
fn parse(raw: &[u8]) -> serde_json::Result<Value> {
    serde_json::from_slice(raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(raw))
}

fn read() -> Option<Value> {
    parse(&fs::read(config()).ok()?).ok()
}

fn write(json: &Value) -> Result<()> {
    let path = config();
    let backup = path.with_extension("config.badeel-bak");
    let _ = fs::copy(&path, &backup);
    let staged = paths::temp_sibling(&path, "bnet");
    fs::write(&staged, serde_json::to_vec_pretty(json)?)?;
    fsops::swap_in(&staged, &path)?;
    Ok(())
}

/// لماذا لم يظهر حساب من باتل نت، فيقوله بديل بدل أن يسكت والمستخدم لا يدري
#[derive(Clone, Debug, PartialEq)]
pub enum Empty {
    /// لم يُفتح باتل نت على هذا المستخدم في ويندوز بعد
    NoConfig,
    /// الملف موجود لكنه لم يُقرأ، ومعه السبب
    Unreadable(String),
    /// باتل نت مضبوط على ألّا يتذكّر أسماء الحسابات، فلا قائمة أصلًا
    NotRemembered,
    /// يتذكّر لكنه لم يحفظ اسمًا بعد
    NothingSaved,
}

fn classify(raw: Option<Vec<u8>>) -> Empty {
    let Some(raw) = raw else {
        return Empty::NoConfig;
    };
    let json = match parse(&raw) {
        Ok(j) => j,
        Err(e) => return Empty::Unreadable(e.to_string()),
    };
    let off = jsonpath::get(&json, REMEMBER)
        .and_then(|v| v.as_str())
        .is_some_and(|r| r.trim().eq_ignore_ascii_case("false"));
    if off {
        Empty::NotRemembered
    } else {
        Empty::NothingSaved
    }
}

pub fn why_empty() -> Empty {
    classify(fs::read(config()).ok())
}

/// يجعل باتل نت يتذكّر أسماء الحسابات من الدخول القادم، وباتل نت مغلق قبلها
/// وإلا كتب قيمته القديمة فوقها عند خروجه
pub fn remember_names() -> Result<()> {
    let mut json = read().context("تعذّرت قراءة إعدادات باتل نت")?;
    jsonpath::set(&mut json, REMEMBER, Value::String("true".into()));
    write(&json)
}

fn split(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// يقدّم الاسم المطلوب إلى أوّل القائمة، ويرجع لا شيء إن لم يكن فيها
fn reorder(list: &str, pick: &str) -> Option<String> {
    let mut names = split(list);
    let i = names.iter().position(|n| n.eq_ignore_ascii_case(pick))?;
    let head = names.remove(i);
    names.insert(0, head);
    Some(names.join(","))
}

/// الأسماء المحفوظة بترتيب باتل نت نفسه
pub fn accounts() -> Vec<String> {
    read()
        .as_ref()
        .and_then(|json| jsonpath::get(json, NAMES))
        .and_then(|v| v.as_str())
        .map(split)
        .unwrap_or_default()
}

pub fn current_account() -> Option<String> {
    accounts().into_iter().next()
}

pub fn select_account(name: &str) -> Result<()> {
    let mut json = read().context("تعذّرت قراءة إعدادات باتل نت")?;
    let list = jsonpath::get(&json, NAMES)
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let next = reorder(list, name).context("باتل نت لا يتذكّر هذا الحساب على هذا الجهاز")?;

    jsonpath::set(&mut json, NAMES, Value::String(next));
    // لو أُطفئ هذا المفتاح نسي باتل نت القائمة كلها ولم يبق للتبديل ما يعمل عليه
    jsonpath::set(&mut json, REMEMBER, Value::String("true".into()));
    write(&json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picked_name_moves_to_the_front_and_nobody_is_lost() {
        let list = "a@x.com,b@x.com,c@x.com";
        assert_eq!(
            reorder(list, "b@x.com").as_deref(),
            Some("b@x.com,a@x.com,c@x.com")
        );
        assert_eq!(reorder(list, "a@x.com").as_deref(), Some(list));
        assert_eq!(reorder(list, "B@X.COM").as_deref(), Some("b@x.com,a@x.com,c@x.com"));
        assert!(reorder(list, "d@x.com").is_none());
    }

    #[test]
    fn a_file_saved_with_a_bom_still_reads() {
        let raw = b"\xEF\xBB\xBF{\"Client\":{\"SavedAccountNames\":\"a@x.com\"}}";
        let json = parse(raw).expect("BOM is skipped");
        assert_eq!(json["Client"]["SavedAccountNames"], "a@x.com");
    }

    #[test]
    fn an_empty_list_says_why() {
        assert_eq!(classify(None), Empty::NoConfig);
        assert!(matches!(classify(Some(b"{ broken".to_vec())), Empty::Unreadable(_)));
        assert_eq!(
            classify(Some(b"\xEF\xBB\xBF{\"Client\":{\"RememberAccountName\":\"false\"}}".to_vec())),
            Empty::NotRemembered
        );
        assert_eq!(
            classify(Some(b"{\"Client\":{\"RememberAccountName\":\"true\"}}".to_vec())),
            Empty::NothingSaved
        );
        assert_eq!(classify(Some(b"{\"Client\":{}}".to_vec())), Empty::NothingSaved);
    }

    #[test]
    fn spacing_and_empty_entries_are_ignored() {
        assert_eq!(split(" a@x.com , ,b@x.com "), vec!["a@x.com", "b@x.com"]);
        assert!(split("").is_empty());
    }
}
