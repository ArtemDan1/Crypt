use crypt_core::model::meta::MetaDoc;
use crypt_core::*;
#[test]
fn field_values_validate_all_kinds() {
    let a = OptionId::new();
    let b = OptionId::new();
    let options = vec![SelectOption {
        id: a,
        label: "A".into(),
        color: None,
    }];
    assert!(FieldValue::Text("x".into()).fits(&FieldKind::Text));
    assert!(FieldValue::Url("https://example.org".into()).fits(&FieldKind::Url));
    assert!(FieldValue::Checkbox(true).fits(&FieldKind::Checkbox));
    assert!(FieldValue::Number(2.0).fits(&FieldKind::Number));
    assert!(!FieldValue::Number(f64::NAN).fits(&FieldKind::Number));
    assert!(!FieldValue::Number(f64::INFINITY).fits(&FieldKind::Number));
    for (s, valid) in [
        ("2026-10-28", true),
        ("2024-02-29", true),
        ("2026-02-29", false),
        ("2026-13-40", false),
        ("14.11.2026", false),
        ("2026-1-01", false),
        ("2026-04-31", false),
    ] {
        assert_eq!(
            FieldValue::Date(s.into()).fits(&FieldKind::Date),
            valid,
            "{s}"
        );
    }
    assert!(FieldValue::Select(a).fits(&FieldKind::Select {
        options: options.clone()
    }));
    assert!(!FieldValue::Select(b).fits(&FieldKind::Select {
        options: options.clone()
    }));
    assert!(
        FieldValue::MultiSelect(vec![a]).fits(&FieldKind::MultiSelect {
            options: options.clone()
        })
    );
    assert!(!FieldValue::MultiSelect(vec![a, b]).fits(&FieldKind::MultiSelect { options }));
    let ids = vec![RecordId::new(), RecordId::new()];
    assert!(FieldValue::Links(ids.clone()).fits(&FieldKind::RecordLink {
        target_type: None,
        multiple: true
    }));
    assert!(!FieldValue::Links(ids.clone()).fits(&FieldKind::SecretLink { multiple: false }));
    assert!(!FieldValue::Links(ids).fits(&FieldKind::Text));
}
#[test]
fn put_type_validates() {
    let mut m = MetaDoc::new(1);
    let mut t = starter_types().remove(0);
    t.name = " ".into();
    assert!(matches!(
        m.put_type(t.clone()),
        Err(CryptError::Validation(_))
    ));
    t.name = "Test".into();
    t.fields.push(t.fields[0].clone());
    assert!(matches!(m.put_type(t), Err(CryptError::Validation(_))));
    let a = OptionId::new();
    let t = TypeDef {
        id: TypeId::new(),
        name: "X".into(),
        icon: "".into(),
        fields: vec![FieldDef {
            id: FieldId::new(),
            name: "X".into(),
            kind: FieldKind::Select {
                options: vec![
                    SelectOption {
                        id: a,
                        label: "A".into(),
                        color: None,
                    },
                    SelectOption {
                        id: a,
                        label: "B".into(),
                        color: None,
                    },
                ],
            },
        }],
    };
    assert!(matches!(m.put_type(t), Err(CryptError::Validation(_))));
}
#[test]
fn meta_roundtrip_and_tombstones() {
    let mut m = MetaDoc::new(1);
    let t = starter_types().remove(0);
    m.put_type(t.clone()).unwrap();
    let f = Folder {
        id: FolderId::new(),
        name: "Folder".into(),
        parent: None,
    };
    m.put_folder(f.clone()).unwrap();
    let view = ViewDef {
        id: ViewId::new(),
        type_id: t.id,
        name: "Table".into(),
        config: serde_json::json!({"sort":[["price","asc"]]}),
    };
    m.put_view(view.clone()).unwrap();
    let id = RecordId::new();
    m.add_tombstone(&id, 42);
    let loaded = MetaDoc::import(2, &m.export()).unwrap();
    assert_eq!(loaded.get_type(&t.id), Some(t.clone()));
    assert_eq!(loaded.list_folders(), vec![f]);
    assert_eq!(loaded.list_views(&t.id), vec![view]);
    assert_eq!(loaded.tombstones(), vec![(id, 42)]);
    assert_eq!(t.fields.len(), 7);
}
#[cfg(feature = "fs")]
#[test]
fn metadata_persists_and_folder_cycles_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let seed = SeedPhrase::generate();
    let mut v = Vault::create(dir.path(), &seed).unwrap();
    let t = starter_types().remove(0);
    v.put_type(t.clone()).unwrap();
    let a = v.create_folder("A", None).unwrap();
    let b = v.create_folder("B", Some(a)).unwrap();
    assert!(matches!(
        v.move_folder(&a, Some(b)),
        Err(CryptError::Validation(_))
    ));
    assert!(matches!(
        v.delete_folder(&a),
        Err(CryptError::Validation(_))
    ));
    v.rename_folder(&b, "Child").unwrap();
    let view = ViewDef {
        id: ViewId::new(),
        type_id: t.id,
        name: "Table".into(),
        config: serde_json::json!({"sort":[["price","asc"]]}),
    };
    v.put_view(view.clone()).unwrap();
    v.lock();
    let v = Vault::open_with_seed(dir.path(), &seed).unwrap();
    assert_eq!(v.types(), vec![t.clone()]);
    assert_eq!(v.views(&t.id), vec![view]);
    assert_eq!(
        v.folders().iter().find(|f| f.id == b).unwrap().name,
        "Child"
    );
    let bytes = std::fs::read(dir.path().join("meta.bin")).unwrap();
    assert!(
        !bytes
            .windows("Сервер".len())
            .any(|b| b == "Сервер".as_bytes())
    );
}
