#![cfg(feature = "fs")]
use crypt_core::*;
use std::fs;
use std::path::Path;

fn project_type() -> TypeDef {
    let opt = SelectOption {
        id: OptionId::new(),
        label: "Активен".into(),
        color: None,
    };
    let field = |name: &str, kind: FieldKind| FieldDef {
        id: FieldId::new(),
        name: name.into(),
        kind,
    };
    TypeDef {
        id: TypeId::new(),
        name: "Проект".into(),
        icon: "folder".into(),
        fields: vec![
            field("Статус", FieldKind::Select { options: vec![opt] }),
            field("Бюджет", FieldKind::Number),
            field("Дедлайн", FieldKind::Date),
            field("Сайт", FieldKind::Url),
            field("Готово", FieldKind::Checkbox),
        ],
    }
}
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn find(v: &Vault, title: &str) -> RecordView {
    let id = v
        .list(&ListFilter::default())
        .into_iter()
        .find(|r| r.title == title)
        .unwrap_or_else(|| panic!("no record {title}"))
        .id;
    v.get(&id).unwrap()
}

#[test]
fn obsidian_folder_becomes_folders_notes_fields_and_links() {
    let src = tempfile::tempdir().unwrap();
    let root = src.path().join("Мой Obsidian");
    write(&root.join(".obsidian/app.json"), "{}");
    write(
        &root.join("Idea.md"),
        "---\ntags: [work, Идеи]\nstatus: draft\naliases: [x]\n---\n# Idea\nSee [[Projects/Plan|план]] and [[Missing]] #inline-tag\n![[image.png]]\n",
    );
    write(
        &root.join("Projects/Plan.md"),
        "---\ntype: Проект\nСтатус: Активен\nБюджет: 1500\nДедлайн: 2026-12-31\nсайт: https://x.y\nГотово: true\n---\nBody [[Idea#Section]]\n",
    );
    write(&root.join("Projects/image.png"), "png");
    write(&root.join("notes.txt"), "txt");

    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(dir.path(), &SeedPhrase::generate()).unwrap();
    let t = project_type();
    v.put_type(t.clone()).unwrap();
    let report = v.import_markdown_dir(&root).unwrap();
    assert_eq!(report.notes, 2);
    assert_eq!(report.folders, 2);
    assert_eq!(report.typed, 1);
    assert_eq!(report.links, 2);
    assert_eq!(report.skipped_files, 2);

    let idea = find(&v, "Idea");
    let plan = find(&v, "Plan");
    let folders = v.folders();
    let top = folders.iter().find(|f| f.name == "Мой Obsidian").unwrap();
    let sub = folders.iter().find(|f| f.name == "Projects").unwrap();
    assert_eq!(sub.parent, Some(top.id));
    assert_eq!(idea.folder, Some(top.id));
    assert_eq!(plan.folder, Some(sub.id));

    assert_eq!(idea.tags, vec!["inline-tag", "work", "идеи"]);
    assert!(
        idea.body.starts_with("---\n"),
        "unknown keys stay as frontmatter"
    );
    assert!(idea.body.contains("status: draft"));
    assert!(idea.body.contains(&format!("[[{}|план]]", plan.id)));
    assert!(idea.body.contains("[[Missing]]"));
    assert!(idea.body.contains("![[image.png]]"));

    assert_eq!(plan.type_id, Some(t.id));
    let f = |name: &str| t.fields.iter().find(|x| x.name == name).unwrap().id;
    let opt = match &t.fields[0].kind {
        FieldKind::Select { options } => options[0].id,
        _ => unreachable!(),
    };
    assert_eq!(plan.fields[&f("Статус")], FieldValue::Select(opt));
    assert_eq!(plan.fields[&f("Бюджет")], FieldValue::Number(1500.0));
    assert_eq!(
        plan.fields[&f("Дедлайн")],
        FieldValue::Date("2026-12-31".into())
    );
    assert_eq!(
        plan.fields[&f("Сайт")],
        FieldValue::Url("https://x.y".into())
    );
    assert_eq!(plan.fields[&f("Готово")], FieldValue::Checkbox(true));
    assert_eq!(plan.body, format!("Body [[{}|Idea]]\n", idea.id));
}

#[test]
fn export_writes_tree_frontmatter_links_and_hides_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let mut v = Vault::create(dir.path(), &SeedPhrase::generate()).unwrap();
    let t = project_type();
    v.put_type(t.clone()).unwrap();
    let work = v.create_folder("Работа", None).unwrap();
    let new = |title: &str| NewRecord {
        title: title.into(),
        ..Default::default()
    };
    let idea = v.create_record(new("Идея")).unwrap();
    let plan = v
        .create_record(NewRecord {
            type_id: Some(t.id),
            folder: Some(work),
            ..new("План")
        })
        .unwrap();
    v.set_field(&plan, &t.fields[1].id, Some(FieldValue::Number(1500.0)))
        .unwrap();
    v.set_field(&plan, &t.fields[4].id, Some(FieldValue::Checkbox(true)))
        .unwrap();
    v.add_tag(&plan, "срочно").unwrap();
    v.set_body(&plan, &format!("См. [[{idea}|Идея]] и [[{idea}|мысль]]"))
        .unwrap();
    v.create_record(new("a/b:c")).unwrap();
    v.create_record(new("Дубль")).unwrap();
    v.create_record(new("Дубль")).unwrap();
    let junk = v.create_record(new("Мусор")).unwrap();
    v.delete(&junk).unwrap();
    let mail = v
        .create_record(NewRecord {
            secret_kind: Some(SecretKind::Login),
            ..new("Почта")
        })
        .unwrap();
    v.set_secret_value(&mail, "password", Some("p@ss")).unwrap();

    let out = tempfile::tempdir().unwrap();
    let report = v.export_markdown(out.path(), false).unwrap();
    assert_eq!(report.notes, 5);
    assert_eq!(report.secrets, 1);
    let base = Path::new(&report.path);
    assert!(base.starts_with(out.path()));
    let plan_md = fs::read_to_string(base.join("Работа/План.md")).unwrap();
    assert!(plan_md.starts_with("---\n"));
    assert!(plan_md.contains("type: Проект"));
    assert!(plan_md.contains("Бюджет: 1500"));
    assert!(plan_md.contains("Готово: true"));
    assert!(plan_md.contains("срочно"));
    assert!(plan_md.contains("См. [[Идея]] и [[Идея|мысль]]"));
    assert!(base.join("Идея.md").exists());
    assert!(base.join("a-b-c.md").exists());
    assert!(base.join("Дубль.md").exists() && base.join("Дубль (2).md").exists());
    assert!(!base.join("Мусор.md").exists());
    let mail_md = fs::read_to_string(base.join("Секреты/Почта.md")).unwrap();
    assert!(mail_md.contains("crypt-secret: login"));
    assert!(!mail_md.contains("p@ss"));

    // A second export never overwrites the first.
    let again = v.export_markdown(out.path(), true).unwrap();
    assert_ne!(again.path, report.path);
    let mail_md = fs::read_to_string(Path::new(&again.path).join("Секреты/Почта.md")).unwrap();
    assert!(mail_md.contains("p@ss"));

    // Round trip: importing the export restores links and typed fields.
    let dir2 = tempfile::tempdir().unwrap();
    let mut w = Vault::create(dir2.path(), &SeedPhrase::generate()).unwrap();
    w.put_type(t.clone()).unwrap();
    w.import_markdown_dir(base).unwrap();
    let plan2 = find(&w, "План");
    let idea2 = find(&w, "Идея");
    assert_eq!(plan2.type_id, Some(t.id));
    assert_eq!(plan2.fields[&t.fields[1].id], FieldValue::Number(1500.0));
    assert_eq!(plan2.tags, vec!["срочно"]);
    assert_eq!(
        plan2.body.trim_end(),
        format!("См. [[{0}|Идея]] и [[{0}|мысль]]", idea2.id)
    );
}
