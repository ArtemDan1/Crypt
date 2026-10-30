#![cfg(feature = "fs")]
use crypt_core::*;
use crypt_core::{crypto::envelope::open, model::meta::MetaDoc, storage::layout::Aad};
fn create(v: &mut Vault, title: &str, kind: Option<SecretKind>) -> RecordId {
    v.create_record(NewRecord {
        title: title.into(),
        secret_kind: kind,
        ..Default::default()
    })
    .unwrap()
}
#[test]
fn secret_values_roundtrip_history_validation_and_redaction() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let id = create(&mut v, "Login", Some(SecretKind::Login));
    let plain = create(&mut v, "Note", None);
    v.set_secret_value(&id, "password", Some("old1")).unwrap();
    v.set_secret_value(&id, "password", Some("new2")).unwrap();
    assert!(v.set_secret_value(&id, "cvv", Some("bad")).is_err());
    assert!(v.secret_value(&id, "cvv").is_err());
    assert!(v.secret_history(&id, "cvv").is_err());
    assert!(v.set_secret_value(&plain, "password", Some("bad")).is_err());
    let serialized = serde_json::to_string(&v.get(&id).unwrap()).unwrap();
    assert!(!serialized.contains("new2"));
    assert!(!serialized.contains("old1"));
    v.lock();
    let mut v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert_eq!(&*v.secret_value(&id, "password").unwrap().unwrap(), "new2");
    let history = v.secret_history(&id, "password").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(&*history[0].value, "old1");
    assert!(!format!("{:?}", history[0]).contains("old1"));
    v.set_secret_value(&id, "password", Some("new2")).unwrap();
    assert_eq!(v.secret_history(&id, "password").unwrap().len(), 1);
    v.set_secret_value(&id, "password", None).unwrap();
    assert!(v.secret_value(&id, "password").unwrap().is_none());
    assert_eq!(
        &*v.secret_history(&id, "password").unwrap()[0].value,
        "new2"
    );
}
#[test]
fn delete_secret_purges_file_history_and_adds_tombstone() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let id = create(&mut v, "Login", Some(SecretKind::Login));
    v.set_secret_value(&id, "password", Some("old1")).unwrap();
    v.set_secret_value(&id, "password", Some("new2")).unwrap();
    let object = dir.path().join(format!("objects/{id}.bin"));
    let obsolete = std::fs::read(&object).unwrap();
    let tmp = dir.path().join(format!("objects/{id}.bin.123.tmp"));
    std::fs::write(&tmp, &obsolete).unwrap();
    v.delete(&id).unwrap();
    assert!(!object.exists());
    assert!(!tmp.exists());
    assert!(matches!(v.get(&id), Err(CryptError::NotFound(_))));
    assert!(
        v.list(&ListFilter {
            secrets: Some(true),
            ..Default::default()
        })
        .is_empty()
    );
    let meta = open(
        v.data_key(),
        &Aad::Meta.bytes(),
        &std::fs::read(dir.path().join("meta.bin")).unwrap(),
    )
    .unwrap();
    assert!(
        MetaDoc::import(1, &meta)
            .unwrap()
            .tombstones()
            .iter()
            .any(|(rid, _)| *rid == id)
    );
    // A stale copy must not resurrect a purged document.
    std::fs::write(&object, obsolete).unwrap();
    v.lock();
    let v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert!(v.get(&id).is_err());
    assert!(!object.exists());
}
#[test]
fn trash_restore_empty_and_type_in_use() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let t = starter_types().remove(0);
    v.put_type(t.clone()).unwrap();
    let a = v
        .create_record(NewRecord {
            title: "a".into(),
            type_id: Some(t.id),
            ..Default::default()
        })
        .unwrap();
    let b = create(&mut v, "b", None);
    v.delete(&a).unwrap();
    assert!(v.delete_type(&t.id).is_err());
    assert_eq!(
        v.list(&ListFilter {
            trashed: true,
            ..Default::default()
        })[0]
            .id,
        a
    );
    v.restore(&a).unwrap();
    assert_eq!(v.list(&ListFilter::default()).len(), 2);
    v.delete(&a).unwrap();
    v.delete(&b).unwrap();
    assert_eq!(v.empty_trash().unwrap(), 2);
    for id in [a, b] {
        assert!(!dir.path().join(format!("objects/{id}.bin")).exists());
    }
}
#[test]
fn purged_links_are_filtered_and_agent_flag_is_ssh_only() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let secret = create(&mut v, "Login", Some(SecretKind::Login));
    let ssh = create(&mut v, "SSH", Some(SecretKind::SshKey));
    assert!(v.set_agent_enabled(&secret, true).is_err());
    assert!(v.agent_keys().iter().any(|(id, _)| *id == ssh));
    v.set_agent_enabled(&ssh, false).unwrap();
    assert!(v.agent_keys().is_empty());
    v.set_agent_enabled(&ssh, true).unwrap();
    assert_eq!(v.agent_keys(), vec![(ssh, "SSH".into())]);
    let t = starter_types().remove(0);
    let field = t.fields[6].id;
    v.put_type(t.clone()).unwrap();
    let id = v
        .create_record(NewRecord {
            title: "Server".into(),
            type_id: Some(t.id),
            ..Default::default()
        })
        .unwrap();
    v.set_field(&id, &field, Some(FieldValue::Links(vec![secret])))
        .unwrap();
    v.delete(&secret).unwrap();
    assert_eq!(
        v.get(&id).unwrap().fields[&field],
        FieldValue::Links(vec![])
    );
}
