//! Markdown folders: Obsidian-style import and .md export.
use super::Vault;
use crate::model::record::{NewRecord, now_ms};
use crate::{CryptError, FieldKind, FieldValue, FolderId, RecordId, Result, TypeDef, TypeId};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader, yaml::Hash};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MarkdownImportReport {
    pub notes: usize,
    pub folders: usize,
    /// Notes that got a type from their frontmatter.
    pub typed: usize,
    /// Obsidian links turned into record links.
    pub links: usize,
    /// Non-markdown, unreadable or oversized files.
    pub skipped_files: usize,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ExportReport {
    /// The new folder holding the export.
    pub path: String,
    pub notes: usize,
    pub secrets: usize,
}

const MAX_FILE: u64 = 10 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const SECRETS_DIR: &str = "Секреты";

static OBSIDIAN_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(!?)\[\[([^\[\]|\n]+?)(?:\|([^\]\n]*))?\]\]").expect("valid regex")
});
static CRYPT_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[([0-7][0-9A-HJKMNP-TV-Z]{25})(?:\|([^\]\n]*))?\]\]").expect("valid regex")
});
static INLINE_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|\s)#([\p{L}\p{N}_/-]*[\p{L}_/-][\p{L}\p{N}_/-]*)").expect("valid regex")
});

struct Note {
    dirs: Vec<String>,
    stem: String,
    content: String,
}

/// Case-insensitive for any script (Cyrillic field names included).
fn same(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}
fn hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn walk(
    dir: &Path,
    dirs: &mut Vec<String>,
    out: &mut Vec<Note>,
    report: &mut MarkdownImportReport,
) -> Result<()> {
    if dirs.len() > MAX_DEPTH {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<std::io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if hidden(&name) {
            continue;
        }
        // Never follow links out of the chosen folder.
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.is_dir() {
            dirs.push(name);
            walk(&entry.path(), dirs, out, report)?;
            dirs.pop();
            continue;
        }
        let path = entry.path();
        let is_md = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if !meta.is_file() || !is_md || meta.len() > MAX_FILE {
            report.skipped_files += 1;
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(content) => out.push(Note {
                dirs: dirs.clone(),
                stem: path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                content,
            }),
            Err(_) => report.skipped_files += 1,
        }
    }
    Ok(())
}

/// Split a leading `---` YAML block; malformed frontmatter stays part of the body.
fn split_frontmatter(content: &str) -> (Option<Hash>, &str) {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return (None, content);
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" || trimmed == "..." {
            let yaml = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return match YamlLoader::load_from_str(yaml) {
                Ok(docs) => match docs.into_iter().next() {
                    Some(Yaml::Hash(h)) => (Some(h), body),
                    None => (Some(Hash::new()), body),
                    _ => (None, content),
                },
                Err(_) => (None, content),
            };
        }
        offset += line.len();
    }
    (None, content)
}

fn yaml_text(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(n) => Some(n.to_string()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}
fn yaml_list(y: &Yaml) -> Vec<String> {
    match y {
        Yaml::Array(a) => a.iter().filter_map(yaml_text).collect(),
        Yaml::String(s) => s
            .split([',', ' '])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        other => yaml_text(other).into_iter().collect(),
    }
}
fn take_key(h: &mut Hash, name: &str) -> Option<Yaml> {
    let key = h
        .keys()
        .find(|k| yaml_text(k).is_some_and(|k| same(&k, name)))?
        .clone();
    h.remove(&key)
}

/// Lowercased lookup keys for a note: its stem and its path inside the imported folder.
fn note_keys(dirs: &[String], stem: &str) -> [String; 2] {
    let mut path = dirs.join("/");
    if !path.is_empty() {
        path.push('/');
    }
    path.push_str(stem);
    [stem.to_lowercase(), path.to_lowercase()]
}

fn link_target(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = raw.split(['#', '^']).next().unwrap_or(raw).trim();
    raw.strip_suffix(".md").unwrap_or(raw)
}

fn convert_value(
    y: &Yaml,
    kind: &FieldKind,
    resolve: &dyn Fn(&str) -> Option<RecordId>,
) -> Option<FieldValue> {
    let value = match kind {
        FieldKind::Text => FieldValue::Text(yaml_text(y)?),
        FieldKind::Url => FieldValue::Url(yaml_text(y)?),
        FieldKind::Number => FieldValue::Number(match y {
            Yaml::Integer(n) => *n as f64,
            Yaml::Real(s) => s.parse().ok()?,
            Yaml::String(s) => s
                .replace([' ', '\u{a0}'], "")
                .replace(',', ".")
                .parse()
                .ok()?,
            _ => return None,
        }),
        FieldKind::Date => {
            let s = yaml_text(y)?;
            FieldValue::Date(s.get(..10).unwrap_or(&s).to_string())
        }
        FieldKind::Checkbox => FieldValue::Checkbox(match y {
            Yaml::Boolean(b) => *b,
            Yaml::String(s) => match s.trim().to_lowercase().as_str() {
                "true" | "yes" | "да" | "1" => true,
                "false" | "no" | "нет" | "0" => false,
                _ => return None,
            },
            _ => return None,
        }),
        FieldKind::Select { options } => {
            let label = yaml_text(y)?;
            let o = options.iter().find(|o| same(&o.label, &label))?;
            FieldValue::Select(o.id)
        }
        FieldKind::MultiSelect { options } => {
            let ids: Option<Vec<_>> = yaml_list(y)
                .iter()
                .map(|l| options.iter().find(|o| same(&o.label, l)).map(|o| o.id))
                .collect();
            FieldValue::MultiSelect(ids?)
        }
        FieldKind::RecordLink { .. } | FieldKind::SecretLink { .. } => {
            let items = match y {
                Yaml::Array(a) => a.iter().filter_map(yaml_text).collect(),
                other => vec![yaml_text(other)?],
            };
            let ids: Option<Vec<_>> = items
                .iter()
                .map(|s| {
                    let inner = s.trim().trim_start_matches("[[").trim_end_matches("]]");
                    resolve(link_target(inner.split('|').next().unwrap_or(inner)))
                })
                .collect();
            FieldValue::Links(ids?)
        }
    };
    value.fits(kind).then_some(value)
}

fn emit_yaml(h: Hash) -> String {
    let mut out = String::new();
    let doc = Yaml::Hash(h);
    if YamlEmitter::new(&mut out).dump(&doc).is_err() {
        return String::new();
    }
    let body = out.strip_prefix("---\n").unwrap_or(&out);
    format!("---\n{}\n---\n", body.trim_end())
}

fn inline_tags(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            out.extend(INLINE_TAG.captures_iter(line).map(|c| c[1].to_string()));
        }
    }
    out
}

/// A safe file or folder name: no separators, no leading dot, never empty.
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim_start_matches('.').trim();
    let cleaned: String = cleaned.chars().take(120).collect();
    if cleaned.is_empty() {
        "Без названия".into()
    } else {
        cleaned
    }
}
fn unique(used: &mut HashSet<String>, name: &str) -> String {
    let mut candidate = name.to_string();
    let mut n = 2;
    while !used.insert(candidate.to_lowercase()) {
        candidate = format!("{name} ({n})");
        n += 1;
    }
    candidate
}
fn today() -> String {
    // Civil date from Unix days (Howard Hinnant's algorithm), UTC.
    let days = now_ms().div_euclid(86_400_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
fn number_yaml(n: f64) -> Yaml {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        Yaml::Integer(n as i64)
    } else {
        Yaml::Real(n.to_string())
    }
}

impl Vault {
    /// Import an Obsidian-style folder: subfolders become folders under a new top folder
    /// named after it, frontmatter fills tags and the fields of a matching type, and
    /// `[[Note]]` links become record links. Files are read before anything is written.
    pub fn import_markdown_dir(&mut self, root: &Path) -> Result<MarkdownImportReport> {
        if !fs::metadata(root).map(|m| m.is_dir()).unwrap_or(false) {
            return Err(CryptError::ImportFormat("Папка не найдена".into()));
        }
        let mut report = MarkdownImportReport::default();
        let mut notes = Vec::new();
        walk(root, &mut Vec::new(), &mut notes, &mut report)?;
        let types = self.meta.list_types();

        // Ids first, so links between notes resolve in a single pass.
        let ids: Vec<RecordId> = notes.iter().map(|_| RecordId::new()).collect();
        let mut by_name: HashMap<String, RecordId> = HashMap::new();
        for (n, id) in notes.iter().zip(&ids) {
            for key in note_keys(&n.dirs, &n.stem) {
                by_name.entry(key).or_insert(*id);
            }
        }
        let resolve = |target: &str| by_name.get(&target.trim().to_lowercase()).copied();

        let top_name = root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Импорт".into());
        let top = self.create_folder(&top_name, None)?;
        report.folders += 1;
        let mut folders: HashMap<Vec<String>, FolderId> = HashMap::from([(vec![], top)]);

        for (note, id) in notes.iter().zip(ids) {
            let mut parent = top;
            for depth in 1..=note.dirs.len() {
                let path = note.dirs[..depth].to_vec();
                parent = match folders.get(&path) {
                    Some(f) => *f,
                    None => {
                        let f = self.create_folder(&note.dirs[depth - 1], Some(parent))?;
                        report.folders += 1;
                        folders.insert(path, f);
                        f
                    }
                };
            }

            let (front, rest) = split_frontmatter(&note.content);
            // The blank line that conventionally follows frontmatter is not content.
            let rest = match front {
                Some(_) => rest
                    .strip_prefix("\r\n")
                    .or_else(|| rest.strip_prefix('\n'))
                    .unwrap_or(rest),
                None => rest,
            };
            let mut front = front.unwrap_or_default();
            let title = take_key(&mut front, "title")
                .and_then(|y| yaml_text(&y))
                .filter(|t| !t.trim().is_empty())
                .unwrap_or_else(|| note.stem.clone());
            let mut tags: Vec<String> = ["tags", "tag"]
                .iter()
                .filter_map(|k| take_key(&mut front, k))
                .flat_map(|y| yaml_list(&y))
                .map(|t| t.trim_start_matches('#').to_string())
                .collect();
            tags.extend(inline_tags(rest));

            let named = front
                .iter()
                .find(|(k, _)| yaml_text(k).is_some_and(|k| same(&k, "type")))
                .and_then(|(_, v)| yaml_text(v))
                .and_then(|name| types.iter().find(|t| same(&t.name, &name)));
            let t: Option<&TypeDef> = named.or_else(|| {
                // Without an explicit type, require at least two matching field names.
                types
                    .iter()
                    .map(|t| {
                        let hits = t
                            .fields
                            .iter()
                            .filter(|f| {
                                front
                                    .keys()
                                    .any(|k| yaml_text(k).is_some_and(|k| same(&k, &f.name)))
                            })
                            .count();
                        (hits, t)
                    })
                    .filter(|(hits, _)| *hits >= 2)
                    .max_by_key(|(hits, _)| *hits)
                    .map(|(_, t)| t)
            });
            if named.is_some() {
                take_key(&mut front, "type");
            }
            let mut fields = BTreeMap::new();
            if let Some(t) = t {
                for f in &t.fields {
                    let Some(y) = take_key(&mut front, &f.name) else {
                        continue;
                    };
                    match convert_value(&y, &f.kind, &resolve) {
                        Some(v) => {
                            fields.insert(f.id, v);
                        }
                        None => {
                            front.insert(Yaml::String(f.name.clone()), y);
                        }
                    }
                }
            }

            let mut links = 0;
            let converted = OBSIDIAN_LINK.replace_all(rest, |c: &regex::Captures| {
                let whole = c[0].to_string();
                if !c[1].is_empty() || CRYPT_LINK.is_match(&whole) {
                    return whole;
                }
                let target = link_target(&c[2]);
                match resolve(target) {
                    Some(id) => {
                        links += 1;
                        let alias = c.get(3).map(|m| m.as_str()).unwrap_or(target);
                        format!("[[{id}|{}]]", alias.replace(['|', ']', '\n'], " "))
                    }
                    None => whole,
                }
            });
            report.links += links;
            let body = if front.is_empty() {
                converted.to_string()
            } else {
                format!("{}{converted}", emit_yaml(front))
            };

            let type_id: Option<TypeId> = t.map(|t| t.id);
            report.typed += usize::from(type_id.is_some());
            let new = NewRecord {
                title,
                type_id,
                folder: Some(parent),
                secret_kind: None,
            };
            self.insert_built(id, new, "import", |r| {
                if !body.is_empty() {
                    r.set_body(&body)?;
                }
                for tag in &tags {
                    // Invalid tags (e.g. only "#") are dropped, not fatal.
                    let _ = r.add_tag(tag);
                }
                for (f, v) in fields {
                    r.set_field(f, Some(v))?;
                }
                Ok(())
            })?;
            report.notes += 1;
        }
        Ok(report)
    }

    /// Write every live record as .md into a new dated folder inside `dest`. Secret values
    /// are written only when `include_secrets` is set; otherwise just the name and kind.
    pub fn export_markdown(&self, dest: &Path, include_secrets: bool) -> Result<ExportReport> {
        if !fs::metadata(dest).map(|m| m.is_dir()).unwrap_or(false) {
            return Err(CryptError::ImportFormat("Папка не найдена".into()));
        }
        let mut used_top = HashSet::new();
        for e in fs::read_dir(dest)? {
            used_top.insert(e?.file_name().to_string_lossy().to_lowercase());
        }
        let base = dest.join(unique(&mut used_top, &format!("Crypt {}", today())));
        fs::create_dir(&base)?;

        // Folder paths, with sibling names made unique.
        let folders = self.folders();
        let mut paths: HashMap<FolderId, PathBuf> = HashMap::new();
        let mut used_in: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        let mut pending: Vec<_> = folders.iter().collect();
        while !pending.is_empty() {
            let before = pending.len();
            pending.retain(|f| {
                let parent = match f.parent {
                    None => PathBuf::new(),
                    Some(p) => match paths.get(&p) {
                        Some(path) => path.clone(),
                        None => return true,
                    },
                };
                let name = unique(
                    used_in.entry(parent.clone()).or_default(),
                    &safe_name(&f.name),
                );
                paths.insert(f.id, parent.join(name));
                false
            });
            if pending.len() == before {
                break; // orphaned folders (missing parent) are skipped
            }
        }

        let mut records: Vec<_> = self
            .records
            .values()
            .filter(|r| r.deleted_at().is_none())
            .collect();
        records.sort_by_key(|r| (r.title(), r.id()));
        // File stem for every exported record, for links in bodies and fields.
        let mut stems: HashMap<RecordId, String> = HashMap::new();
        let mut files: Vec<(PathBuf, RecordId)> = Vec::new();
        for r in &records {
            let dir = if r.secret_kind().is_some() {
                PathBuf::from(SECRETS_DIR)
            } else {
                match r.folder() {
                    Some(f) => match paths.get(&f) {
                        Some(p) => p.clone(),
                        None => PathBuf::new(),
                    },
                    None => PathBuf::new(),
                }
            };
            let stem = unique(
                used_in.entry(dir.clone()).or_default(),
                &safe_name(&r.title()),
            );
            stems.insert(r.id(), stem.clone());
            files.push((dir.join(format!("{stem}.md")), r.id()));
        }

        let link_text = |id: &RecordId, alias: Option<&str>| match stems.get(id) {
            Some(stem) => match alias {
                Some(a) if a != stem => format!("[[{stem}|{a}]]"),
                _ => format!("[[{stem}]]"),
            },
            None => alias.unwrap_or_default().to_string(),
        };

        let mut report = ExportReport {
            path: base.to_string_lossy().to_string(),
            ..Default::default()
        };
        for (rel, id) in files {
            let r = &self.records[&id];
            let mut front = Hash::new();
            let mut body = r.body();
            if let Some(kind) = r.secret_kind() {
                let kind_name = serde_json::to_value(kind)?
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                front.insert(Yaml::String("crypt-secret".into()), Yaml::String(kind_name));
                if include_secrets {
                    for name in kind.value_names() {
                        if let Some(v) = r.secret_value(name)? {
                            front.insert(Yaml::String((*name).into()), Yaml::String(v.to_string()));
                        }
                    }
                } else {
                    body = "Значения секрета не экспортировались.\n".into();
                }
            }
            let t = r.type_id().and_then(|t| self.meta.get_type(&t));
            if let Some(t) = &t {
                front.insert(Yaml::String("type".into()), Yaml::String(t.name.clone()));
            }
            let tags = r.tags();
            if !tags.is_empty() {
                front.insert(
                    Yaml::String("tags".into()),
                    Yaml::Array(tags.into_iter().map(Yaml::String).collect()),
                );
            }
            if let Some(t) = &t {
                let values = r.fields();
                for f in &t.fields {
                    let Some(v) = values.get(&f.id).filter(|v| v.fits(&f.kind)) else {
                        continue;
                    };
                    let y = match (v, &f.kind) {
                        (FieldValue::Text(s) | FieldValue::Url(s) | FieldValue::Date(s), _) => {
                            Yaml::String(s.clone())
                        }
                        (FieldValue::Number(n), _) => number_yaml(*n),
                        (FieldValue::Checkbox(b), _) => Yaml::Boolean(*b),
                        (FieldValue::Select(o), FieldKind::Select { options }) => {
                            match options.iter().find(|x| x.id == *o) {
                                Some(o) => Yaml::String(o.label.clone()),
                                None => continue,
                            }
                        }
                        (FieldValue::MultiSelect(ids), FieldKind::MultiSelect { options }) => {
                            Yaml::Array(
                                options
                                    .iter()
                                    .filter(|o| ids.contains(&o.id))
                                    .map(|o| Yaml::String(o.label.clone()))
                                    .collect(),
                            )
                        }
                        (FieldValue::Links(ids), _) => Yaml::Array(
                            ids.iter()
                                .filter(|id| stems.contains_key(id))
                                .map(|id| Yaml::String(link_text(id, None)))
                                .collect(),
                        ),
                        _ => continue,
                    };
                    front.insert(Yaml::String(f.name.clone()), y);
                }
            }
            let body = CRYPT_LINK.replace_all(&body, |c: &regex::Captures| {
                match c[1].parse::<RecordId>() {
                    Ok(id) => link_text(&id, c.get(2).map(|m| m.as_str())),
                    Err(_) => c[0].to_string(),
                }
            });
            let text = if front.is_empty() {
                body.to_string()
            } else {
                format!("{}\n{body}", emit_yaml(front))
            };
            let path = base.join(&rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, text)?;
            if r.secret_kind().is_some() {
                #[cfg(unix)]
                if include_secrets {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
                }
                report.secrets += 1;
            } else {
                report.notes += 1;
            }
        }
        // Empty folders are part of the structure too.
        for p in paths.values() {
            fs::create_dir_all(base.join(p))?;
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_sanitized_and_unique() {
        assert_eq!(safe_name(" a/b:c "), "a-b-c");
        assert_eq!(safe_name("..."), "Без названия");
        let mut used = HashSet::new();
        assert_eq!(unique(&mut used, "X"), "X");
        assert_eq!(unique(&mut used, "x"), "x (2)");
    }
    #[test]
    fn frontmatter_split_tolerates_garbage() {
        let (h, body) = split_frontmatter("---\na: 1\n---\nbody");
        assert_eq!(h.unwrap().len(), 1);
        assert_eq!(body, "body");
        let (h, body) = split_frontmatter("---\n: [\n---\nbody");
        assert!(h.is_none());
        assert!(body.starts_with("---"));
        assert!(split_frontmatter("no front").0.is_none());
    }
    #[test]
    fn date_is_iso() {
        let d = today();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
    }
}
