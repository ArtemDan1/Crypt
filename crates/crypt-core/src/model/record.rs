use super::{
    ids::*,
    meta::{crdt, json_string, read_json},
    secret::SecretKind,
    types::FieldValue,
};
use crate::{CryptError, Result};
use loro::{ExportMode, LoroDoc, LoroValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NewRecord {
    pub title: String,
    pub type_id: Option<TypeId>,
    pub folder: Option<FolderId>,
    pub secret_kind: Option<SecretKind>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecordView {
    pub id: RecordId,
    pub title: String,
    pub body: String,
    pub type_id: Option<TypeId>,
    pub folder: Option<FolderId>,
    pub tags: Vec<String>,
    pub fields: BTreeMap<FieldId, FieldValue>,
    pub hidden_fields: BTreeMap<FieldId, FieldValue>,
    pub secret_kind: Option<SecretKind>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecordSummary {
    pub id: RecordId,
    pub title: String,
    pub type_id: Option<TypeId>,
    pub folder: Option<FolderId>,
    pub tags: Vec<String>,
    pub secret_kind: Option<SecretKind>,
    /// Login username (a public secret value), shown in lists and searchable.
    #[serde(default)]
    pub username: Option<String>,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListFilter {
    pub type_id: Option<TypeId>,
    pub folder: Option<FolderId>,
    pub tag: Option<String>,
    pub secrets: Option<bool>,
    pub trashed: bool,
}
pub fn normalize_tag(tag: &str) -> Result<String> {
    let t = tag.trim().trim_start_matches('#').trim().to_lowercase();
    if t.is_empty() {
        return Err(CryptError::Validation("empty tag".into()));
    }
    Ok(t)
}
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
pub struct RecordDoc {
    pub(crate) doc: LoroDoc,
    id: RecordId,
    /// Overrides the per-edit commit label (e.g. everything during creation is "create").
    label: Option<&'static str>,
}
/// Edits with the same label from one device within this window form one version.
const VERSION_WINDOW_S: i64 = 300;
fn label_for(key: &str) -> &'static str {
    match key {
        "folder_id" => "folder",
        _ => "type",
    }
}
/// One entry of a record's history. `id` is opaque: "<peer>:<counter>".
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecordVersion {
    pub id: String,
    pub at: i64,
    pub kind: String,
    pub this_device: bool,
}
impl std::fmt::Debug for RecordDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordDoc")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
impl RecordDoc {
    pub fn new(peer: u64, id: RecordId, title: &str, now: i64) -> Self {
        Self::new_as(peer, id, title, now, "create")
    }
    /// A new record whose first version carries `label` (e.g. "import").
    pub(crate) fn new_as(
        peer: u64,
        id: RecordId,
        title: &str,
        now: i64,
        label: &'static str,
    ) -> Self {
        let doc = LoroDoc::new();
        doc.set_peer_id(peer).expect("new attached doc");
        doc.set_record_timestamp(true);
        doc.set_change_merge_interval(VERSION_WINDOW_S);
        doc.set_next_commit_message(label);
        let m = doc.get_map("meta");
        for (k, v) in [("id", id.to_string()), ("title", title.to_string())] {
            m.insert(k, v).expect("attached map");
        }
        m.insert("created_at", now).expect("attached map");
        m.insert("updated_at", now).expect("attached map");
        doc.commit();
        Self {
            doc,
            id,
            label: None,
        }
    }
    pub fn import(peer: u64, bytes: &[u8]) -> Result<Self> {
        let doc = LoroDoc::new();
        doc.import(bytes).map_err(crdt)?;
        doc.set_peer_id(peer).map_err(crdt)?;
        doc.set_record_timestamp(true);
        doc.set_change_merge_interval(VERSION_WINDOW_S);
        let meta = doc.get_map("meta");
        let id = json_string(&meta, "id")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| crdt("missing id"))?;
        if json_string(&meta, "title").is_none() {
            return Err(crdt("missing title"));
        }
        for key in ["created_at", "updated_at"] {
            if !matches!(
                meta.get(key).map(|v| v.get_deep_value()),
                Some(LoroValue::I64(_))
            ) {
                return Err(crdt("invalid timestamp"));
            }
        }
        for (key, valid) in [
            ("type_id", read_json::<TypeId>(&meta, "type_id").is_some()),
            (
                "folder_id",
                read_json::<FolderId>(&meta, "folder_id").is_some(),
            ),
            (
                "secret_kind",
                read_json::<SecretKind>(&meta, "secret_kind").is_some(),
            ),
        ] {
            if meta.get(key).is_some() && !valid {
                return Err(crdt("invalid optional metadata"));
            }
        }
        if let Some(v) = meta.get("deleted_at")
            && !matches!(v.get_deep_value(), LoroValue::I64(_))
        {
            return Err(crdt("invalid deleted timestamp"));
        }
        if let Some(v) = meta.get("agent_enabled")
            && !matches!(v.get_deep_value(), LoroValue::Bool(_))
        {
            return Err(crdt("invalid agent flag"));
        }
        // Refuse malformed field payloads rather than silently losing values.
        let mut bad = false;
        doc.get_map("fields").for_each(|k, v| {
            if k.parse::<FieldId>().is_err() {
                bad = true;
            }
            match v.get_deep_value() {
                LoroValue::String(s) => {
                    if serde_json::from_str::<FieldValue>(&s).is_err() {
                        bad = true;
                    }
                }
                _ => bad = true,
            }
        });
        if bad {
            return Err(crdt("invalid field"));
        }
        Ok(Self {
            doc,
            id,
            label: None,
        })
    }
    pub fn export(&self) -> Vec<u8> {
        self.doc.commit();
        self.doc
            .export(ExportMode::Snapshot)
            .expect("attached document export")
    }
    pub fn id(&self) -> RecordId {
        self.id
    }
    pub fn title(&self) -> String {
        json_string(&self.doc.get_map("meta"), "title").unwrap_or_default()
    }
    pub fn body(&self) -> String {
        self.doc.get_text("body").to_string()
    }
    fn number(&self, key: &str) -> Option<i64> {
        match self.doc.get_map("meta").get(key)?.get_deep_value() {
            LoroValue::I64(n) => Some(n),
            _ => None,
        }
    }
    pub fn created_at(&self) -> i64 {
        self.number("created_at").unwrap_or_default()
    }
    pub fn updated_at(&self) -> i64 {
        self.number("updated_at").unwrap_or_default()
    }
    pub fn deleted_at(&self) -> Option<i64> {
        self.number("deleted_at")
    }
    pub fn type_id(&self) -> Option<TypeId> {
        read_json(&self.doc.get_map("meta"), "type_id")
    }
    pub fn folder(&self) -> Option<FolderId> {
        read_json(&self.doc.get_map("meta"), "folder_id")
    }
    pub fn secret_kind(&self) -> Option<SecretKind> {
        read_json(&self.doc.get_map("meta"), "secret_kind")
    }
    pub(crate) fn touch(&self, kind: &'static str) -> Result<()> {
        self.doc.set_next_commit_message(self.label.unwrap_or(kind));
        self.doc
            .get_map("meta")
            .insert(
                "updated_at",
                now_ms().max(self.updated_at().saturating_add(1)),
            )
            .map_err(crdt)?;
        self.doc.commit();
        Ok(())
    }
    fn set_json<T: Serialize>(&self, key: &str, value: Option<T>) -> Result<()> {
        let m = self.doc.get_map("meta");
        if let Some(v) = value {
            m.insert(key, serde_json::to_string(&v)?).map_err(crdt)?;
        } else {
            m.delete(key).map_err(crdt)?;
        }
        self.touch(label_for(key))
    }
    pub fn set_title(&mut self, title: &str) -> Result<()> {
        self.doc
            .get_map("meta")
            .insert("title", title)
            .map_err(crdt)?;
        self.touch("title")
    }
    pub fn set_body(&mut self, body: &str) -> Result<()> {
        self.doc
            .get_text("body")
            .update(body, Default::default())
            .map_err(crdt)?;
        self.touch("body")
    }
    pub fn set_type(&mut self, id: Option<TypeId>) -> Result<()> {
        self.set_json("type_id", id)
    }
    pub fn set_folder(&mut self, id: Option<FolderId>) -> Result<()> {
        self.set_json("folder_id", id)
    }
    pub fn set_secret_kind(&mut self, kind: Option<SecretKind>) -> Result<()> {
        self.set_json("secret_kind", kind)
    }
    pub fn set_deleted_at(&mut self, at: Option<i64>) -> Result<()> {
        let m = self.doc.get_map("meta");
        if let Some(at) = at {
            m.insert("deleted_at", at).map_err(crdt)?;
        } else {
            m.delete("deleted_at").map_err(crdt)?;
        }
        self.touch("trash")
    }
    pub fn tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        self.doc.get_map("tags").for_each(|k, v| {
            if matches!(v.get_deep_value(), LoroValue::Bool(true)) {
                tags.push(k.to_string());
            }
        });
        tags.sort();
        tags
    }
    pub fn add_tag(&mut self, tag: &str) -> Result<()> {
        self.doc
            .get_map("tags")
            .insert(&normalize_tag(tag)?, true)
            .map_err(crdt)?;
        self.touch("tags")
    }
    pub fn remove_tag(&mut self, tag: &str) -> Result<()> {
        self.doc
            .get_map("tags")
            .delete(&normalize_tag(tag)?)
            .map_err(crdt)?;
        self.touch("tags")
    }
    pub fn fields(&self) -> BTreeMap<FieldId, FieldValue> {
        let mut out = BTreeMap::new();
        let m = self.doc.get_map("fields");
        m.for_each(|k, _| {
            if let (Ok(id), Some(v)) = (k.parse(), read_json(&m, k)) {
                out.insert(id, v);
            }
        });
        out
    }
    pub fn field(&self, id: &FieldId) -> Option<FieldValue> {
        read_json(&self.doc.get_map("fields"), &id.to_string())
    }
    pub fn set_field(&mut self, id: FieldId, value: Option<FieldValue>) -> Result<()> {
        if matches!(&value, Some(FieldValue::Number(n)) if !n.is_finite()) {
            return Err(CryptError::Validation("number must be finite".into()));
        }
        let m = self.doc.get_map("fields");
        if let Some(v) = value {
            m.insert(&id.to_string(), serde_json::to_string(&v)?)
                .map_err(crdt)?;
        } else {
            m.delete(&id.to_string()).map_err(crdt)?;
        }
        self.touch("fields")
    }
    pub fn summary(&self) -> RecordSummary {
        RecordSummary {
            id: self.id,
            title: self.title(),
            type_id: self.type_id(),
            folder: self.folder(),
            tags: self.tags(),
            secret_kind: self.secret_kind(),
            username: (self.secret_kind() == Some(SecretKind::Login))
                .then(|| json_string(&self.doc.get_map("secret"), "username"))
                .flatten()
                .filter(|s| !s.is_empty()),
            updated_at: self.updated_at(),
        }
    }
}

impl RecordDoc {
    fn validate_secret_name(&self, name: &str) -> Result<()> {
        if !self
            .secret_kind()
            .is_some_and(|k| k.value_names().contains(&name))
        {
            return Err(CryptError::Validation(
                "invalid secret value name or record kind".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn secret_value(&self, name: &str) -> Result<Option<zeroize::Zeroizing<String>>> {
        self.validate_secret_name(name)?;
        Ok(json_string(&self.doc.get_map("secret"), name).map(zeroize::Zeroizing::new))
    }
    pub(crate) fn set_secret_value(&mut self, name: &str, value: Option<&str>) -> Result<()> {
        self.validate_secret_name(name)?;
        let old = self.secret_value(name)?;
        if old.as_deref().map(String::as_str) == value {
            return Ok(());
        }
        if let Some(old) = old.filter(|s| !s.is_empty()) {
            let history = self.doc.get_map("secret_history");
            let list = match history.get(name) {
                Some(loro::ValueOrContainer::Container(loro::Container::List(list))) => list,
                Some(_) => return Err(crdt("invalid history container")),
                None => history
                    .insert_container(name, loro::LoroList::new())
                    .map_err(crdt)?,
            };
            #[derive(Serialize)]
            struct Revision<'a> {
                value: &'a str,
                replaced_at: i64,
            }
            let json = zeroize::Zeroizing::new(serde_json::to_string(&Revision {
                value: &old,
                replaced_at: now_ms(),
            })?);
            list.push(json.as_str()).map_err(crdt)?;
        }
        let map = self.doc.get_map("secret");
        if let Some(value) = value {
            map.insert(name, value).map_err(crdt)?;
        } else {
            map.delete(name).map_err(crdt)?;
        }
        self.touch("secret")
    }
    pub(crate) fn secret_history(&self, name: &str) -> Result<Vec<super::secret::SecretRevision>> {
        self.validate_secret_name(name)?;
        let mut out = Vec::new();
        let history = self.doc.get_map("secret_history");
        let list = match history.get(name) {
            None => return Ok(out),
            Some(loro::ValueOrContainer::Container(loro::Container::List(list))) => list,
            _ => return Err(crdt("invalid history")),
        };
        #[derive(Deserialize)]
        struct Revision {
            value: String,
            replaced_at: i64,
        }
        for i in (0..list.len()).rev() {
            let v = list.get(i).ok_or_else(|| crdt("invalid history entry"))?;
            let json = match v.get_deep_value() {
                LoroValue::String(s) => zeroize::Zeroizing::new(s.to_string()),
                _ => return Err(crdt("invalid history entry")),
            };
            let r: Revision = serde_json::from_str(&json).map_err(crdt)?;
            out.push(super::secret::SecretRevision {
                value: zeroize::Zeroizing::new(r.value),
                replaced_at: r.replaced_at,
            });
        }
        Ok(out)
    }
    pub fn agent_enabled(&self) -> bool {
        matches!(
            self.doc
                .get_map("meta")
                .get("agent_enabled")
                .map(|v| v.get_deep_value()),
            Some(LoroValue::Bool(true))
        )
    }
    pub fn set_agent_enabled(&mut self, enabled: bool) -> Result<()> {
        if self.secret_kind() != Some(SecretKind::SshKey) {
            return Err(CryptError::Validation("agent requires an SSH key".into()));
        }
        self.doc
            .get_map("meta")
            .insert("agent_enabled", enabled)
            .map_err(crdt)?;
        self.touch("agent")
    }
}

impl RecordDoc {
    /// Run edits under one commit label (e.g. "create", "revert").
    pub(crate) fn with_label<T>(
        &mut self,
        label: &'static str,
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let prev = self.label.replace(label);
        let out = f(self);
        self.label = prev;
        out
    }
    /// History, newest first. Adjacent edits with the same label from one device
    /// within `VERSION_WINDOW_S` collapse into one version (the latest state wins).
    pub fn versions(&self, this_peer: u64) -> Vec<RecordVersion> {
        self.doc.commit();
        let heads: Vec<loro::ID> = self.doc.oplog_frontiers().iter().collect();
        let mut changes = Vec::new();
        let _ = self.doc.travel_change_ancestors(&heads, &mut |c| {
            changes.push(c);
            std::ops::ControlFlow::Continue(())
        });
        changes.sort_by_key(|c| (c.lamport, c.id.peer));
        let mut out: Vec<(loro::ChangeMeta, RecordVersion)> = Vec::new();
        for c in changes {
            let kind = c.message.as_deref().unwrap_or("edit").to_string();
            let last = c.id.counter + c.len as i32 - 1;
            let v = RecordVersion {
                id: format!("{}:{}", c.id.peer, last),
                at: c.timestamp.saturating_mul(1000),
                kind,
                this_device: c.id.peer == this_peer,
            };
            if let Some((prev, pv)) = out.last_mut()
                && prev.id.peer == c.id.peer
                && pv.kind == v.kind
                && c.timestamp - prev.timestamp <= VERSION_WINDOW_S
            {
                *pv = v;
                *prev = c;
                continue;
            }
            out.push((c, v));
        }
        out.into_iter().rev().map(|(_, v)| v).collect()
    }
    fn parse_version(&self, version: &str) -> Result<loro::ID> {
        let bad = || CryptError::Validation("unknown version".into());
        let (peer, counter) = version.split_once(':').ok_or_else(bad)?;
        let id = loro::ID::new(
            peer.parse().map_err(|_| bad())?,
            counter.parse().map_err(|_| bad())?,
        );
        if !self.doc.oplog_vv().includes_id(id) {
            return Err(bad());
        }
        Ok(id)
    }
    /// The record as it was at `version` (read-only copy, never persisted).
    pub fn at(&self, version: &str) -> Result<RecordDoc> {
        self.doc.commit();
        let id = self.parse_version(version)?;
        let doc = self
            .doc
            .fork_at(&loro::Frontiers::from_id(id))
            .map_err(crdt)?;
        Ok(Self {
            doc,
            id: self.id,
            label: None,
        })
    }
    /// The current head as a version id (used by vault snapshots).
    pub fn head(&self) -> String {
        self.versions(0)
            .first()
            .map(|v| v.id.clone())
            .unwrap_or_default()
    }
}
