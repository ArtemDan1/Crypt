use super::{ids::*, types::*};
use crate::{CryptError, Result};
use loro::{ExportMode, LoroDoc, LoroMap, LoroValue};
use serde::{Serialize, de::DeserializeOwned};

pub(crate) fn crdt(_: impl std::fmt::Display) -> CryptError {
    CryptError::Crdt("invalid document operation".into())
}
pub(crate) fn json_string(map: &LoroMap, key: &str) -> Option<String> {
    match map.get(key)?.get_deep_value() {
        LoroValue::String(s) => Some(s.to_string()),
        _ => None,
    }
}
pub(crate) fn read_json<T: DeserializeOwned>(map: &LoroMap, key: &str) -> Option<T> {
    json_string(map, key).and_then(|s| serde_json::from_str(&s).ok())
}
fn values<T: DeserializeOwned>(map: &LoroMap) -> Result<Vec<T>> {
    let mut out = Vec::new();
    map.for_each(|_, v| {
        let result = match v.get_deep_value() {
            LoroValue::String(s) => serde_json::from_str::<T>(&s).map_err(|_| crdt("invalid JSON")),
            _ => Err(crdt("invalid map value")),
        };
        out.push(result);
    });
    out.into_iter().collect()
}
pub struct MetaDoc {
    doc: LoroDoc,
}
impl MetaDoc {
    pub fn new(peer: u64) -> Self {
        let doc = LoroDoc::new();
        doc.set_peer_id(peer).expect("new attached doc");
        doc.set_record_timestamp(true);
        Self { doc }
    }
    pub fn export(&self) -> Vec<u8> {
        self.doc.commit();
        self.doc
            .export(ExportMode::Snapshot)
            .expect("attached document export")
    }
    pub fn import(peer: u64, bytes: &[u8]) -> Result<Self> {
        let m = Self::new(peer);
        m.doc.import(bytes).map_err(crdt)?;
        for t in values::<TypeDef>(&m.doc.get_map("types"))? {
            t.validate()?;
        }
        values::<Folder>(&m.doc.get_map("folders"))?;
        values::<ViewDef>(&m.doc.get_map("views"))?;
        values::<Snapshot>(&m.doc.get_map("snapshots"))?;
        Ok(m)
    }
    fn put<T: Serialize>(&mut self, map: &str, key: &str, value: &T) -> Result<()> {
        self.doc
            .get_map(map)
            .insert(key, serde_json::to_string(value)?)
            .map_err(crdt)?;
        self.doc.commit();
        Ok(())
    }
    fn remove(&mut self, map: &str, key: &str) {
        self.doc.get_map(map).delete(key).expect("attached map");
        self.doc.commit();
    }
    /// User-facing vault name; `None` until the user renames it.
    pub fn vault_name(&self) -> Option<String> {
        json_string(&self.doc.get_map("vault"), "name")
    }
    pub fn set_vault_name(&mut self, name: &str) -> Result<()> {
        self.doc
            .get_map("vault")
            .insert("name", name)
            .map_err(crdt)?;
        self.doc.commit();
        Ok(())
    }
    pub fn list_types(&self) -> Vec<TypeDef> {
        let mut out = values::<TypeDef>(&self.doc.get_map("types")).expect("validated metadata");
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
    pub fn get_type(&self, id: &TypeId) -> Option<TypeDef> {
        read_json(&self.doc.get_map("types"), &id.to_string())
    }
    pub fn put_type(&mut self, t: TypeDef) -> Result<()> {
        t.validate()?;
        self.put("types", &t.id.to_string(), &t)
    }
    pub fn remove_type(&mut self, id: &TypeId) {
        self.remove("types", &id.to_string());
    }
    pub fn list_folders(&self) -> Vec<Folder> {
        let mut out = values::<Folder>(&self.doc.get_map("folders")).expect("validated metadata");
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
    pub fn put_folder(&mut self, f: Folder) -> Result<()> {
        if f.name.trim().is_empty() {
            return Err(CryptError::Validation("folder name is empty".into()));
        }
        let folders = self.list_folders();
        let mut cursor = f.parent;
        let mut seen = std::collections::HashSet::new();
        while let Some(id) = cursor {
            if id == f.id || !seen.insert(id) {
                return Err(CryptError::Validation("folder cycle".into()));
            }
            cursor = folders
                .iter()
                .find(|x| x.id == id)
                .ok_or_else(|| CryptError::Validation("parent folder missing".into()))?
                .parent;
        }
        self.put("folders", &f.id.to_string(), &f)
    }
    pub fn remove_folder(&mut self, id: &FolderId) {
        self.remove("folders", &id.to_string());
    }
    pub fn list_views(&self, id: &TypeId) -> Vec<ViewDef> {
        let mut out: Vec<ViewDef> = values::<ViewDef>(&self.doc.get_map("views"))
            .expect("validated metadata")
            .into_iter()
            .filter(|v| v.type_id == *id)
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
    pub fn put_view(&mut self, v: ViewDef) -> Result<()> {
        if self.get_type(&v.type_id).is_none() || v.name.trim().is_empty() {
            return Err(CryptError::Validation("view needs a type and name".into()));
        }
        self.put("views", &v.id.to_string(), &v)
    }
    pub fn remove_view(&mut self, id: &ViewId) {
        self.remove("views", &id.to_string());
    }
    pub fn add_tombstone(&mut self, id: &RecordId, at_ms: i64) {
        self.doc
            .get_map("purged")
            .insert(&id.to_string(), at_ms)
            .expect("attached map");
        self.doc.commit();
    }
    pub fn tombstones(&self) -> Vec<(RecordId, i64)> {
        let mut out = Vec::new();
        self.doc.get_map("purged").for_each(|k, v| {
            if let (Ok(id), LoroValue::I64(at)) = (k.parse(), v.get_deep_value()) {
                out.push((id, at));
            }
        });
        out.sort();
        out
    }
    pub fn list_snapshots(&self) -> Vec<Snapshot> {
        let mut out =
            values::<Snapshot>(&self.doc.get_map("snapshots")).expect("validated metadata");
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
    }
    pub fn get_snapshot(&self, id: &SnapshotId) -> Option<Snapshot> {
        read_json(&self.doc.get_map("snapshots"), &id.to_string())
    }
    pub fn put_snapshot(&mut self, s: &Snapshot) -> Result<()> {
        self.put("snapshots", &s.id.to_string(), s)
    }
    pub fn remove_snapshot(&mut self, id: &SnapshotId) {
        self.remove("snapshots", &id.to_string());
    }
}

/// A named point of the whole vault: each record's version and trash state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub name: String,
    pub created_at: i64,
    pub records: std::collections::BTreeMap<RecordId, SnapshotEntry>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SnapshotEntry {
    pub version: String,
    pub trashed: bool,
}
/// What the UI lists (record map omitted).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SnapshotInfo {
    pub id: SnapshotId,
    pub name: String,
    pub created_at: i64,
    pub records: usize,
}
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SnapshotRestoreReport {
    /// Records whose contents or trash state changed.
    pub restored: usize,
    /// Records created after the snapshot, moved to trash.
    pub trashed: usize,
    /// Records in the snapshot that were purged since.
    pub missing: usize,
    /// Secrets imported after the snapshot and never edited, removed for good.
    #[serde(default)]
    pub removed: usize,
}
impl Snapshot {
    pub fn info(&self) -> SnapshotInfo {
        SnapshotInfo {
            id: self.id,
            name: self.name.clone(),
            created_at: self.created_at,
            records: self.records.len(),
        }
    }
}
