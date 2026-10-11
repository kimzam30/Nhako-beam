//! The Nhako Beam desktop app: the Svelte UI in a Tauri window, running the real engine.
//!
//! - `engine.rs` holds the engine and turns the app's `Engine` contract
//!   (`desktop/src/lib/engine/types.ts`) into Tauri commands and events.
//! - `shell.rs` is the desktop part: tray icon, keep running in the background, notifications.
//! - `filemenu.rs` is "Send with Nhako Beam" in Explorer and Files, and the `--send` option.

mod engine;
mod filemenu;
mod shell;

use std::path::Path;

use tauri::Manager;

/// Started by "start at login": stay in the tray instead of opening the window.
pub const HIDDEN_ARG: &str = "--hidden";

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,iroh=warn,iroh_relay=warn,netwatch=warn,portmapper=warn".into()),
        )
        .init();

    tauri::Builder::default()
        // A second launch (from the menu, the .desktop file, or "Send with Nhako Beam") shows the
        // window that is already running, and hands over any files to send.
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            app.state::<shell::Shell>().queue_send(app, filemenu::send_paths(&args, Path::new(&cwd)));
            shell::show_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            app.manage(engine::Host::new(config_dir.clone()));
            let shell = shell::Shell::load(config_dir);
            shell.setup_file_menu(&app.path().data_dir()?);
            let args: Vec<String> = std::env::args().collect();
            let cwd = std::env::current_dir().unwrap_or_default();
            shell.queue_send(app.handle(), filemenu::send_paths(&args, &cwd));
            app.manage(shell);
            shell::build_tray(app.handle())?;
            if !std::env::args().any(|a| a == HIDDEN_ARG) {
                shell::show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(shell::on_window_event)
        .invoke_handler(tauri::generate_handler![
            engine::engine_start,
            engine::engine_stop,
            engine::rescan,
            engine::files_from_paths,
            engine::send,
            engine::accept,
            engine::decline,
            engine::pause,
            engine::resume,
            engine::cancel,
            engine::retry,
            engine::create_code,
            engine::cancel_code,
            engine::redeem_code,
            engine::set_trusted,
            engine::set_auto_accept,
            engine::rename_self,
            engine::get_self,
            engine::set_save_dir,
            engine::get_connection,
            engine::set_connection,
            engine::check_server,
            engine::open_received,
            shell::take_send_paths,
            shell::get_file_menu,
            shell::set_file_menu,
            shell::get_background,
            shell::set_background,
            shell::quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the app")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // Closing the last window with "keep running" on is not a real exit.
                if code.is_none() && app.state::<shell::Shell>().keep_running() {
                    api.prevent_exit();
                }
            }
        });
}
