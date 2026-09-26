#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Value(String),
    Object(Vec<(String, Node)>),
}

impl Node {
    pub fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Object(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Node::Value(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Value(s) => Some(s),
            Node::Object(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Node)] {
        match self {
            Node::Object(e) => e,
            Node::Value(_) => &[],
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if let Node::Object(entries) = self {
            match entries.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
                Some((_, v)) => *v = Node::Value(value.to_string()),
                None => entries.push((key.to_string(), Node::Value(value.to_string()))),
            }
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Node> {
        match self {
            Node::Object(entries) => entries
                .iter_mut()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Node::Value(_) => None,
        }
    }
}

pub fn parse(text: &str) -> Node {
    let mut chars = text.chars().peekable();
    let mut root: Vec<(String, Node)> = Vec::new();
    while let Some(key) = next_token(&mut chars) {
        let value = parse_value(&mut chars);
        root.push((key, value));
    }
    Node::Object(root)
}

fn parse_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> Node {
    skip_ws(chars);
    match chars.peek() {
        Some('{') => {
            chars.next();
            let mut entries = Vec::new();
            loop {
                skip_ws(chars);
                match chars.peek() {
                    Some('}') => {
                        chars.next();
                        break;
                    }
                    None => break,
                    _ => {
                        let Some(key) = next_token(chars) else { break };
                        let value = parse_value(chars);
                        entries.push((key, value));
                    }
                }
            }
            Node::Object(entries)
        }
        _ => Node::Value(next_token(chars).unwrap_or_default()),
    }
}

fn skip_ws(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if *c == '/' {

            chars.next();
            if chars.peek() == Some(&'/') {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
        } else {
            break;
        }
    }
}

fn next_token(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    skip_ws(chars);
    match chars.peek()? {
        '"' => {
            chars.next();
            let mut out = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some(other) => out.push(other),
                        None => break,
                    },
                    other => out.push(other),
                }
            }
            Some(out)
        }
        '{' | '}' => None,
        _ => {
            let mut out = String::new();
            while let Some(c) = chars.peek() {
                if c.is_whitespace() || *c == '{' || *c == '}' {
                    break;
                }
                out.push(*c);
                chars.next();
            }
            (!out.is_empty()).then_some(out)
        }
    }
}

pub fn write(node: &Node) -> String {
    let mut out = String::new();
    if let Node::Object(entries) = node {
        for (k, v) in entries {
            write_entry(&mut out, k, v, 0);
        }
    }
    out
}

fn write_entry(out: &mut String, key: &str, value: &Node, depth: usize) {
    let pad = "\t".repeat(depth);
    match value {
        Node::Value(v) => {
            out.push_str(&format!("{pad}\"{}\"\t\t\"{}\"\n", escape(key), escape(v)));
        }
        Node::Object(entries) => {
            out.push_str(&format!("{pad}\"{}\"\n{pad}{{\n", escape(key)));
            for (k, v) in entries {
                write_entry(out, k, v, depth + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
"users"
{
	"76561198000000001"
	{
		"AccountName"		"ryan"
		"PersonaName"		"Athlawi"
		"RememberPassword"		"1"
		"MostRecent"		"1"
		"Timestamp"		"1700000000"
	}
	"76561198000000002"
	{
		"AccountName"		"alt"
		"MostRecent"		"0"
	}
}
"#;

    #[test]
    fn parses_nested_users() {
        let root = parse(SAMPLE);
        let users = root.get("users").unwrap();
        assert_eq!(users.entries().len(), 2);
        assert_eq!(
            users
                .get("76561198000000001")
                .and_then(|u| u.get("AccountName"))
                .and_then(|v| v.as_str()),
            Some("ryan")
        );
    }

    #[test]
    fn rewrite_keeps_unknown_keys() {
        let mut root = parse(SAMPLE);
        root.get_mut("users")
            .and_then(|u| u.get_mut("76561198000000001"))
            .unwrap()
            .set("MostRecent", "0");

        let text = write(&root);
        assert!(text.contains("Timestamp"), "مفتاح لا نفهمه يجب أن يبقى");
        assert!(text.contains("PersonaName"));

        let again = parse(&text);
        assert_eq!(
            again
                .get("users")
                .and_then(|u| u.get("76561198000000001"))
                .and_then(|u| u.get("MostRecent"))
                .and_then(|v| v.as_str()),
            Some("0")
        );
    }
}
