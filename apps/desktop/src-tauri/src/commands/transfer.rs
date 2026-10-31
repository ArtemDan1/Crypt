//! Import (KeePass, Obsidian) and Markdown export. Each import first takes a vault
//! snapshot so it can be rolled back from Settings.
use crate::commands::library::with_vault;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crypt_core::{ExportReport, KeepassReport, MarkdownImportReport, Vault};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tauri::State;
use zeroize::Zeroizing;

const MAX_IMPORT_FILE: u64 = 200 * 1024 * 1024;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Imported<T> {
    pub report: T,
    pub snapshot: String,
}

fn read_limited(path: &str) -> ApiResult<Zeroizing<Vec<u8>>> {
    let meta = std::fs::metadata(path).map_err(|_| ApiError::new("ImportFormat", "Файл не найден"))?;
    if !meta.is_file() || meta.len() > MAX_IMPORT_FILE {
        return Err(ApiError::new("ImportFormat", "Файл не подходит для импорта"));
    }
    Ok(Zeroizing::new(
        std::fs::read(path).map_err(|_| ApiError::new("ImportFormat", "Не удалось прочитать файл"))?,
    ))
}
fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}
fn with_snapshot<T>(
    v: &mut Vault,
    label: String,
    f: impl FnOnce(&mut Vault) -> crypt_core::Result<T>,
) -> crypt_core::Result<Imported<T>> {
    let snapshot = v.create_snapshot(&label)?;
    match f(v) {
        Ok(report) => Ok(Imported { report, snapshot: snapshot.name }),
        Err(e) => {
            // Nothing (or only part) was imported: drop the snapshot only when nothing was.
            let total = v.list(&Default::default()).len()
                + v.list(&crypt_core::ListFilter { trashed: true, ..Default::default() }).len();
            if total == snapshot.records {
                let _ = v.delete_snapshot(&snapshot.id);
            }
            Err(e)
        }
    }
}

pub fn import_keepass_file(
    st: &AppState,
    path: &str,
    password: Option<&str>,
    keyfile: Option<&str>,
) -> ApiResult<Imported<KeepassReport>> {
    let data = read_limited(path)?;
    let key = keyfile.map(read_limited).transpose()?;
    with_vault(st, |v| {
        with_snapshot(v, format!("Перед импортом {}", file_name(path)), |v| {
            v.import_keepass(&data, password, key.as_deref().map(|k| k.as_slice()))
        })
    })
}

#[tauri::command]
pub async fn import_keepass(
    st: State<'_, Arc<AppState>>,
    path: String,
    password: Option<String>,
    keyfile: Option<String>,
) -> ApiResult<Imported<KeepassReport>> {
    let password = password.map(Zeroizing::new);
    import_keepass_file(&st, &path, password.as_deref().map(String::as_str), keyfile.as_deref())
}

#[tauri::command]
pub async fn import_markdown(
    st: State<'_, Arc<AppState>>,
    path: String,
) -> ApiResult<Imported<MarkdownImportReport>> {
    with_vault(&st, |v| {
        with_snapshot(v, format!("Перед импортом {}", file_name(&path)), |v| {
            v.import_markdown_dir(Path::new(&path))
        })
    })
}

#[tauri::command]
pub async fn export_markdown(
    st: State<'_, Arc<AppState>>,
    path: String,
    include_secrets: bool,
) -> ApiResult<ExportReport> {
    with_vault(&st, |v| v.export_markdown(Path::new(&path), include_secrets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::lifecycle::{create_vault, generate_seed};

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

    #[test]
    fn failed_import_leaves_no_snapshot_and_success_keeps_one() {
        let (dir, st) = state();
        let junk = dir.path().join("junk.kdbx");
        std::fs::write(&junk, b"not a database").unwrap();
        let err = import_keepass_file(&st, junk.to_str().unwrap(), Some("pw"), None).unwrap_err();
        assert_eq!(err.code, "ImportFormat");
        assert!(with_vault(&st, |v| Ok(v.snapshots())).unwrap().is_empty());

        let notes = dir.path().join("Заметки");
        std::fs::create_dir(&notes).unwrap();
        std::fs::write(notes.join("a.md"), "hello").unwrap();
        let out = with_vault(&st, |v| {
            with_snapshot(v, "Перед импортом Заметки".into(), |v| v.import_markdown_dir(&notes))
        })
        .unwrap();
        assert_eq!(out.report.notes, 1);
        assert_eq!(out.snapshot, "Перед импортом Заметки");
        assert_eq!(with_vault(&st, |v| Ok(v.snapshots())).unwrap().len(), 1);
    }

    #[test]
    fn missing_file_is_a_clear_error() {
        let (_d, st) = state();
        let err = import_keepass_file(&st, "/nonexistent/x.kdbx", Some("pw"), None).unwrap_err();
        assert_eq!(err.code, "ImportFormat");
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use crate::commands::lifecycle::{create_vault,generate_seed,lock};
    fn state() -> (tempfile::TempDir,AppState) {
        let d=tempfile::tempdir().unwrap();let st=AppState::new(d.path().join("vault"),d.path().join("settings.json"),Box::new(crate::unlock::keystore::MemoryKeyStore::default()),Box::new(crate::unlock::authenticator::FakeAuthenticator::ok()));create_vault(&st,generate_seed()).unwrap();(d,st)
    }
    fn source(d:&Path) -> String {let path=d.join("aegis.json");std::fs::write(&path,include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../crates/crypt-core/tests/fixtures/otp/aegis_plain.json"))).unwrap();path.to_string_lossy().into_owned()}
    #[test]
    fn browser_import_creates_logins_and_snapshot() {
        let (d,st)=state();let path=d.path().join("browser.csv");std::fs::write(&path,b"name,url,username,password,note\nMail,https://example.com,u,p,\nMissing,https://x,u,,").unwrap();
        let path=path.to_str().unwrap();let p=preview_csv_file(&st,path).unwrap();assert_eq!((p.new,p.skipped),(1,1));
        let report=import_csv_file(&st,path,"Browser").unwrap();assert_eq!(report.report.imported,1);assert_eq!(report.report.skipped.len(),1);
        assert_eq!(st.vault_guard().as_ref().unwrap().snapshots().len(),1);
    }
    #[test]
    fn otp_preview_token_is_one_use_and_expiring() {
        let (d,st)=state();let path=source(d.path());
        let preview=preview_otp_source(&st,OtpSource::Aegis {path:path.clone(),password:None}).unwrap();assert_eq!(preview.rows.len(),1);
        let r=import_otp_pending(&st,&preview.token,"2FA").unwrap();assert_eq!(r.report.created,1);
        assert!(import_otp_pending(&st,&preview.token,"2FA").is_err());
        let _p=preview_otp_source(&st,OtpSource::Aegis {path:path.clone(),password:None}).unwrap();
        assert!(import_otp_pending(&st,"wrong","2FA").is_err());
        assert!(st.pending_otp.lock().unwrap().is_none());
        let p=preview_otp_source(&st,OtpSource::Aegis {path,password:None}).unwrap();
        st.pending_otp.lock().unwrap().as_mut().unwrap().created=std::time::Instant::now()-OTP_TTL-std::time::Duration::from_secs(1);
        assert!(import_otp_pending(&st,&p.token,"2FA").is_err());
        assert!(st.pending_otp.lock().unwrap().is_none());
    }
    #[test]
    fn lock_clears_drafts_and_cancel_rejects_late_preview() {
        let (d,st)=state();let path=source(d.path());
        preview_otp_source(&st,OtpSource::Aegis {path,password:None}).unwrap();lock(&st);assert!(st.pending_otp.lock().unwrap().is_none());
        let (_d,st)=state();let epoch=st.otp_generation.load(std::sync::atomic::Ordering::SeqCst);cancel_pending(&st);
        let parsed=crypt_core::import_otp::parse_aegis(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../crates/crypt-core/tests/fixtures/otp/aegis_plain.json")),None).unwrap();
        assert!(store_otp_preview(&st,parsed,epoch).is_err());assert!(st.pending_otp.lock().unwrap().is_none());
    }
}

use crypt_core::{import_csv::{parse_browser_csv,CsvFormat},import_apply::{LoginPreview,LoginImportReport},import_otp::{self,OtpParse,OtpSkip,OtpTarget,OtpImportReport}};
use serde::Deserialize;
use std::sync::atomic::Ordering;
use std::time::{Instant,Duration};
const OTP_TTL: Duration = Duration::from_secs(5*60);
pub struct PendingOtp {pub token:String,pub created:Instant,pub parsed:OtpParse}
#[derive(Serialize)]
pub struct BrowserPreview {pub format:CsvFormat,pub new:usize,pub duplicates:usize,pub skipped:usize,pub sample:Vec<(String,String)>}
pub fn preview_csv_file(st:&AppState,path:&str) -> ApiResult<BrowserPreview> {
    let bytes=read_limited(path)?;let parsed=parse_browser_csv(&bytes)?;
    let LoginPreview {new,duplicates,sample,..}=with_vault(st,|v|Ok(v.preview_logins(&parsed.logins)))?;
    Ok(BrowserPreview {format:parsed.format,new,duplicates,skipped:parsed.skipped.len(),sample})
}
pub fn import_csv_file(st:&AppState,path:&str,folder:&str) -> ApiResult<Imported<LoginImportReport>> {
    let bytes=read_limited(path)?;let parsed=parse_browser_csv(&bytes)?;
    with_vault(st,|v|with_snapshot(v,format!("Перед импортом {}",file_name(path)),|v| {
        let mut report=v.import_logins(parsed.logins,folder)?;report.skipped=parsed.skipped;Ok(report)
    }))
}
fn optional_password<'de,D:serde::Deserializer<'de>>(d:D)->Result<Option<Zeroizing<String>>,D::Error> {Option::<String>::deserialize(d).map(|p|p.map(Zeroizing::new))}
#[derive(Deserialize)]
#[serde(tag="kind",rename_all="camelCase")]
pub enum OtpSource {
    Images {paths:Vec<String>},
    ClipboardImage,
    Aegis {path:String,#[serde(default,deserialize_with="optional_password")]password:Option<Zeroizing<String>>},
    Twofas {path:String},
}
#[derive(Serialize)]
pub struct OtpRow {pub issuer:String,pub account:String,pub target:serde_json::Value}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct OtpPreview {pub token:String,pub rows:Vec<OtpRow>,pub skipped:Vec<(String,OtpSkip)>,pub missing_batches:Vec<(u32,u32)>}
pub fn cancel_pending(st:&AppState) -> u64 {
    let mut pending=st.pending_otp.lock().unwrap_or_else(|e|e.into_inner());
    let epoch=st.otp_generation.fetch_add(1,Ordering::SeqCst)+1;
    *pending=None;epoch
}
pub fn store_otp_preview(st:&AppState,parsed:OtpParse,epoch:u64) -> ApiResult<OtpPreview> {
    with_vault(st,|v| {
        let mut pending=st.pending_otp.lock().unwrap_or_else(|e|e.into_inner());
        if st.otp_generation.load(Ordering::SeqCst)!=epoch {return Err(crypt_core::CryptError::Validation("Предпросмотр отменён".into()));}
        let rows=v.preview_otp(&parsed.drafts).into_iter().map(|r|OtpRow {issuer:r.issuer,account:r.account,target:match r.target {OtpTarget::New=>serde_json::json!("new"),OtpTarget::Attach {title,..}=>serde_json::json!({"attach":title})}}).collect();
        let mut random=[0u8;24];use rand::RngCore;rand::rngs::OsRng.fill_bytes(&mut random);let token=random.iter().map(|b|format!("{b:02x}")).collect::<String>();
        let result=OtpPreview {token:token.clone(),rows,skipped:parsed.skipped.clone(),missing_batches:parsed.missing_batches.clone()};
        *pending=Some(PendingOtp {token,created:Instant::now(),parsed});Ok(result)
    })
}
pub fn preview_otp_source(st:&AppState,source:OtpSource) -> ApiResult<OtpPreview> {
    let epoch=cancel_pending(st);
    let parsed=match source {
        OtpSource::Aegis {path,password}=>import_otp::parse_aegis(&read_limited(&path)?,password.as_deref().map(String::as_str))?,
        OtpSource::Twofas {path}=>import_otp::parse_2fas(&read_limited(&path)?)?,
        OtpSource::ClipboardImage=>return Err(ApiError::new("ImportFormat","Выберите изображения QR-кодов из файлов")),
        OtpSource::Images {paths}=>{
            if paths.is_empty() || paths.len()>32 {return Err(ApiError::new("ImportFormat","Выберите от 1 до 32 изображений"));}
            let mut images=Zeroizing::new(Vec::<Vec<u8>>::new());let mut total=0u64;
            for path in paths {let mut bytes=read_limited(&path)?;total+=bytes.len() as u64;if total>MAX_IMPORT_FILE {return Err(ApiError::new("ImportFormat","Изображения занимают больше 200 МБ"));}images.push(std::mem::take(&mut *bytes));}
            let uris=Zeroizing::new(import_otp::decode_qr_images(&images)?);import_otp::parse_otp_uris(&uris)
        }
    };
    store_otp_preview(st,parsed,epoch)
}
pub fn import_otp_pending(st:&AppState,token:&str,folder:&str) -> ApiResult<Imported<OtpImportReport>> {
    with_vault(st,|v| {
        let pending=st.pending_otp.lock().unwrap_or_else(|e|e.into_inner()).take().ok_or_else(||crypt_core::CryptError::Validation("Предпросмотр истёк. Выберите файл заново".into()))?;
        if pending.token!=token || pending.created.elapsed()>OTP_TTL {return Err(crypt_core::CryptError::Validation("Предпросмотр истёк. Выберите файл заново".into()));}
        with_snapshot(v,"Перед импортом 2FA".into(),|v| {
            let mut report=v.import_otp(pending.parsed.drafts,folder)?;report.skipped=pending.parsed.skipped;report.missing_batches=pending.parsed.missing_batches;Ok(report)
        })
    })
}
#[tauri::command]
pub async fn preview_browser_csv(st:State<'_,Arc<AppState>>,path:String)->ApiResult<BrowserPreview> {
    let st=st.inner().clone();tauri::async_runtime::spawn_blocking(move||preview_csv_file(&st,&path)).await.map_err(|_|ApiError::new("Internal","Внутренняя ошибка"))?
}
#[tauri::command]
pub async fn import_browser_csv(st:State<'_,Arc<AppState>>,path:String,folder:String)->ApiResult<Imported<LoginImportReport>> {
    let st=st.inner().clone();tauri::async_runtime::spawn_blocking(move||import_csv_file(&st,&path,&folder)).await.map_err(|_|ApiError::new("Internal","Внутренняя ошибка"))?
}
#[tauri::command]
pub async fn preview_otp(st:State<'_,Arc<AppState>>,source:OtpSource)->ApiResult<OtpPreview> {
    let state=st.inner().clone();
    let result=tauri::async_runtime::spawn_blocking(move||preview_otp_source(&state,source)).await.map_err(|_|ApiError::new("Internal","Внутренняя ошибка"))??;
    // Keep only an opaque token in the expiry worker; draft material stays in AppState.
    let weak=Arc::downgrade(st.inner());let token=result.token.clone();
    std::thread::spawn(move|| {std::thread::sleep(OTP_TTL);if let Some(st)=weak.upgrade() {let mut p=st.pending_otp.lock().unwrap_or_else(|e|e.into_inner());if p.as_ref().is_some_and(|p|p.token==token && p.created.elapsed()>=OTP_TTL) {*p=None;}}});
    Ok(result)
}
#[tauri::command]
pub async fn import_otp(st:State<'_,Arc<AppState>>,token:String,folder:String)->ApiResult<Imported<OtpImportReport>> {
    let st=st.inner().clone();tauri::async_runtime::spawn_blocking(move||import_otp_pending(&st,&token,&folder)).await.map_err(|_|ApiError::new("Internal","Внутренняя ошибка"))?
}
#[tauri::command]
pub async fn cancel_otp_import(st:State<'_,Arc<AppState>>) -> ApiResult<()> {cancel_pending(&st);Ok(())}
#[tauri::command]
pub async fn trash_file(st:State<'_,Arc<AppState>>,path:String)->ApiResult<()> {
    let root=st.vault_dir.clone();
    tauri::async_runtime::spawn_blocking(move|| {
        let path=Path::new(&path);let meta=std::fs::symlink_metadata(path).map_err(|_|ApiError::new("Io","Файл не найден"))?;
        if !meta.is_file() {return Err(ApiError::new("Validation","Можно переместить только обычный файл"));}
        let canonical=std::fs::canonicalize(path).map_err(|_|ApiError::new("Io","Файл не найден"))?;
        if canonical.starts_with(std::fs::canonicalize(root).unwrap_or_default()) {return Err(ApiError::new("Validation","Нельзя удалять файлы хранилища"));}
        #[cfg(target_os="macos")] {
            use objc2_foundation::{NSFileManager,NSURL,NSString};
            let url=NSURL::fileURLWithPath(&NSString::from_str(canonical.to_str().ok_or_else(||ApiError::new("Io","Неверный путь"))?));
            NSFileManager::defaultManager().trashItemAtURL_resultingItemURL_error(&url,None).map_err(|_|ApiError::new("Io","Не удалось переместить файл в Корзину"))?;Ok(())
        }
        #[cfg(not(target_os="macos"))] {Err(ApiError::new("Unavailable","Системная Корзина доступна только на macOS"))}
    }).await.map_err(|_|ApiError::new("Internal","Внутренняя ошибка"))?
}
