use serde_json::Value;

pub fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(cur)
}

pub fn set(root: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut cur = root;
    for part in &parts[..parts.len() - 1] {
        if !cur.is_object() {
            *cur = Value::Object(serde_json::Map::new());
        }
        cur = cur
            .as_object_mut()
            .expect("صار كائنًا للتو")
            .entry((*part).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
    }
    if !cur.is_object() {
        *cur = Value::Object(serde_json::Map::new());
    }
    cur.as_object_mut()
        .expect("صار كائنًا للتو")
        .insert(parts[parts.len() - 1].to_string(), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_reads_nested() {
        let v = json!({"Client": {"SavedAccountNames": "a@x.com,b@x.com"}});
        assert_eq!(
            get(&v, "Client.SavedAccountNames").and_then(|v| v.as_str()),
            Some("a@x.com,b@x.com")
        );
        assert!(get(&v, "Client.Missing").is_none());
    }

    #[test]
    fn set_keeps_siblings_and_creates_path() {
        let mut v = json!({"Client": {"Volume": "99", "SavedAccountNames": "a"}});
        set(&mut v, "Client.SavedAccountNames", json!("b,a"));
        set(&mut v, "New.Deep.Key", json!("1"));

        assert_eq!(v["Client"]["SavedAccountNames"], json!("b,a"));
        assert_eq!(v["Client"]["Volume"], json!("99"), "لا تُمسّ بقية الإعدادات");
        assert_eq!(v["New"]["Deep"]["Key"], json!("1"));
    }
}
