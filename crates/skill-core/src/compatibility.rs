//! Conservative display protection; it is not a credential-free certification.
pub fn contains_sensitive_literal(text: &str) -> bool {
    text.split(|c: char| c.is_whitespace() || "\"'`=,:;()[]{}<>".contains(c)).any(|word| {
        ["sk-", "ghp_", "github_pat_", "xoxb-"].iter().any(|prefix| {
            word.strip_prefix(prefix).is_some_and(|tail| tail.len() >= 24 && tail.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'))
        }) || word.starts_with("-----BEGIN")
    }) || text.contains("PRIVATE KEY-----")
}

pub fn safe_text(text: &str) -> String {
    if contains_sensitive_literal(text) { "[REDACTED_SECRET]".into() } else { text.into() }
}
