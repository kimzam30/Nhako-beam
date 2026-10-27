//! The Nhako Beam desktop app: the Svelte UI in a Tauri window, running the real engine.
//!
//! - `engine.rs` holds the engine and turns the app's `Engine` contract
//!   (`desktop/src/lib/engine/types.ts`) into Tauri commands and events.
//! - `shell.rs` is the desktop part: tray icon, keep running in the background, notifications.
//! - `filemenu.rs` is "Send with Nhako Beam" in Explorer and Files, and the `--send` option.
//! - `popover.rs` is the small window next to the tray icon.
//! - `sound.rs` plays the app's sounds (design/sounds/).

mod engine;
mod filemenu;
mod popover;
mod shell;
mod sound;

use std::path::Path;

use tauri::Manager;

/// Started by "start at login": stay in the tray instead of opening the window.
pub const HIDDEN_ARG: &str = "--hidden";

const LOG_FILTER: &str = "info,iroh=warn,iroh_relay=warn,netwatch=warn,portmapper=warn";

/// Logs to the terminal and to the log file in the config folder (the Windows app has no terminal,
/// so the file is the only record; Settings > About > Copy diagnostics includes it).
fn init_logging(config_dir: &Path) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    let file = nhako_core::logfile::install(config_dir);
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| LOG_FILTER.into()))
        .with(fmt::layer())
        .with(fmt::layer().with_ansi(false).with_writer(move || file.writer()))
        .try_init();
}

pub fn run() {
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
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(sound::Sounds::new());
            let config_dir = app.path().app_config_dir()?;
            init_logging(&config_dir);
            tracing::info!(version = %app.package_info().version, "Nhako Beam desktop starting");
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
        .on_webview_event(shell::on_webview_event)
        .invoke_handler(tauri::generate_handler![
            engine::engine_start,
            engine::engine_stop,
            engine::rescan,
            engine::files_from_paths,
            engine::pick_files,
            engine::pick_folders,
            engine::send_text,
            engine::read_clipboard,
            engine::write_clipboard,
            engine::ask_trust,
            engine::cancel_trust,
            engine::answer_trust,
            engine::share_link,
            engine::stop_link,
            engine::answer_visitor,
            engine::get_update_check,
            engine::set_update_check,
            engine::pick_save_dir,
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
            engine::diagnostics,
            engine::get_connection,
            engine::set_connection,
            engine::check_server,
            engine::get_lan_discovery,
            engine::set_lan_discovery,
            engine::get_who_can_send,
            engine::set_who_can_send,
            engine::clear_history,
            shell::open_link,
            popover::open_main,
            popover::hand_to_main,
            engine::open_received,
            shell::take_send_paths,
            shell::get_file_menu,
            shell::set_file_menu,
            shell::get_background,
            shell::set_background,
            shell::quit,
            sound::play_sound,
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
