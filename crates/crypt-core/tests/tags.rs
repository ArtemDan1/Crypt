use crypt_core::{ListFilter, NewRecord, SeedPhrase, Vault};

#[test]
fn all_tags_counts_and_excludes_trash() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(&dir.path().join("v"), &SeedPhrase::generate()).unwrap();
    let mk = |v: &mut Vault, tags: &[&str]| {
        let id = v
            .create_record(NewRecord {
                title: "x".into(),
                ..Default::default()
            })
            .unwrap();
        for t in tags {
            v.add_tag(&id, t).unwrap();
        }
        id
    };
    mk(&mut v, &["a", "b"]);
    mk(&mut v, &["a"]);
    assert_eq!(
        v.all_tags(),
        vec![("a".to_string(), 2), ("b".to_string(), 1)]
    );
    let b_only = mk(&mut v, &["b", "c"]);
    v.delete(&b_only).unwrap();
    assert_eq!(
        v.all_tags(),
        vec![("a".to_string(), 2), ("b".to_string(), 1)]
    );
}

#[test]
fn list_filter_deserializes_from_camel_case() {
    let f: ListFilter = serde_json::from_str(r#"{"tag":"a","trashed":true}"#).unwrap();
    assert_eq!(f.tag.as_deref(), Some("a"));
    assert!(f.trashed);
    let empty: ListFilter = serde_json::from_str("{}").unwrap();
    assert!(!empty.trashed && empty.type_id.is_none());
}
