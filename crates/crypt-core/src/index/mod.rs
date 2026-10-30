pub mod links;
use crate::{FieldValue, RecordId, model::record::RecordDoc};
use links::parse_wikilinks;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: RecordId,
    pub title: String,
    pub snippet: String,
    pub in_title: bool,
}
struct Entry {
    title: String,
    title_lower: String,
    body: String,
    body_lower: String,
    tags: Vec<String>,
    updated_at: i64,
    links: BTreeSet<RecordId>,
}
#[derive(Default)]
pub struct Index {
    entries: BTreeMap<RecordId, Entry>,
    removed: BTreeSet<RecordId>,
}
impl Index {
    pub fn upsert(&mut self, r: &RecordDoc) {
        if r.deleted_at().is_some() {
            self.remove(&r.id());
            return;
        }
        let title = r.title();
        let body = r.body();
        let mut links: BTreeSet<_> = parse_wikilinks(&body)
            .into_iter()
            .map(|l| l.target)
            .collect();
        for value in r.fields().into_values() {
            if let FieldValue::Links(ids) = value {
                links.extend(ids);
            }
        }
        self.removed.remove(&r.id());
        self.entries.insert(
            r.id(),
            Entry {
                title_lower: title.to_lowercase(),
                body_lower: body.to_lowercase(),
                title,
                body,
                tags: r.tags(),
                updated_at: r.updated_at(),
                links,
            },
        );
    }
    pub fn remove(&mut self, id: &RecordId) {
        self.entries.remove(id);
        self.removed.insert(*id);
    }
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let query = query.trim().to_lowercase();
        if query.is_empty() || limit == 0 {
            return Vec::new();
        }
        let mut matches: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(id, e)| {
                let title = e.title_lower.contains(&query);
                (title
                    || e.body_lower.contains(&query)
                    || e.tags.iter().any(|t| t.contains(&query)))
                .then_some((*id, e, title))
            })
            .collect();
        matches.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then(b.1.updated_at.cmp(&a.1.updated_at))
                .then(a.0.cmp(&b.0))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|(id, e, in_title)| SearchHit {
                id,
                title: e.title.clone(),
                snippet: snippet(&e.body, &e.body_lower, &query),
                in_title,
            })
            .collect()
    }
    pub fn backlinks(&self, id: &RecordId) -> Vec<RecordId> {
        if self.removed.contains(id) {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter_map(|(source, e)| e.links.contains(id).then_some(*source))
            .collect()
    }
}
fn snippet(body: &str, lower: &str, query: &str) -> String {
    let offset = lower.find(query).unwrap_or(0);
    let mut lowered_bytes: usize = 0;
    let mut position: usize = 0;
    for c in body.chars() {
        if lowered_bytes >= offset {
            break;
        }
        lowered_bytes += c.to_lowercase().map(char::len_utf8).sum::<usize>();
        position += 1;
    }
    body.chars()
        .skip(position.saturating_sub(25))
        .take(80)
        .collect()
}
