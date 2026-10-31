pub mod agent;
pub mod app_menu;
pub mod autolock;
pub mod clipboard;
pub mod commands;
pub mod error;
pub mod ssh_config;
pub mod state;
pub mod tray;
pub mod recent;
pub mod unlock;

use commands::library as lib_cmd;
use commands::lifecycle::cmd as lifecycle;
use commands::unlock as unlock_cmd;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let st = std::sync::Arc::new(AppState::new(
                data.join("vault"),
                data.join("settings.json"),
                Box::new(crate::unlock::keystore::MacKeychain),
                Box::new(crate::unlock::macos::MacAuthenticator),
            ));
            *st.app.lock().unwrap_or_else(|e| e.into_inner()) = Some(app.handle().clone());
            app.manage(st.clone());
            agent::start_configured(st.clone());
            autolock::observe_system(app.handle());
            tray::init(app.handle())?;
            app_menu::init(app.handle())?;
            if std::env::args().any(|s| s == "--hidden") {
                tray::hide(app.handle());
            } else {
                tray::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                tray::hide(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::transfer::preview_browser_csv,
            commands::transfer::import_browser_csv,
            commands::transfer::preview_otp,
            commands::transfer::import_otp,
            commands::transfer::cancel_otp_import,
            commands::transfer::trash_file,
            commands::backup::export_backup,
            commands::backup::inspect_backup,
            commands::backup::restore_backup,
            ssh_config::ssh_config_status,
            ssh_config::configure_ssh_config,
            ssh_config::unconfigure_ssh_config,
            commands::secrets::cmd::secret_overview,
            commands::secrets::cmd::reveal_secret,
            commands::secrets::cmd::copy_secret,
            commands::secrets::cmd::recent_secrets,
            lifecycle::show,
            lifecycle::tray_hide,
            lifecycle::app_quit,
            commands::secrets::cmd::set_secret,
            commands::secrets::cmd::secret_history,
            commands::secrets::cmd::reveal_secret_revision,
            commands::secrets::cmd::copy_secret_revision,
            commands::secrets::cmd::generate_password,
            commands::secrets::cmd::generate_passwords,
            commands::secrets::cmd::copy_text,
            commands::secrets::cmd::totp_now,
            commands::secrets::cmd::copy_totp,
            commands::secrets::cmd::set_ssh_key,
            commands::secrets::cmd::generate_ssh_key,
            commands::secrets::cmd::set_agent_enabled,
            commands::ssh_import::scan_ssh_dir,
            commands::ssh_import::import_ssh_keys,
            agent::agent_status,
            lifecycle::status,
            lifecycle::delete_vault,
            lifecycle::print_seed,
            lifecycle::generate_seed,
            lifecycle::create_vault,
            lifecycle::unlock_with_seed,
            lifecycle::lock,
            lifecycle::get_settings,
            lifecycle::set_settings,
            lifecycle::vault_path,
            autolock::flush_done,
            unlock_cmd::unlock_methods,
            unlock_cmd::enable_biometric,
            unlock_cmd::disable_biometric,
            unlock_cmd::unlock_biometric,
            unlock_cmd::set_pin,
            unlock_cmd::remove_pin,
            unlock_cmd::unlock_pin,
            unlock_cmd::reveal_seed,
            lib_cmd::list_types,
            lib_cmd::put_type,
            lib_cmd::delete_type,
            lib_cmd::starter_types,
            lib_cmd::list_folders,
            lib_cmd::create_folder,
            lib_cmd::rename_folder,
            lib_cmd::move_folder,
            lib_cmd::delete_folder,
            lib_cmd::list_views,
            lib_cmd::put_view,
            lib_cmd::delete_view,
            lib_cmd::list_records,
            lib_cmd::get_record,
            lib_cmd::get_records,
            lib_cmd::create_record,
            lib_cmd::set_title,
            lib_cmd::set_body,
            lib_cmd::set_type,
            lib_cmd::set_folder,
            lib_cmd::add_tag,
            lib_cmd::remove_tag,
            lib_cmd::set_field,
            lib_cmd::delete_record,
            lib_cmd::restore_record,
            lib_cmd::purge_record,
            lib_cmd::empty_trash,
            lib_cmd::search,
            lib_cmd::backlinks,
            lib_cmd::all_tags,
            lib_cmd::record_versions,
            lib_cmd::record_at_version,
            lib_cmd::revert_record,
            lib_cmd::list_snapshots,
            lib_cmd::create_snapshot,
            lib_cmd::delete_snapshot,
            lib_cmd::restore_snapshot,
            commands::transfer::import_keepass,
            commands::transfer::import_markdown,
            commands::transfer::export_markdown,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Crypt")
        .run(|app, event| match event {
            tauri::RunEvent::Reopen { .. } => tray::show(app),
            tauri::RunEvent::Exit => {
                if let Some(st) = app.try_state::<std::sync::Arc<AppState>>() {
                    st.agent.stop();
                }
            }
            _ => {}
        });
}
