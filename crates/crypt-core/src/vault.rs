use crate::{
    CryptError, Result, SecretKey, SeedPhrase, VaultId,
    crypto::{
        envelope::{open, seal},
        keys::VaultKeys,
    },
    storage::{
        fs::{read_file, write_atomic},
        layout::{Aad, VaultPaths},
    },
};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize)]
struct Header {
    format: u32,
    vault_id: VaultId,
    created_at: i64,
}
#[derive(Serialize, Deserialize)]
struct Device {
    peer_id: u64,
}

use crate::model::{ids::*, meta::MetaDoc, types::*};

use crate::model::record::{
    ListFilter, NewRecord, RecordDoc, RecordSummary, RecordView, normalize_tag,
};
use std::collections::{BTreeMap, HashSet};
use zeroize::Zeroizing;

pub struct Vault {
    index: crate::index::Index,
    records: BTreeMap<RecordId, RecordDoc>,
    load_errors: Vec<CryptError>,
    meta: MetaDoc,
    paths: VaultPaths,
    keys: VaultKeys,
    peer_id: u64,
}
pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
impl Vault {
    pub fn create(root: &Path, seed: &SeedPhrase) -> Result<Self> {
        if root.exists() && (!root.is_dir() || fs::read_dir(root)?.next().is_some()) {
            return Err(CryptError::VaultExists);
        }
        fs::create_dir_all(root)?;
        let paths = VaultPaths::new(root);
        fs::create_dir(paths.objects_dir())?;
        let keys = VaultKeys::derive(seed);
        write_atomic(
            &paths.keycheck(),
            &seal(&keys.data_key, &Aad::KeyCheck.bytes(), b"crypt-keycheck-v1"),
        )?;
        write_atomic(
            &paths.seed(),
            &seal(&keys.data_key, &Aad::Seed.bytes(), seed.as_str().as_bytes()),
        )?;
        let peer_id = Self::device_peer(&paths)?;
        write_atomic(
            &paths.header(),
            &serde_json::to_vec(&Header {
                format: 1,
                vault_id: keys.vault_id.clone(),
                created_at: now_ms(),
            })?,
        )?;
        let meta = MetaDoc::new(peer_id);
        write_atomic(
            &paths.meta(),
            &seal(&keys.data_key, &Aad::Meta.bytes(), &meta.export()),
        )?;
        Ok(Self {
            paths,
            keys,
            peer_id,
            meta,
            records: BTreeMap::new(),
            load_errors: Vec::new(),
            index: crate::index::Index::default(),
        })
    }
    fn device_peer(paths: &VaultPaths) -> Result<u64> {
        if paths.device().exists() {
            return Ok(serde_json::from_slice::<Device>(&read_file(&paths.device())?)?.peer_id);
        }
        let peer_id = OsRng.next_u64();
        write_atomic(&paths.device(), &serde_json::to_vec(&Device { peer_id })?)?;
        Ok(peer_id)
    }
    pub fn open_with_seed(root: &Path, seed: &SeedPhrase) -> Result<Self> {
        Self::open_with_key(root, VaultKeys::derive(seed).data_key)
    }
    pub fn open_with_key(root: &Path, data_key: SecretKey) -> Result<Self> {
        let paths = VaultPaths::new(root);
        let bytes = match read_file(&paths.header()) {
            Err(CryptError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CryptError::NotAVault);
            }
            other => other?,
        };
        let header: Header = serde_json::from_slice(&bytes)?;
        if header.format != 1 {
            return Err(CryptError::UnsupportedFormat(header.format));
        }
        let check = open(
            &data_key,
            &Aad::KeyCheck.bytes(),
            &read_file(&paths.keycheck())?,
        )
        .map_err(|_| CryptError::WrongKey)?;
        if check.as_slice() != b"crypt-keycheck-v1" {
            return Err(CryptError::WrongKey);
        }
        let plain = open(&data_key, &Aad::Seed.bytes(), &read_file(&paths.seed())?)?;
        let text = std::str::from_utf8(&plain).map_err(|_| CryptError::Corrupt {
            file: "seed.bin".into(),
        })?;
        let seed = SeedPhrase::parse(text)?;
        let keys = VaultKeys::derive(&seed);
        if keys.data_key.as_bytes() != data_key.as_bytes() || header.vault_id != keys.vault_id {
            return Err(CryptError::WrongKey);
        }
        let peer_id = Self::device_peer(&paths)?;
        let plain = open(
            &keys.data_key,
            &Aad::Meta.bytes(),
            &read_file(&paths.meta())?,
        )?;
        let meta = MetaDoc::import(peer_id, &plain)?;
        let mut vault = Self {
            paths,
            keys,
            peer_id,
            meta,
            records: BTreeMap::new(),
            load_errors: Vec::new(),
            index: crate::index::Index::default(),
        };
        vault.load_records()?;
        Ok(vault)
    }
    pub fn data_key(&self) -> &SecretKey {
        &self.keys.data_key
    }
    pub fn vault_id(&self) -> &VaultId {
        &self.keys.vault_id
    }
    pub fn reveal_seed(&self) -> Result<SeedPhrase> {
        let plain = open(
            &self.keys.data_key,
            &Aad::Seed.bytes(),
            &read_file(&self.paths.seed())?,
        )?;
        SeedPhrase::parse(std::str::from_utf8(&plain).map_err(|_| CryptError::InvalidSeed)?)
    }
    pub fn lock(self) {}
}

impl Vault {
    // Stage changes in an independent document: failed disk writes never change the live state.
    fn edit_meta<T>(&mut self, edit: impl FnOnce(&mut MetaDoc) -> Result<T>) -> Result<T> {
        let mut staged = MetaDoc::import(self.peer_id, &self.meta.export())?;
        let result = edit(&mut staged)?;
        write_atomic(
            &self.paths.meta(),
            &seal(&self.keys.data_key, &Aad::Meta.bytes(), &staged.export()),
        )?;
        self.meta = staged;
        Ok(result)
    }
    pub fn types(&self) -> Vec<TypeDef> {
        self.meta.list_types()
    }
    pub fn put_type(&mut self, t: TypeDef) -> Result<()> {
        self.edit_meta(|m| m.put_type(t))
    }
    pub fn delete_type(&mut self, id: &TypeId) -> Result<()> {
        if self.records.values().any(|r| r.type_id() == Some(*id)) {
            return Err(CryptError::Validation("type is in use".into()));
        }
        if self.meta.get_type(id).is_none() {
            return Err(CryptError::NotFound(id.to_string()));
        }
        self.edit_meta(|m| {
            for v in m.list_views(id) {
                m.remove_view(&v.id);
            }
            m.remove_type(id);
            Ok(())
        })
    }
    pub fn folders(&self) -> Vec<Folder> {
        self.meta.list_folders()
    }
    fn folder(&self, id: &FolderId) -> Result<Folder> {
        self.folders()
            .into_iter()
            .find(|f| f.id == *id)
            .ok_or_else(|| CryptError::NotFound(id.to_string()))
    }
    pub fn create_folder(&mut self, name: &str, parent: Option<FolderId>) -> Result<FolderId> {
        let id = FolderId::new();
        self.edit_meta(|m| {
            m.put_folder(Folder {
                id,
                name: name.trim().into(),
                parent,
            })
        })?;
        Ok(id)
    }
    pub fn rename_folder(&mut self, id: &FolderId, name: &str) -> Result<()> {
        let mut f = self.folder(id)?;
        f.name = name.trim().into();
        self.edit_meta(|m| m.put_folder(f))
    }
    pub fn move_folder(&mut self, id: &FolderId, parent: Option<FolderId>) -> Result<()> {
        let mut f = self.folder(id)?;
        f.parent = parent;
        self.edit_meta(|m| m.put_folder(f))
    }
    pub fn delete_folder(&mut self, id: &FolderId) -> Result<()> {
        self.folder(id)?;
        if self.records.values().any(|r| r.folder() == Some(*id)) {
            return Err(CryptError::Validation("folder has records".into()));
        }
        if self.folders().iter().any(|f| f.parent == Some(*id)) {
            return Err(CryptError::Validation("folder has children".into()));
        }
        self.edit_meta(|m| {
            m.remove_folder(id);
            Ok(())
        })
    }
    pub fn views(&self, id: &TypeId) -> Vec<ViewDef> {
        self.meta.list_views(id)
    }
    pub fn put_view(&mut self, v: ViewDef) -> Result<()> {
        self.edit_meta(|m| m.put_view(v))
    }
    pub fn delete_view(&mut self, id: &ViewId) -> Result<()> {
        self.edit_meta(|m| {
            m.remove_view(id);
            Ok(())
        })
    }
}

impl Vault {
    fn load_records(&mut self) -> Result<()> {
        let purged: HashSet<_> = self
            .meta
            .tombstones()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let mut cleaned = false;
        for entry in fs::read_dir(self.paths.objects_dir())? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let object_name = name.strip_suffix(".bin").or_else(|| {
                name.ends_with(".tmp")
                    .then(|| name.split_once(".bin.").map(|(id, _)| id))
                    .flatten()
            });
            if object_name
                .and_then(|s| s.parse::<RecordId>().ok())
                .is_some_and(|id| purged.contains(&id))
            {
                match fs::remove_file(&path) {
                    Ok(()) => cleaned = true,
                    Err(_) => self.load_errors.push(CryptError::Corrupt { file: name }),
                }
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("bin") {
                continue;
            }
            let result = (|| {
                let id: RecordId = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| CryptError::Corrupt { file: name.clone() })?;
                if !entry.file_type()?.is_file() {
                    return Err(CryptError::Corrupt { file: name.clone() });
                }
                let plain = open(
                    &self.keys.data_key,
                    &Aad::Object(id).bytes(),
                    &read_file(&path)?,
                )?;
                let doc = RecordDoc::import(self.peer_id, &plain)?;
                if doc.id() != id {
                    return Err(CryptError::Corrupt { file: name.clone() });
                }
                Ok(Some((id, doc)))
            })();
            match result {
                Ok(Some((id, doc))) => {
                    self.index.upsert(&doc);
                    self.records.insert(id, doc);
                }
                Ok(None) => {}
                Err(_) => self.load_errors.push(CryptError::Corrupt { file: name }),
            }
        }
        #[cfg(unix)]
        if cleaned {
            fs::File::open(self.paths.objects_dir())?.sync_all()?;
        }
        #[cfg(not(unix))]
        let _ = cleaned;
        Ok(())
    }
    fn record(&self, id: &RecordId) -> Result<&RecordDoc> {
        self.records
            .get(id)
            .ok_or_else(|| CryptError::NotFound(id.to_string()))
    }
    fn persist_record(&self, r: &RecordDoc) -> Result<()> {
        let plain = Zeroizing::new(r.export());
        write_atomic(
            &self.paths.object(&r.id()),
            &seal(&self.keys.data_key, &Aad::Object(r.id()).bytes(), &plain),
        )
    }
    fn edit_record(
        &mut self,
        id: &RecordId,
        edit: impl FnOnce(&mut RecordDoc) -> Result<()>,
    ) -> Result<()> {
        let plain = Zeroizing::new(self.record(id)?.export());
        let mut staged = RecordDoc::import(self.peer_id, &plain)?;
        edit(&mut staged)?;
        self.persist_record(&staged)?;
        self.index.upsert(&staged);
        self.records.insert(*id, staged);
        Ok(())
    }
    fn validate_type(&self, id: Option<TypeId>) -> Result<()> {
        if id.is_some_and(|id| self.meta.get_type(&id).is_none()) {
            return Err(CryptError::Validation("unknown type".into()));
        }
        Ok(())
    }
    fn validate_folder(&self, id: Option<FolderId>) -> Result<()> {
        if let Some(id) = id {
            self.folder(&id)?;
        }
        Ok(())
    }
    pub fn create_record(&mut self, new: NewRecord) -> Result<RecordId> {
        self.validate_type(new.type_id)?;
        self.validate_folder(new.folder)?;
        let id = RecordId::new();
        let mut r = RecordDoc::new(self.peer_id, id, &new.title, now_ms());
        r.with_label("create", |r| {
            r.set_type(new.type_id)?;
            r.set_folder(new.folder)?;
            r.set_secret_kind(new.secret_kind)?;
            if new.secret_kind == Some(SecretKind::SshKey) {
                r.set_agent_enabled(true)?;
            }
            Ok(())
        })?;
        self.persist_record(&r)?;
        self.index.upsert(&r);
        self.records.insert(id, r);
        Ok(id)
    }
    /// Build a whole record in memory and write it once; every edit shares one `label` version.
    pub(crate) fn insert_built(
        &mut self,
        id: RecordId,
        new: NewRecord,
        label: &'static str,
        build: impl FnOnce(&mut RecordDoc) -> Result<()>,
    ) -> Result<RecordId> {
        self.validate_type(new.type_id)?;
        self.validate_folder(new.folder)?;
        if self.records.contains_key(&id) {
            return Err(CryptError::Validation("duplicate record id".into()));
        }
        let mut r = RecordDoc::new_as(self.peer_id, id, &new.title, now_ms(), label);
        r.with_label(label, |r| {
            r.set_type(new.type_id)?;
            r.set_folder(new.folder)?;
            r.set_secret_kind(new.secret_kind)?;
            build(r)
        })?;
        self.persist_record(&r)?;
        self.index.upsert(&r);
        self.records.insert(id, r);
        Ok(id)
    }
    pub fn get(&self, id: &RecordId) -> Result<RecordView> {
        Ok(self.view_of(self.record(id)?))
    }
    fn view_of(&self, r: &RecordDoc) -> RecordView {
        let t = r.type_id().and_then(|id| self.meta.get_type(&id));
        let mut fields = BTreeMap::new();
        let mut hidden_fields = BTreeMap::new();
        for (id, mut value) in r.fields() {
            if t.as_ref()
                .and_then(|t| t.fields.iter().find(|f| f.id == id))
                .is_some_and(|f| value.fits(&f.kind))
            {
                if let FieldValue::Links(ids) = &mut value {
                    ids.retain(|id| self.records.contains_key(id));
                }
                fields.insert(id, value);
            } else {
                hidden_fields.insert(id, value);
            }
        }
        RecordView {
            id: r.id(),
            title: r.title(),
            body: r.body(),
            type_id: r.type_id(),
            folder: r.folder(),
            tags: r.tags(),
            fields,
            hidden_fields,
            secret_kind: r.secret_kind(),
            created_at: r.created_at(),
            updated_at: r.updated_at(),
            deleted_at: r.deleted_at(),
        }
    }
    pub fn list(&self, filter: &ListFilter) -> Vec<RecordSummary> {
        let tag = match filter.tag.as_deref().map(normalize_tag).transpose() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        let mut out: Vec<_> = self
            .records
            .values()
            .filter(|r| {
                r.deleted_at().is_some() == filter.trashed
                    && filter.type_id.is_none_or(|id| r.type_id() == Some(id))
                    && filter.folder.is_none_or(|id| r.folder() == Some(id))
                    && filter
                        .secrets
                        .is_none_or(|secret| r.secret_kind().is_some() == secret)
                    && tag.as_ref().is_none_or(|tag| r.tags().contains(tag))
            })
            .map(RecordDoc::summary)
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
        out
    }
    pub fn set_title(&mut self, id: &RecordId, title: &str) -> Result<()> {
        self.edit_record(id, |r| r.set_title(title))
    }
    pub fn set_body(&mut self, id: &RecordId, body: &str) -> Result<()> {
        self.edit_record(id, |r| r.set_body(body))
    }
    pub fn set_type(&mut self, id: &RecordId, type_id: Option<TypeId>) -> Result<()> {
        self.validate_type(type_id)?;
        self.edit_record(id, |r| r.set_type(type_id))
    }
    pub fn set_folder(&mut self, id: &RecordId, folder: Option<FolderId>) -> Result<()> {
        self.validate_folder(folder)?;
        self.edit_record(id, |r| r.set_folder(folder))
    }
    pub fn add_tag(&mut self, id: &RecordId, tag: &str) -> Result<()> {
        self.edit_record(id, |r| r.add_tag(tag))
    }
    pub fn remove_tag(&mut self, id: &RecordId, tag: &str) -> Result<()> {
        self.edit_record(id, |r| r.remove_tag(tag))
    }
    pub fn set_field(
        &mut self,
        id: &RecordId,
        field: &FieldId,
        value: Option<FieldValue>,
    ) -> Result<()> {
        let t = self
            .record(id)?
            .type_id()
            .and_then(|id| self.meta.get_type(&id))
            .ok_or_else(|| CryptError::Validation("record has no current type".into()))?;
        let def = t
            .fields
            .iter()
            .find(|f| f.id == *field)
            .ok_or_else(|| CryptError::Validation("unknown field".into()))?;
        if value.as_ref().is_some_and(|v| !v.fits(&def.kind)) {
            return Err(CryptError::Validation("field kind mismatch".into()));
        }
        if let Some(FieldValue::Links(ids)) = &value {
            for target in ids {
                let target = self.record(target)?;
                match &def.kind {
                    FieldKind::SecretLink { .. } if target.secret_kind().is_none() => {
                        return Err(CryptError::Validation("link needs a secret".into()));
                    }
                    FieldKind::RecordLink {
                        target_type: Some(t),
                        ..
                    } if target.type_id() != Some(*t) => {
                        return Err(CryptError::Validation("linked record type mismatch".into()));
                    }
                    _ => {}
                }
            }
        }
        self.edit_record(id, |r| r.set_field(*field, value))
    }
    pub fn load_errors(&self) -> &[CryptError] {
        &self.load_errors
    }
}

use crate::model::secret::{SecretKind, SecretRevision};
impl Vault {
    pub fn secret_value(&self, id: &RecordId, name: &str) -> Result<Option<Zeroizing<String>>> {
        self.record(id)?.secret_value(name)
    }
    pub fn set_secret_value(
        &mut self,
        id: &RecordId,
        name: &str,
        value: Option<&str>,
    ) -> Result<()> {
        self.edit_record(id, |r| r.set_secret_value(name, value))
    }
    pub fn secret_history(&self, id: &RecordId, name: &str) -> Result<Vec<SecretRevision>> {
        self.record(id)?.secret_history(name)
    }
    pub fn set_agent_enabled(&mut self, id: &RecordId, enabled: bool) -> Result<()> {
        self.edit_record(id, |r| r.set_agent_enabled(enabled))
    }
    pub fn agent_keys(&self) -> Vec<(RecordId, String)> {
        self.records
            .values()
            .filter(|r| {
                r.secret_kind() == Some(SecretKind::SshKey)
                    && r.deleted_at().is_none()
                    && r.agent_enabled()
            })
            .map(|r| (r.id(), r.title()))
            .collect()
    }
    pub fn delete(&mut self, id: &RecordId) -> Result<()> {
        if self.record(id)?.secret_kind().is_some() {
            self.purge(id)
        } else {
            self.edit_record(id, |r| r.set_deleted_at(Some(now_ms())))
        }
    }
    pub fn restore(&mut self, id: &RecordId) -> Result<()> {
        self.edit_record(id, |r| r.set_deleted_at(None))
    }
    // Persist the tombstone before unlinking: after a crash a stale object must never be loaded.
    pub fn purge(&mut self, id: &RecordId) -> Result<()> {
        self.record(id)?;
        self.edit_meta(|m| {
            m.add_tombstone(id, now_ms());
            Ok(())
        })?;
        self.records.remove(id);
        self.index.remove(id);
        self.remove_object_files(id)
    }
    fn remove_object_files(&self, id: &RecordId) -> Result<()> {
        match fs::remove_file(self.paths.object(id)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let prefix = format!("{id}.bin.");
        for entry in fs::read_dir(self.paths.objects_dir())? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix) && name.ends_with(".tmp") {
                fs::remove_file(entry.path())?;
            }
        }
        #[cfg(unix)]
        fs::File::open(self.paths.objects_dir())?.sync_all()?;
        Ok(())
    }
    pub fn empty_trash(&mut self) -> Result<usize> {
        let ids: Vec<_> = self
            .records
            .values()
            .filter(|r| r.deleted_at().is_some())
            .map(RecordDoc::id)
            .collect();
        for id in &ids {
            self.purge(id)?;
        }
        Ok(ids.len())
    }
}

impl Vault {
    /// Tags of records outside the trash, most used first.
    pub fn all_tags(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for r in self.records.values().filter(|r| r.deleted_at().is_none()) {
            for t in r.tags() {
                *counts.entry(t).or_default() += 1;
            }
        }
        let mut out: Vec<_> = counts.into_iter().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }
    pub fn search(&self, query: &str, limit: usize) -> Vec<crate::SearchHit> {
        self.index.search(query, limit)
    }
    pub fn backlinks(&self, id: &RecordId) -> Vec<RecordSummary> {
        if !self.records.contains_key(id) {
            return Vec::new();
        }
        let mut out: Vec<_> = self
            .index
            .backlinks(id)
            .iter()
            .filter_map(|id| self.records.get(id))
            .filter(|r| r.deleted_at().is_none())
            .map(RecordDoc::summary)
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
        out
    }
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("vault_id", &self.keys.vault_id)
            .finish_non_exhaustive()
    }
}

impl Vault {
    pub fn set_ssh_private_key(
        &mut self,
        id: &RecordId,
        pem: &str,
        passphrase: Option<&str>,
    ) -> Result<crate::ssh::SshKeyInfo> {
        if self.record(id)?.secret_kind() != Some(SecretKind::SshKey) {
            return Err(CryptError::Validation("not an SSH key".into()));
        }
        let (info, plain) = crate::ssh::inspect_private_key(pem, passphrase)?;
        self.edit_record(id, |r| {
            r.set_secret_value("private_key", Some(&plain))?;
            r.set_secret_value("public_key", Some(&info.public_key))?;
            r.set_secret_value("passphrase", None)?;
            if r.secret_value("comment")?.is_none_or(|v| v.is_empty()) {
                r.set_secret_value("comment", Some(&info.comment))?;
            }
            Ok(())
        })?;
        Ok(info)
    }
    pub fn ssh_key_info(&self, id: &RecordId) -> Result<Option<crate::ssh::SshKeyInfo>> {
        self.secret_value(id, "public_key")?
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut info = crate::ssh::public_key_info(&s)?;
                info.comment = self
                    .secret_value(id, "comment")?
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                Ok(info)
            })
            .transpose()
    }
    pub fn agent_identities(&self) -> Vec<crate::ssh::AgentIdentity> {
        self.records
            .values()
            .filter(|r| {
                r.secret_kind() == Some(SecretKind::SshKey)
                    && r.deleted_at().is_none()
                    && r.agent_enabled()
            })
            .filter_map(|r| {
                let info = self.ssh_key_info(&r.id()).ok()??;
                Some(crate::ssh::AgentIdentity {
                    id: r.id(),
                    title: r.title(),
                    public_key: info.public_key,
                    fingerprint: info.fingerprint,
                })
            })
            .collect()
    }
    pub fn sign_with(&self, id: &RecordId, data: &[u8], flags: u32) -> Result<Vec<u8>> {
        let r = self.record(id)?;
        if r.secret_kind() != Some(SecretKind::SshKey)
            || !r.agent_enabled()
            || r.deleted_at().is_some()
        {
            return Err(CryptError::Validation("key unavailable to agent".into()));
        }
        let key = self
            .secret_value(id, "private_key")?
            .ok_or_else(|| CryptError::Validation("key is empty".into()))?;
        crate::ssh::sign(&key, data, flags)
    }
    pub fn totp_code(
        &self,
        id: &RecordId,
        unix_secs: u64,
    ) -> Result<Option<crate::totp::TotpCode>> {
        self.secret_value(id, "totp_seed")?
            .filter(|s| !s.trim().is_empty())
            .map(|seed| {
                Ok(crate::totp::totp_at(
                    &crate::totp::parse_totp(&seed)?,
                    unix_secs,
                ))
            })
            .transpose()
    }
}

impl Vault {
    pub fn agent_enabled(&self, id: &RecordId) -> Result<bool> {
        Ok(self.record(id)?.agent_enabled())
    }
}

mod import_keepass;
mod markdown;
pub use import_keepass::KeepassReport;
pub use markdown::{ExportReport, MarkdownImportReport};

use crate::model::meta::{Snapshot, SnapshotEntry, SnapshotInfo, SnapshotRestoreReport};
use crate::model::record::RecordVersion;

impl Vault {
    pub fn record_versions(&self, id: &RecordId) -> Result<Vec<RecordVersion>> {
        Ok(self.record(id)?.versions(self.peer_id))
    }
    /// The record as it was at `version`. Secret values are never part of a view.
    pub fn record_at(&self, id: &RecordId, version: &str) -> Result<RecordView> {
        Ok(self.view_of(&self.record(id)?.at(version)?))
    }
    /// Bring title, body, type, tags and fields back to `version` as a new "revert" version.
    /// Secret values, folder and trash state are left alone.
    pub fn revert_record(&mut self, id: &RecordId, version: &str) -> Result<()> {
        let current = self.record(id)?;
        let old = current.at(version)?;
        let type_id = match old.type_id() {
            Some(t) if self.meta.get_type(&t).is_none() => current.type_id(),
            t => t,
        };
        let (title, body, tags, fields) = (old.title(), old.body(), old.tags(), old.fields());
        self.edit_record(id, |r| {
            r.with_label("revert", |r| {
                if r.title() != title {
                    r.set_title(&title)?;
                }
                if r.body() != body {
                    r.set_body(&body)?;
                }
                if r.type_id() != type_id {
                    r.set_type(type_id)?;
                }
                for t in r.tags() {
                    if !tags.contains(&t) {
                        r.remove_tag(&t)?;
                    }
                }
                for t in &tags {
                    if !r.tags().contains(t) {
                        r.add_tag(t)?;
                    }
                }
                let now = r.fields();
                for k in now.keys().filter(|k| !fields.contains_key(*k)) {
                    r.set_field(*k, None)?;
                }
                for (k, v) in &fields {
                    if now.get(k) != Some(v) {
                        r.set_field(*k, Some(v.clone()))?;
                    }
                }
                r.touch("revert")
            })
        })
    }
    pub fn snapshots(&self) -> Vec<SnapshotInfo> {
        self.meta
            .list_snapshots()
            .iter()
            .map(Snapshot::info)
            .collect()
    }
    pub fn create_snapshot(&mut self, name: &str) -> Result<SnapshotInfo> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CryptError::Validation("snapshot name is empty".into()));
        }
        let records = self
            .records
            .values()
            .map(|r| {
                let entry = SnapshotEntry {
                    version: r.head(),
                    trashed: r.deleted_at().is_some(),
                };
                (r.id(), entry)
            })
            .collect();
        let snap = Snapshot {
            id: SnapshotId::new(),
            name: name.into(),
            created_at: now_ms(),
            records,
        };
        self.edit_meta(|m| m.put_snapshot(&snap))?;
        Ok(snap.info())
    }
    pub fn delete_snapshot(&mut self, id: &SnapshotId) -> Result<()> {
        if self.meta.get_snapshot(id).is_none() {
            return Err(CryptError::NotFound(id.to_string()));
        }
        self.edit_meta(|m| {
            m.remove_snapshot(id);
            Ok(())
        })
    }
    /// Roll every record back to the snapshot. Nothing is destroyed: reverts add versions,
    /// later non-secret records go to trash, secrets created later are kept.
    pub fn restore_snapshot(&mut self, id: &SnapshotId) -> Result<SnapshotRestoreReport> {
        let snap = self
            .meta
            .get_snapshot(id)
            .ok_or_else(|| CryptError::NotFound(id.to_string()))?;
        let mut report = SnapshotRestoreReport::default();
        for (rid, entry) in &snap.records {
            let Ok(r) = self.record(rid) else {
                report.missing += 1;
                continue;
            };
            let mut changed = false;
            let old_folder = r
                .at(&entry.version)?
                .folder()
                .filter(|f| self.folder(f).is_ok());
            if r.head() != entry.version {
                self.revert_record(rid, &entry.version)?;
                changed = true;
            }
            let r = self.record(rid)?;
            if r.folder() != old_folder && (old_folder.is_some() || r.folder().is_some()) {
                let target = old_folder;
                self.edit_record(rid, |r| r.set_folder(target))?;
                changed = true;
            }
            if self.record(rid)?.deleted_at().is_some() != entry.trashed {
                let at = entry.trashed.then(now_ms);
                self.edit_record(rid, |r| r.set_deleted_at(at))?;
                changed = true;
            }
            report.restored += changed as usize;
        }
        let later: Vec<RecordId> = self
            .records
            .values()
            .filter(|r| {
                !snap.records.contains_key(&r.id())
                    && r.secret_kind().is_none()
                    && r.deleted_at().is_none()
            })
            .map(|r| r.id())
            .collect();
        for rid in later {
            self.edit_record(&rid, |r| r.set_deleted_at(Some(now_ms())))?;
            report.trashed += 1;
        }
        Ok(report)
    }
}

pub mod backup;
