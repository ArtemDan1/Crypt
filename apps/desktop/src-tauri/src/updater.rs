//! Updates from GitHub Releases, the same way Verge does it: find the latest
//! release, download its .pkg and hand it to macOS Installer.
use std::cmp::Ordering;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::error::{ApiError, ApiResult};

const LATEST: &str = "https://api.github.com/repos/ArtemDan1/Crypt/releases/latest";
/// Only our own release assets are ever downloaded.
const ASSET_PREFIX: &str = "https://github.com/ArtemDan1/Crypt/releases/download/";
const CURL: &str = "/usr/bin/curl";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub pkg_url: String,
    pub release_url: String,
    pub size: u64,
    pub notes: String,
}

/// Versions are release dates "2026.11.4" (older ones "1.0.0"); a leading "v" is ignored.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parse = |v: &str| -> Vec<u64> {
        let v = v.trim().trim_start_matches(['v', 'V']);
        let v = v.split('+').next().unwrap_or_default();
        v.split('.')
            .map(|p| p.trim().parse().unwrap_or(0))
            .collect()
    };
    let (pa, pb) = (parse(a), parse(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (
            pa.get(i).copied().unwrap_or(0),
            pb.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x.cmp(&y);
        }
    }
    Ordering::Equal
}

/// The update to offer from a GitHub "latest release" response, if it is newer.
pub fn pick_update(release: &Value, current: &str) -> Option<UpdateInfo> {
    let tag = release["tag_name"].as_str()?;
    if compare_versions(tag, current) != Ordering::Greater {
        return None;
    }
    let asset = release["assets"].as_array()?.iter().find(|a| {
        a["name"]
            .as_str()
            .is_some_and(|n| n.to_lowercase().ends_with(".pkg"))
            && a["browser_download_url"]
                .as_str()
                .is_some_and(|u| u.starts_with(ASSET_PREFIX))
    })?;
    Some(UpdateInfo {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        pkg_url: asset["browser_download_url"].as_str()?.to_string(),
        release_url: release["html_url"].as_str().unwrap_or_default().to_string(),
        size: asset["size"].as_u64().unwrap_or(0),
        notes: release["body"].as_str().unwrap_or_default().to_string(),
    })
}

fn fetch_latest() -> ApiResult<Option<Value>> {
    let out = Command::new(CURL)
        .args(["-sSL", "--max-time", "20", "-w", "\n%{http_code}"])
        .args(["-H", "Accept: application/vnd.github+json"])
        .args(["-H", "User-Agent: Crypt-updater"])
        .arg(LATEST)
        .output()
        .map_err(|_| ApiError::new("Network", "Не удалось проверить обновления"))?;
    if !out.status.success() {
        return Err(ApiError::new("Network", "Нет связи с GitHub"));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", ""));
    match code.trim() {
        // No published release yet: nothing to update to.
        "404" => Ok(None),
        "200" => serde_json::from_str(body)
            .map(Some)
            .map_err(|_| ApiError::new("Network", "GitHub ответил непонятно")),
        c => Err(ApiError::new("Network", format!("GitHub ответил {c}"))),
    }
}

fn pkg_path() -> PathBuf {
    std::env::temp_dir().join("Crypt-update.pkg")
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> ApiResult<Option<UpdateInfo>> {
    let current = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(fetch_latest()?.and_then(|r| pick_update(&r, &current)))
    })
    .await
    .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}

/// Downloads the .pkg, reporting progress (0…1) as "update-progress" to the asking window.
#[tauri::command]
pub async fn download_update(
    window: tauri::WebviewWindow,
    url: String,
    size: u64,
) -> ApiResult<()> {
    if !url.starts_with(ASSET_PREFIX) || !url.to_lowercase().ends_with(".pkg") {
        return Err(ApiError::new("Validation", "Неверный адрес обновления"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let path = pkg_path();
        let _ = std::fs::remove_file(&path);
        let mut child = Command::new(CURL)
            .args([
                "-fsSL",
                "--max-time",
                "600",
                "-H",
                "User-Agent: Crypt-updater",
                "-o",
            ])
            .arg(&path)
            .arg(&url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ApiError::new("Network", "Не удалось начать загрузку"))?;
        loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|_| ApiError::new("Network", "Загрузка прервалась"))?
            {
                if !status.success() {
                    let _ = std::fs::remove_file(&path);
                    return Err(ApiError::new("Network", "Не удалось скачать обновление"));
                }
                let _ = window.emit("update-progress", 1.0);
                return Ok(());
            }
            if size > 0 {
                let got = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let _ = window.emit("update-progress", (got as f64 / size as f64).min(0.99));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    })
    .await
    .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}

/// Saves edits, locks the vault, opens Installer and quits: Installer replaces
/// the app in /Applications, which must not happen under a running copy.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> ApiResult<()> {
    let path = pkg_path();
    if !path.exists() {
        return Err(ApiError::new("NotFound", "Файл обновления не найден"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        crate::autolock::lock_with_flush(&app);
        let ok = Command::new("/usr/bin/open")
            .arg(&path)
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            return Err(ApiError::new("Internal", "Не удалось запустить установщик"));
        }
        std::thread::sleep(Duration::from_millis(500));
        app.exit(0);
        Ok(())
    })
    .await
    .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions_compare_by_date_parts() {
        assert_eq!(
            compare_versions("2026.11.4", "2026.11.3"),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions("2026.11.10", "2026.11.9"),
            Ordering::Greater
        );
        assert_eq!(compare_versions("v2026.11.4", "2026.11.4"), Ordering::Equal);
        assert_eq!(compare_versions("2026.1.1", "1.0.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "2026.11.4"), Ordering::Less);
    }

    fn release(tag: &str, url: &str) -> Value {
        json!({
            "tag_name": tag,
            "html_url": "https://github.com/ArtemDan1/Crypt/releases/tag/x",
            "body": "notes",
            "assets": [
                { "name": "Crypt_x_aarch64.dmg", "browser_download_url": format!("{ASSET_PREFIX}x/a.dmg"), "size": 1 },
                { "name": "Crypt_x_aarch64.pkg", "browser_download_url": url, "size": 42 }
            ]
        })
    }

    #[test]
    fn offers_only_newer_pkg_from_our_repo() {
        let url = format!("{ASSET_PREFIX}v2026.11.5/Crypt_2026.11.5_aarch64.pkg");
        let u = pick_update(&release("v2026.11.5", &url), "2026.11.4").unwrap();
        assert_eq!(u.version, "2026.11.5");
        assert_eq!(u.pkg_url, url);
        assert_eq!(u.size, 42);
        assert!(pick_update(&release("v2026.11.4", &url), "2026.11.4").is_none());
        assert!(
            pick_update(
                &release("v2026.11.5", "https://evil.example/x.pkg"),
                "2026.11.4"
            )
            .is_none()
        );
    }
}
