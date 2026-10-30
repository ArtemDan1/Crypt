#![cfg(feature = "fs")]
use crypt_core::model::record::RecordDoc;
use crypt_core::*;
fn new(title: &str, type_id: Option<TypeId>, folder: Option<FolderId>) -> NewRecord {
    NewRecord {
        title: title.into(),
        type_id,
        folder,
        secret_kind: None,
    }
}
#[test]
fn record_doc_roundtrip() {
    let id = RecordId::new();
    let field = FieldId::new();
    let mut r = RecordDoc::new(1, id, "title", 100);
    r.set_body("Кириллица 😀").unwrap();
    r.add_tag(" #Подписки ").unwrap();
    assert!(r.add_tag("## ").is_err());
    r.set_field(field, Some(FieldValue::Number(399.0))).unwrap();
    let r = RecordDoc::import(2, &r.export()).unwrap();
    assert_eq!(r.id(), id);
    assert_eq!(r.body(), "Кириллица 😀");
    assert_eq!(r.tags(), vec!["подписки"]);
    assert_eq!(r.field(&field), Some(FieldValue::Number(399.0)));
    assert!(r.updated_at() > r.created_at());
}
#[test]
fn records_roundtrip_filters_validation_and_hidden_fields() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let mut t = starter_types().remove(0);
    let port = t.fields[4].id;
    v.put_type(t.clone()).unwrap();
    let folder = v.create_folder("Серверы", None).unwrap();
    let id = v
        .create_record(new("Яндекс 360", Some(t.id), Some(folder)))
        .unwrap();
    let note = v.create_record(new("note", None, None)).unwrap();
    v.set_body(&id, "Семейная подписка").unwrap();
    v.add_tag(&id, " #Подписки ").unwrap();
    v.set_field(&id, &port, Some(FieldValue::Number(399.0)))
        .unwrap();
    assert!(matches!(
        v.set_field(&id, &port, Some(FieldValue::Text("x".into()))),
        Err(CryptError::Validation(_))
    ));
    assert!(
        v.set_field(&id, &FieldId::new(), Some(FieldValue::Number(1.0)))
            .is_err()
    );
    assert!(
        v.set_field(&note, &port, Some(FieldValue::Number(1.0)))
            .is_err()
    );
    assert!(v.delete_type(&t.id).is_err());
    assert!(v.delete_folder(&folder).is_err());
    for filter in [
        ListFilter {
            type_id: Some(t.id),
            ..Default::default()
        },
        ListFilter {
            folder: Some(folder),
            ..Default::default()
        },
        ListFilter {
            tag: Some("#ПОДПИСКИ".into()),
            ..Default::default()
        },
    ] {
        assert_eq!(
            v.list(&filter).iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![id]
        );
    }
    t.fields[4].kind = FieldKind::Text;
    v.put_type(t.clone()).unwrap();
    let view = v.get(&id).unwrap();
    assert!(!view.fields.contains_key(&port));
    assert_eq!(view.hidden_fields[&port], FieldValue::Number(399.0));
    let field = t.fields.remove(4);
    v.put_type(t.clone()).unwrap();
    assert!(v.get(&id).unwrap().hidden_fields.contains_key(&port));
    t.fields.push(FieldDef {
        kind: FieldKind::Number,
        ..field
    });
    v.put_type(t).unwrap();
    v.lock();
    let v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    let view = v.get(&id).unwrap();
    assert_eq!(view.body, "Семейная подписка");
    assert_eq!(view.tags, vec!["подписки"]);
    assert_eq!(view.fields[&port], FieldValue::Number(399.0));
    let bytes = std::fs::read(dir.path().join(format!("objects/{id}.bin"))).unwrap();
    assert!(
        !bytes
            .windows(view.body.len())
            .any(|b| b == view.body.as_bytes())
    );
}
#[test]
fn corrupt_object_does_not_block_vault_and_tmp_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let a = v.create_record(new("bad", None, None)).unwrap();
    let b = v.create_record(new("good", None, None)).unwrap();
    v.lock();
    std::fs::write(dir.path().join(format!("objects/{a}.bin")), b"bad").unwrap();
    std::fs::write(dir.path().join("objects/junk.123.tmp"), b"junk").unwrap();
    let v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert_eq!(v.load_errors().len(), 1);
    assert_eq!(v.get(&b).unwrap().title, "good");
    assert!(v.get(&a).is_err());
}
