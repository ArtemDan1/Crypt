use tauri::{AppHandle, Emitter, Listener, Manager, tray::{TrayIconBuilder, TrayIconEvent, MouseButtonState}};
pub fn show(app: &AppHandle) {
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

pub fn refresh_tray(app: &AppHandle) {
    let _ = app.emit_to("tray", "tray://refresh", ());
}
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("tray") {
        let w = window.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Focused(false)) { let _ = w.hide(); }
        });
    }
    TrayIconBuilder::with_id("crypt-tray")
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template@2x.png"))?)
        .icon_as_template(true)
        .tooltip("Crypt")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { rect, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("tray") {
                    if w.is_visible().unwrap_or(false) { let _ = w.hide(); return; }
                    let scale = w.scale_factor().unwrap_or(1.0);
                    let pos = rect.position.to_logical::<f64>(scale);
                    let size = rect.size.to_logical::<f64>(scale);
                    let _ = w.set_position(tauri::LogicalPosition::new(pos.x + size.width - 340.0, pos.y + size.height + 6.0));
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = app.emit_to("tray", "tray://shown", ());
                }
            }
        }).build(app)?;
    for event in ["agent://changed", "agent://unlock-needed"] {
        let a = app.clone();
        app.listen(event, move |_| refresh_tray(&a));
    }
    Ok(())
}
