//! Native macOS menu bar, in Russian, with working app commands.
use tauri::{
    AppHandle, Emitter,
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
};

/// What the frontend should do for a menu item. Lock is handled in Rust.
#[derive(Debug, PartialEq, Eq)]
pub enum MenuAction {
    Lock,
    Ui(&'static str),
}

/// Ids are prefixed: tray menu handlers also see these events.
pub fn menu_action(id: &str) -> Option<MenuAction> {
    Some(match id {
        "app-lock" => MenuAction::Lock,
        "app-new-record" => MenuAction::Ui("new-record"),
        "app-new-folder" => MenuAction::Ui("new-folder"),
        "app-search" => MenuAction::Ui("search"),
        "app-settings" => MenuAction::Ui("settings"),
        "app-new-tab" => MenuAction::Ui("new-tab"),
        "app-new-window" => MenuAction::Ui("new-window"),
        "app-close-tab" => MenuAction::Ui("close-tab"),
        "app-close-window" => MenuAction::Ui("close-window"),
        "app-back" => MenuAction::Ui("back"),
        "app-forward" => MenuAction::Ui("forward"),
        "app-next-tab" => MenuAction::Ui("next-tab"),
        "app-prev-tab" => MenuAction::Ui("prev-tab"),
        _ => return None,
    })
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let item =
        |id: &str, text: &str, accel: &str| MenuItem::with_id(app, id, text, true, Some(accel));
    let sep = || PredefinedMenuItem::separator(app);
    let about = AboutMetadata {
        name: Some("Crypt".into()),
        version: Some(app.package_info().version.to_string()),
        ..Default::default()
    };
    let crypt = Submenu::with_items(
        app,
        "Crypt",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("О Crypt"), Some(about))?,
            &sep()?,
            &item("app-settings", "Настройки…", "CmdOrCtrl+,")?,
            &sep()?,
            &PredefinedMenuItem::hide(app, Some("Скрыть Crypt"))?,
            &PredefinedMenuItem::hide_others(app, Some("Скрыть остальные"))?,
            &PredefinedMenuItem::show_all(app, Some("Показать все"))?,
            &sep()?,
            &PredefinedMenuItem::quit(app, Some("Выйти из Crypt"))?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        "Файл",
        true,
        &[
            &item("app-new-tab", "Новая вкладка", "CmdOrCtrl+T")?,
            &item("app-new-window", "Новое окно", "CmdOrCtrl+Alt+N")?,
            &sep()?,
            &item("app-new-record", "Новая запись", "CmdOrCtrl+N")?,
            &item("app-new-folder", "Новая папка", "CmdOrCtrl+Shift+N")?,
            &sep()?,
            &item("app-lock", "Заблокировать", "CmdOrCtrl+L")?,
            &sep()?,
            &item("app-close-tab", "Закрыть вкладку", "CmdOrCtrl+W")?,
            &item("app-close-window", "Закрыть окно", "CmdOrCtrl+Shift+W")?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Правка",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("Отменить"))?,
            &PredefinedMenuItem::redo(app, Some("Повторить"))?,
            &sep()?,
            &PredefinedMenuItem::cut(app, Some("Вырезать"))?,
            &PredefinedMenuItem::copy(app, Some("Копировать"))?,
            &PredefinedMenuItem::paste(app, Some("Вставить"))?,
            &PredefinedMenuItem::select_all(app, Some("Выбрать все"))?,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "Вид",
        true,
        &[
            &item("app-search", "Поиск", "CmdOrCtrl+K")?,
            &sep()?,
            &item("app-back", "Назад", "CmdOrCtrl+[")?,
            &item("app-forward", "Вперёд", "CmdOrCtrl+]")?,
            &sep()?,
            &item("app-next-tab", "Следующая вкладка", "Ctrl+Tab")?,
            &item("app-prev-tab", "Предыдущая вкладка", "Ctrl+Shift+Tab")?,
            &sep()?,
            &PredefinedMenuItem::fullscreen(app, Some("Во весь экран"))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Окно",
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some("Свернуть"))?,
            &PredefinedMenuItem::maximize(app, Some("Изменить масштаб"))?,
        ],
    )?;
    app.set_menu(Menu::with_items(
        app,
        &[&crypt, &file, &edit, &view, &window],
    )?)?;
    app.on_menu_event(|app, event| match menu_action(event.id().as_ref()) {
        Some(MenuAction::Lock) => crate::autolock::lock_from_system(app),
        Some(MenuAction::Ui(action)) => {
            // Each window has its own tabs: act in the one in front.
            let target = crate::windows::focused_app_window(app).unwrap_or_else(|| {
                crate::tray::show(app);
                "main".into()
            });
            let _ = app.emit_to(target.as_str(), "menu-action", action);
        }
        None => {}
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_ids_map_to_actions() {
        assert_eq!(menu_action("app-lock"), Some(MenuAction::Lock));
        assert_eq!(
            menu_action("app-new-record"),
            Some(MenuAction::Ui("new-record"))
        );
        assert_eq!(
            menu_action("app-settings"),
            Some(MenuAction::Ui("settings"))
        );
        assert_eq!(menu_action("app-new-tab"), Some(MenuAction::Ui("new-tab")));
        assert_eq!(menu_action("app-back"), Some(MenuAction::Ui("back")));
        // Tray ids must not trigger app actions.
        assert_eq!(menu_action("lock"), None);
        assert_eq!(menu_action("quit"), None);
    }
}
