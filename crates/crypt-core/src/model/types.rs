use super::ids::*;
use crate::{CryptError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum FieldKind {
    Text,
    Number,
    Date,
    Url,
    Checkbox,
    Select {
        options: Vec<SelectOption>,
    },
    MultiSelect {
        options: Vec<SelectOption>,
    },
    RecordLink {
        target_type: Option<TypeId>,
        multiple: bool,
    },
    SecretLink {
        multiple: bool,
    },
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SelectOption {
    pub id: OptionId,
    pub label: String,
    pub color: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FieldDef {
    pub id: FieldId,
    pub name: String,
    pub kind: FieldKind,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TypeDef {
    pub id: TypeId,
    pub name: String,
    pub icon: String,
    pub fields: Vec<FieldDef>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum FieldValue {
    Text(String),
    Number(f64),
    Date(String),
    Url(String),
    Checkbox(bool),
    Select(OptionId),
    MultiSelect(Vec<OptionId>),
    Links(Vec<RecordId>),
}
impl FieldValue {
    pub fn fits(&self, kind: &FieldKind) -> bool {
        match (self, kind) {
            (Self::Text(_), FieldKind::Text)
            | (Self::Url(_), FieldKind::Url)
            | (Self::Checkbox(_), FieldKind::Checkbox) => true,
            (Self::Number(n), FieldKind::Number) => n.is_finite(),
            (Self::Date(date), FieldKind::Date) => valid_date(date),
            (Self::Select(id), FieldKind::Select { options }) => {
                options.iter().any(|o| o.id == *id)
            }
            (Self::MultiSelect(ids), FieldKind::MultiSelect { options }) => {
                ids.iter().all(|id| options.iter().any(|o| o.id == *id))
            }
            (
                Self::Links(ids),
                FieldKind::RecordLink { multiple, .. } | FieldKind::SecretLink { multiple },
            ) => *multiple || ids.len() <= 1,
            _ => false,
        }
    }
}
fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = s[..4].parse().unwrap_or(0);
    let month: usize = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..].parse().unwrap_or(0);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        0,
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && (1..=12).contains(&month) && day > 0 && day <= days[month]
}
impl TypeDef {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(CryptError::Validation("type name is empty".into()));
        }
        let mut ids = HashSet::new();
        for f in &self.fields {
            if f.name.trim().is_empty() || !ids.insert(f.id) {
                return Err(CryptError::Validation(
                    "empty field name or duplicate field id".into(),
                ));
            }
            if let FieldKind::Select { options } | FieldKind::MultiSelect { options } = &f.kind {
                let mut options_seen = HashSet::new();
                if options.iter().any(|o| !options_seen.insert(o.id)) {
                    return Err(CryptError::Validation("duplicate option id".into()));
                }
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Folder {
    pub id: FolderId,
    pub name: String,
    pub parent: Option<FolderId>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ViewDef {
    pub id: ViewId,
    pub type_id: TypeId,
    pub name: String,
    pub config: serde_json::Value,
}

pub fn starter_types() -> Vec<TypeDef> {
    let fields = [
        ("Hostname", FieldKind::Text),
        ("IP", FieldKind::Text),
        ("ОС", FieldKind::Text),
        ("Провайдер", FieldKind::Text),
        ("SSH-порт", FieldKind::Number),
        ("SSH-пользователь", FieldKind::Text),
        ("Креды", FieldKind::SecretLink { multiple: true }),
    ]
    .into_iter()
    .map(|(name, kind)| FieldDef {
        id: FieldId::new(),
        name: name.into(),
        kind,
    })
    .collect();
    vec![TypeDef {
        id: TypeId::new(),
        name: "Сервер".into(),
        icon: "server".into(),
        fields,
    }]
}
