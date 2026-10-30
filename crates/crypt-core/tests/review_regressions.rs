#![cfg(feature = "fs")]
use crypt_core::*;
use crypt_core::{
    crypto::envelope::{open, seal},
    model::record::RecordDoc,
    storage::layout::Aad,
};
#[test]
fn tombstoned_secret_temp_is_cleaned_without_object_file() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let id = v
        .create_record(NewRecord {
            title: "secret".into(),
            secret_kind: Some(SecretKind::Login),
            ..Default::default()
        })
        .unwrap();
    v.set_secret_value(&id, "password", Some("old-secret"))
        .unwrap();
    let bytes = std::fs::read(dir.path().join(format!("objects/{id}.bin"))).unwrap();
    v.delete(&id).unwrap();
    v.lock();
    // Simulate the crash window: tombstone persisted, .bin removed, stale temp remains.
    let tmp = dir.path().join(format!("objects/{id}.bin.123.tmp"));
    std::fs::write(&tmp, bytes).unwrap();
    let v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert!(v.get(&id).is_err());
    assert!(!tmp.exists());
}
#[test]
fn malformed_secret_kind_is_quarantined_not_downgraded() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let id = v
        .create_record(NewRecord {
            title: "secret".into(),
            secret_kind: Some(SecretKind::Login),
            ..Default::default()
        })
        .unwrap();
    let key = v.data_key().clone();
    v.lock();
    let file = dir.path().join(format!("objects/{id}.bin"));
    let plain = open(
        &key,
        &Aad::Object(id).bytes(),
        &std::fs::read(&file).unwrap(),
    )
    .unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&plain).unwrap();
    doc.get_map("meta")
        .insert("secret_kind", "not-json-secret")
        .unwrap();
    doc.commit();
    let snapshot = doc.export(loro::ExportMode::Snapshot).unwrap();
    std::fs::write(file, seal(&key, &Aad::Object(id).bytes(), &snapshot)).unwrap();
    let v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert!(v.get(&id).is_err());
    assert_eq!(v.load_errors().len(), 1);
}
#[test]
fn record_doc_rejects_nonfinite_numbers_before_mutation() {
    let mut r = RecordDoc::new(1, RecordId::new(), "record", 1);
    let id = FieldId::new();
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            r.set_field(id, Some(FieldValue::Number(n))),
            Err(CryptError::Validation(_))
        ));
    }
    assert!(r.field(&id).is_none());
    assert!(RecordDoc::import(2, &r.export()).is_ok());
}

#[test]
fn atomic_write_accepts_a_relative_filename() {
    if std::env::var_os("CRYPT_CORE_RELATIVE_WRITE_CHILD").is_some() {
        crypt_core::storage::fs::write_atomic(std::path::Path::new("relative.bin"), b"complete")
            .unwrap();
        assert_eq!(std::fs::read("relative.bin").unwrap(), b"complete");
        return;
    }
    // Isolate cwd in a subprocess; never change the test runner's process-wide cwd.
    let dir = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "atomic_write_accepts_a_relative_filename",
            "--nocapture",
        ])
        .current_dir(dir.path())
        .env("CRYPT_CORE_RELATIVE_WRITE_CHILD", "1")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
}
