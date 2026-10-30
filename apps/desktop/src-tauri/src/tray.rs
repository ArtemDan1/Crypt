pub struct TrayLabels {
    pub status: String,
    pub agent: String,
    pub can_lock: bool,
}
pub fn tray_labels(locked: bool, agent_running: bool, keys: usize) -> TrayLabels {
    let word = if (11..=14).contains(&(keys % 100)) {
        "ключей"
    } else {
        match keys % 10 {
            1 => "ключ",
            2..=4 => "ключа",
            _ => "ключей",
        }
    };
    TrayLabels {
        status: if locked {
            "Crypt заблокирован"
        } else {
            "Crypt открыт"
        }
        .into(),
        agent: if agent_running {
            format!("ssh-agent: работает ({keys} {word})")
        } else {
            "ssh-agent: выключен".into()
        },
        can_lock: !locked,
    }
}
use crate::state::AppState;
use std::sync::Arc;
use tauri::{
    AppHandle, Listener, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
struct TrayState {
    status: MenuItem<tauri::Wry>,
    agent: MenuItem<tauri::Wry>,
    lock: MenuItem<tauri::Wry>,
}
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
    let Some(tray) = app.try_state::<TrayState>() else {
        return;
    };
    let st = app.state::<Arc<AppState>>();
    let locked = st.vault_guard().is_none();
    let labels = tray_labels(
        locked,
        st.agent.running(),
        crate::agent::policy::identities(&st).len(),
    );
    let _ = tray.status.set_text(labels.status);
    let _ = tray.agent.set_text(labels.agent);
    let _ = tray.lock.set_enabled(labels.can_lock);
}
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Crypt заблокирован", false, None::<&str>)?;
    let agent = MenuItem::with_id(
        app,
        "agent-status",
        "ssh-agent: выключен",
        false,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "open", "Открыть Crypt", true, None::<&str>)?;
    let lock = MenuItem::with_id(app, "lock", "Заблокировать", false, Some("CmdOrCtrl+L"))?;
    let quit = MenuItem::with_id(app, "quit", "Выйти Crypt", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &agent,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &lock,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    TrayIconBuilder::with_id("crypt-tray")
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/tray-template@2x.png"
        ))?)
        .icon_as_template(true)
        .tooltip("Crypt")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show(app),
            "lock" => crate::autolock::lock_from_system(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(TrayState {
        status,
        agent,
        lock,
    });
    let a = app.clone();
    app.listen("agent://changed", move |_| {
        let a = a.clone();
        let main = a.clone();
        let _ = main.run_on_main_thread(move || refresh_tray(&a));
    });
    let a = app.clone();
    app.listen("agent://unlock-needed", move |_| {
        let a = a.clone();
        let main = a.clone();
        let _ = main.run_on_main_thread(move || show(&a));
    });
    refresh_tray(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_labels_match_state() {
        for (n, word) in [
            (1, "ключ"),
            (2, "ключа"),
            (5, "ключей"),
            (11, "ключей"),
            (21, "ключ"),
        ] {
            let l = tray_labels(true, true, n);
            assert_eq!(l.status, "Crypt заблокирован");
            assert_eq!(l.agent, format!("ssh-agent: работает ({n} {word})"));
            assert!(!l.can_lock);
        }
        let l = tray_labels(false, false, 0);
        assert_eq!(l.status, "Crypt открыт");
        assert_eq!(l.agent, "ssh-agent: выключен");
        assert!(l.can_lock);
    }
}
