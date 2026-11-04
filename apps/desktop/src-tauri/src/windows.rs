//! Extra app windows ("Открыть в новом окне") next to the main one.
use std::sync::atomic::{AtomicU32, Ordering};

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::{ApiError, ApiResult};

static NEXT: AtomicU32 = AtomicU32::new(1);

/// Main window and windows opened from tabs; never the tray popover.
pub fn is_app_window(label: &str) -> bool {
    label == "main" || label.starts_with("w-")
}

/// Every open app window, hidden ones included: they all hold editors.
pub fn app_window_labels(app: &AppHandle) -> Vec<String> {
    app.webview_windows()
        .into_keys()
        .filter(|l| is_app_window(l))
        .collect()
}

/// The app window the user is looking at, if any.
pub fn focused_app_window(app: &AppHandle) -> Option<String> {
    app.webview_windows()
        .into_iter()
        .find(|(l, w)| is_app_window(l) && w.is_focused().unwrap_or(false))
        .map(|(l, _)| l)
}

/// Whether some app window other than `except` is on screen.
pub fn other_window_visible(app: &AppHandle, except: &str) -> bool {
    app.webview_windows()
        .iter()
        .any(|(l, w)| l != except && is_app_window(l) && w.is_visible().unwrap_or(false))
}

/// Only in-app routes: "/r/<id>", "/folder/<id>"… never a URL.
pub fn valid_route(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains(['#', '\\'])
        && path.len() < 512
}

#[tauri::command]
pub fn open_window(app: AppHandle, path: String) -> ApiResult<()> {
    if !valid_route(&path) {
        return Err(ApiError::new("Validation", "Неверный адрес страницы"));
    }
    let label = format!("w-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let url = WebviewUrl::App(format!("index.html#{path}").into());
    let builder = WebviewWindowBuilder::new(&app, &label, url)
        .title("Crypt")
        .inner_size(1100.0, 760.0)
        .min_inner_size(960.0, 640.0);
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(16.0, 20.0));
    builder
        .build()
        .map_err(|_| ApiError::new("Internal", "Не удалось открыть окно"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels() {
        assert!(is_app_window("main"));
        assert!(is_app_window("w-3"));
        assert!(!is_app_window("tray"));
    }
    #[test]
    fn routes() {
        assert!(valid_route("/r/abc"));
        assert!(valid_route("/"));
        assert!(valid_route("/tag/%D0%B0"));
        assert!(!valid_route("https://evil.example"));
        assert!(!valid_route("//evil.example"));
        assert!(!valid_route("/r/x#y"));
        assert!(!valid_route(""));
    }
}
