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
            &item("app-new-record", "Новая запись", "CmdOrCtrl+N")?,
            &item("app-new-folder", "Новая папка", "CmdOrCtrl+Shift+N")?,
            &sep()?,
            &item("app-lock", "Заблокировать", "CmdOrCtrl+L")?,
            &sep()?,
            &PredefinedMenuItem::close_window(app, Some("Закрыть окно"))?,
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
            crate::tray::show(app);
            let _ = app.emit("menu-action", action);
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
        // Tray ids must not trigger app actions.
        assert_eq!(menu_action("lock"), None);
        assert_eq!(menu_action("quit"), None);
    }
}
