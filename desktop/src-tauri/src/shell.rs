//! The desktop part around the engine: the tray icon, keeping the app running when its window is
//! closed (so files can still arrive), and notifications while nobody is looking at the window.

use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, Runtime, State, Window, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_notification::NotificationExt;
use tracing::warn;

use crate::{
    engine::Host,
    filemenu::{self, MenuState},
};

/// Tells the window there are files from `--send` to pick up with `take_send_paths`.
pub const SEND_EVENT: &str = "nhako://send";

const SETTINGS_FILE: &str = "desktop.json";

/// Desktop-only settings, saved next to the engine's `state.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Saved {
    /// Closing the window keeps the app in the tray, so devices can still send files.
    keep_running: bool,
    /// The "still running in the tray" notification was shown once already.
    told_about_tray: bool,
    /// The file manager menu was added on first run (Windows), so a later "off" stays off.
    file_menu_offered: bool,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            keep_running: true,
            told_about_tray: false,
            file_menu_offered: false,
        }
    }
}

pub struct Shell {
    path: PathBuf,
    keep_running: AtomicBool,
    told_about_tray: AtomicBool,
    file_menu_offered: AtomicBool,
    /// Files from `--send` the window hasn't picked up yet.
    send_queue: Mutex<Vec<PathBuf>>,
}

impl Shell {
    pub fn load(config_dir: PathBuf) -> Self {
        let path = config_dir.join(SETTINGS_FILE);
        let saved: Saved = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self {
            path,
            keep_running: AtomicBool::new(saved.keep_running),
            told_about_tray: AtomicBool::new(saved.told_about_tray),
            file_menu_offered: AtomicBool::new(saved.file_menu_offered),
            send_queue: Mutex::new(Vec::new()),
        }
    }

    pub fn keep_running(&self) -> bool {
        self.keep_running.load(Ordering::Relaxed)
    }

    fn save(&self) {
        let saved = Saved {
            keep_running: self.keep_running(),
            told_about_tray: self.told_about_tray.load(Ordering::Relaxed),
            file_menu_offered: self.file_menu_offered.load(Ordering::Relaxed),
        };
        let write = || -> std::io::Result<()> {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&self.path, serde_json::to_vec_pretty(&saved)?)
        };
        if let Err(e) = write() {
            warn!("couldn't save the desktop settings: {e}");
        }
    }
}

pub fn show_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Nhako Beam", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Nhako Beam")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "quit" => quit_app(app.clone()),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Windows and some Linux panels report clicks; others only open the menu.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else { return };
    if window.label() != "main" {
        return;
    }
    api.prevent_close();
    let app = window.app_handle();
    let shell = app.state::<Shell>();
    if !shell.keep_running() {
        quit_app(app.clone());
        return;
    }
    let _ = window.hide();
    if !shell.told_about_tray.swap(true, Ordering::Relaxed) {
        shell.save();
        notify(
            app,
            "Nhako Beam is still running",
            "Devices can still send you files. To quit, use Quit in the tray menu.",
        );
    }
}

/// Closes the network properly (so other devices see us leave right away), then exits.
pub fn quit_app<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
        app.state::<Host>().stop().await;
        app.exit(0);
    });
}

/// Shows a notification unless the window is open and focused.
pub fn notify_if_away<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    let looking = app
        .get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(false));
    if !looking {
        notify(app, title, body);
    }
}

fn notify<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        warn!("couldn't show a notification: {e}");
    }
}

impl Shell {
    /// Queues files from `--send` (first launch, or a later launch handed over to us).
    pub fn queue_send<R: Runtime>(&self, app: &AppHandle<R>, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        self.send_queue.lock().expect("send queue lock").extend(paths);
        let _ = app.emit(SEND_EVENT, ());
    }

    /// First run on Windows: add "Send with Nhako Beam" to Explorer, as an installed app would.
    /// Linux leaves it to the switch in Settings (an app shouldn't add scripts to Files unasked).
    /// Either way an existing entry is pointed at this copy of the app.
    pub fn setup_file_menu(&self, data_dir: &Path) {
        if cfg!(windows) && !self.file_menu_offered.swap(true, Ordering::Relaxed) {
            if let Err(e) = filemenu::set(data_dir, true) {
                warn!("couldn't add the Explorer menu: {e}");
            }
            self.save();
        }
        filemenu::refresh(data_dir);
    }
}

/// Files from `--send` the window hasn't shown yet. Each file is handed out once.
#[tauri::command]
pub fn take_send_paths(shell: State<'_, Shell>) -> Vec<PathBuf> {
    std::mem::take(&mut *shell.send_queue.lock().expect("send queue lock"))
}

#[tauri::command]
pub fn get_file_menu(app: AppHandle) -> Result<MenuState, String> {
    Ok(filemenu::state(&app.path().data_dir().map_err(|e| e.to_string())?))
}

#[tauri::command]
pub fn set_file_menu(app: AppHandle, on: bool) -> Result<MenuState, String> {
    let dir = app.path().data_dir().map_err(|e| e.to_string())?;
    filemenu::set(&dir, on).map_err(|e| format!("Couldn't change the menu: {e}"))?;
    Ok(filemenu::state(&dir))
}

#[tauri::command]
pub fn get_background(shell: State<'_, Shell>) -> bool {
    shell.keep_running()
}

#[tauri::command]
pub fn set_background(shell: State<'_, Shell>, on: bool) {
    shell.keep_running.store(on, Ordering::Relaxed);
    shell.save();
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    quit_app(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_running_by_default_and_remembers_the_choice() {
        let dir = std::env::temp_dir().join(format!("nhako-shell-{}", std::process::id()));
        let shell = Shell::load(dir.clone());
        assert!(shell.keep_running(), "on by default, so files can arrive with the window closed");
        shell.keep_running.store(false, Ordering::Relaxed);
        shell.save();
        assert!(!Shell::load(dir.clone()).keep_running());
        let _ = std::fs::remove_dir_all(dir);
    }
}
