//! Conservative display protection; it is not a credential-free certification.
pub fn contains_sensitive_literal(text: &str) -> bool {
    text.split("://").skip(1).any(|part| {
        part.split(|c:char|c.is_whitespace() || "/\\?#\"'<>".contains(c)).next().is_some_and(|authority|authority.contains('@'))
    }) || serde_json::from_str::<serde_json::Value>(text).is_ok_and(|value| json_sensitive(&value))
        || text
            .split(['\n', '{', '}', ',', ';'])
            .any(sensitive_assignment)
        || text
            .split(|c: char| c.is_whitespace() || "\"'`=,:;()[]{}<>".contains(c))
            .any(|word| {
                ["sk-", "ghp_", "github_pat_", "xoxb-"]
                    .iter()
                    .any(|prefix| {
                        word.strip_prefix(prefix).is_some_and(|tail| {
                            tail.len() >= 24
                                && tail
                                    .bytes()
                                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                        })
                    })
                    || word.starts_with("-----BEGIN")
            })
        || text.contains("PRIVATE KEY-----")
}

fn json_sensitive(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(key, value)| {
            value
                .as_str()
                .is_some_and(|text| sensitive_assignment(&format!("{key}: {text}")))
                || json_sensitive(value)
        }),
        serde_json::Value::Array(values) => values.iter().any(json_sensitive),
        _ => false,
    }
}

fn sensitive_assignment(line: &str) -> bool {
    let line = line.trim().trim_start_matches("export ");
    let Some((key, value)) = line.split_once('=').or_else(|| line.split_once(':')) else {
        return false;
    };
    let key = key
        .trim()
        .trim_matches(['"', '\'', '$'])
        .to_ascii_lowercase();
    if !matches!(
        key.as_str(),
        "password"
            | "passwd"
            | "pwd"
            | "secret"
            | "token"
            | "api_key"
            | "api-key"
            | "access_key"
            | "authorization"
            | "cookie"
            | "private_key"
    ) {
        return false;
    }
    let value = value
        .trim()
        .trim_end_matches(',')
        .trim()
        .trim_matches(['"', '\'']);
    if value.is_empty() {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    if [
        "your_",
        "example",
        "placeholder",
        "redacted",
        "${",
        "process.env",
        "os.environ",
    ]
    .iter()
    .any(|m| lower.contains(m))
        || lower.starts_with('<')
        || lower.starts_with('$')
    {
        return false;
    }
    !value.contains(['(', ')'])
}

pub fn safe_text(text: &str) -> String {
    if contains_sensitive_literal(text) {
        "[REDACTED_SECRET]".into()
    } else {
        text.into()
    }
}
