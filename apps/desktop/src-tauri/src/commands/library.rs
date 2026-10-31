use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crypt_core::{
    FieldId, FieldValue, Folder, FolderId, ListFilter, NewRecord, RecordId, RecordSummary, RecordView,
    RecordVersion, SearchHit, SnapshotId, SnapshotInfo, SnapshotRestoreReport, TypeDef, TypeId, Vault, ViewDef,
    ViewId,
};
use tauri::State;

/// Run `f` against the open vault; a locked vault answers with code `Locked`.
pub fn with_vault<T>(st: &AppState, f: impl FnOnce(&mut Vault) -> crypt_core::Result<T>) -> ApiResult<T> {
    let mut g = st.vault_guard();
    let v = g.as_mut().ok_or_else(|| ApiError::new("Locked", "Хранилище заблокировано"))?;
    Ok(f(v)?)
}

pub fn fetch_records(st: &AppState, ids: Vec<RecordId>) -> ApiResult<Vec<RecordView>> {
    with_vault(st, |v| Ok(ids.iter().filter_map(|id| v.get(id).ok()).collect()))
}

macro_rules! cmd {
    ($name:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty => |$v:ident| $body:expr) => {
        // async: sync commands run on the main thread, and every write encrypts + fsyncs.
        #[tauri::command]
        pub async fn $name(st: State<'_, std::sync::Arc<AppState>>, $($arg: $ty),*) -> ApiResult<$ret> {
            let result=with_vault(&st, |$v| $body);
            if result.is_ok() && matches!(stringify!($name),"set_title"|"delete_record"|"delete_records"|"purge_record"|"revert_record"|"restore_snapshot") {crate::agent::cache::refresh(&st);}
            result
        }
    };
}

cmd!(vault_name() -> String => |v| Ok(v.name()));
cmd!(rename_vault(name: String) -> () => |v| v.rename(&name));
cmd!(list_types() -> Vec<TypeDef> => |v| Ok(v.types()));
cmd!(put_type(t: TypeDef) -> () => |v| v.put_type(t));
cmd!(delete_type(id: TypeId) -> () => |v| v.delete_type(&id));
cmd!(list_folders() -> Vec<Folder> => |v| Ok(v.folders()));
cmd!(create_folder(name: String, parent: Option<FolderId>) -> FolderId => |v| v.create_folder(&name, parent));
cmd!(rename_folder(id: FolderId, name: String) -> () => |v| v.rename_folder(&id, &name));
cmd!(move_folder(id: FolderId, parent: Option<FolderId>) -> () => |v| v.move_folder(&id, parent));
cmd!(delete_folder(id: FolderId) -> () => |v| v.delete_folder(&id));
cmd!(list_views(type_id: TypeId) -> Vec<ViewDef> => |v| Ok(v.views(&type_id)));
cmd!(put_view(v: ViewDef) -> () => |vault| vault.put_view(v));
cmd!(delete_view(id: ViewId) -> () => |v| v.delete_view(&id));
cmd!(list_records(filter: ListFilter) -> Vec<RecordSummary> => |v| Ok(v.list(&filter)));
cmd!(get_record(id: RecordId) -> RecordView => |v| v.get(&id));
cmd!(create_record(new: NewRecord) -> RecordId => |v| v.create_record(new));
cmd!(set_title(id: RecordId, title: String) -> () => |v| v.set_title(&id, &title));
cmd!(set_body(id: RecordId, body: String) -> () => |v| v.set_body(&id, &body));
cmd!(set_type(id: RecordId, type_id: Option<TypeId>) -> () => |v| v.set_type(&id, type_id));
cmd!(set_folder(id: RecordId, folder: Option<FolderId>) -> () => |v| v.set_folder(&id, folder));
cmd!(add_tag(id: RecordId, tag: String) -> () => |v| v.add_tag(&id, &tag));
cmd!(remove_tag(id: RecordId, tag: String) -> () => |v| v.remove_tag(&id, &tag));
cmd!(set_field(id: RecordId, field: FieldId, value: Option<FieldValue>) -> () => |v| v.set_field(&id, &field, value));
cmd!(delete_record(id: RecordId) -> () => |v| v.delete(&id));
cmd!(delete_records(ids: Vec<RecordId>) -> (usize, usize) => |v| v.delete_many(&ids));
cmd!(restore_record(id: RecordId) -> () => |v| v.restore(&id));
cmd!(purge_record(id: RecordId) -> () => |v| v.purge(&id));
cmd!(empty_trash() -> usize => |v| v.empty_trash());
cmd!(search(query: String, limit: usize) -> Vec<SearchHit> => |v| Ok(v.search(&query, limit.min(100))));
cmd!(backlinks(id: RecordId) -> Vec<RecordSummary> => |v| Ok(v.backlinks(&id)));
cmd!(all_tags() -> Vec<(String, usize)> => |v| Ok(v.all_tags()));
cmd!(record_versions(id: RecordId) -> Vec<RecordVersion> => |v| v.record_versions(&id));
cmd!(record_at_version(id: RecordId, version: String) -> RecordView => |v| v.record_at(&id, &version));
cmd!(revert_record(id: RecordId, version: String) -> () => |v| v.revert_record(&id, &version));
cmd!(list_snapshots() -> Vec<SnapshotInfo> => |v| Ok(v.snapshots()));
cmd!(create_snapshot(name: String) -> SnapshotInfo => |v| v.create_snapshot(&name));
cmd!(delete_snapshot(id: SnapshotId) -> () => |v| v.delete_snapshot(&id));
cmd!(restore_snapshot(id: SnapshotId) -> SnapshotRestoreReport => |v| v.restore_snapshot(&id));

#[tauri::command]
pub fn starter_types() -> Vec<TypeDef> {
    crypt_core::starter_types()
}

#[tauri::command]
pub async fn get_records(st: State<'_, std::sync::Arc<AppState>>, ids: Vec<RecordId>) -> ApiResult<Vec<RecordView>> {
    fetch_records(&st, ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::lifecycle::{create_vault, generate_seed, lock};
    use crate::state::AppState;

    fn state() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::new(
            dir.path().join("vault"),
            dir.path().join("settings.json"),
            Box::new(crate::unlock::keystore::MemoryKeyStore::default()),
            Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()),
        );
        create_vault(&st, generate_seed()).unwrap();
        (dir, st)
    }
    fn new(title: &str) -> NewRecord {
        NewRecord { title: title.into(), ..Default::default() }
    }

    #[test]
    fn locked_vault_returns_locked_code() {
        let (_d, st) = state();
        lock(&st);
        assert_eq!(with_vault(&st, |v| Ok(v.types())).unwrap_err().code, "Locked");
    }

    #[test]
    fn record_crud_through_commands() {
        let (_d, st) = state();
        let id = with_vault(&st, |v| v.create_record(new("Заметка"))).unwrap();
        with_vault(&st, |v| v.set_body(&id, "текст")).unwrap();
        with_vault(&st, |v| v.add_tag(&id, "#Важное")).unwrap();
        let r = with_vault(&st, |v| v.get(&id)).unwrap();
        assert_eq!((r.title.as_str(), r.body.as_str()), ("Заметка", "текст"));
        assert_eq!(r.tags, vec!["важное"]);
        with_vault(&st, |v| v.delete(&id)).unwrap();
        let trashed = with_vault(&st, |v| Ok(v.list(&ListFilter { trashed: true, ..Default::default() }))).unwrap();
        assert_eq!(trashed.len(), 1);
        with_vault(&st, |v| v.restore(&id)).unwrap();
        assert_eq!(with_vault(&st, |v| Ok(v.list(&ListFilter::default()))).unwrap().len(), 1);
    }

    #[test]
    fn get_records_skips_missing() {
        let (_d, st) = state();
        let id = with_vault(&st, |v| v.create_record(new("a"))).unwrap();
        let out = fetch_records(&st, vec![id, RecordId::new()]).unwrap();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn history_wire_format_matches_frontend() {
        let (_d, st) = state();
        let id = with_vault(&st, |v| v.create_record(new("a"))).unwrap();
        let versions = with_vault(&st, |v| v.record_versions(&id)).unwrap();
        let json = serde_json::to_value(&versions[0]).unwrap();
        for key in ["id", "at", "kind", "this_device"] {
            assert!(json.get(key).is_some(), "{key}");
        }
        let snap = with_vault(&st, |v| v.create_snapshot("s")).unwrap();
        let json = serde_json::to_value(&snap).unwrap();
        for key in ["id", "name", "created_at", "records"] {
            assert!(json.get(key).is_some(), "{key}");
        }
        let report = with_vault(&st, |v| v.restore_snapshot(&snap.id)).unwrap();
        let json = serde_json::to_value(&report).unwrap();
        for key in ["restored", "trashed", "missing"] {
            assert!(json.get(key).is_some(), "{key}");
        }
    }
}
