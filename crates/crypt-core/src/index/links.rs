use crate::RecordId;
use regex::Regex;
use std::sync::LazyLock;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiLink {
    pub target: RecordId,
    pub alias: Option<String>,
}
pub fn parse_wikilinks(body: &str) -> Vec<WikiLink> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\[\[([0-7][0-9A-HJKMNP-TV-Z]{25})(?:\|([^\]\n]*))?\]\]")
            .expect("constant regex")
    });
    RE.captures_iter(body)
        .filter_map(|c| {
            Some(WikiLink {
                target: c[1].parse().ok()?,
                alias: c.get(2).map(|m| m.as_str().to_string()),
            })
        })
        .collect()
}
