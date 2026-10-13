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
    popover,
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
    let menu = if cfg!(windows) {
        Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?
    } else {
        // Linux panels don't report clicks on the icon, so the popover is in the menu.
        let quick = MenuItem::with_id(app, "quick", "Quick view", true, None::<&str>)?;
        Menu::with_items(app, &[&quick, &open, &PredefinedMenuItem::separator(app)?, &quit])?
    };
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Nhako Beam")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quick" => {
                let at = app.cursor_position().unwrap_or_default();
                popover::toggle(app, at);
            }
            "open" => show_window(app),
            "quit" => quit_app(app.clone()),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            // Windows: a click opens the popover next to the icon, a double click the window.
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } => popover::toggle(tray.app_handle(), position),
            TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
                popover::hide(tray.app_handle());
                show_window(tray.app_handle());
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Files dropped on a window: the window may now stage them (see `Host::offered`).
pub fn on_webview_event<R: Runtime>(webview: &tauri::Webview<R>, event: &tauri::WebviewEvent) {
    if let tauri::WebviewEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
        webview.state::<Host>().offer(paths.iter().cloned());
    }
}

pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if let WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
        window.state::<Host>().offer(paths.iter().cloned());
    }
    if window.label() == popover::LABEL {
        // Like any tray popover: it goes away when you click elsewhere.
        if matches!(event, WindowEvent::Focused(false)) {
            let _ = window.hide();
        }
        return;
    }
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

/// The main window or the tray popover is open (not hidden or minimized).
pub fn on_screen<R: Runtime>(app: &AppHandle<R>) -> bool {
    ["main", popover::LABEL]
        .iter()
        .filter_map(|l| app.get_webview_window(l))
        .any(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

fn looking<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// A notification with buttons, unless the window is open and focused. `on_action` gets the
/// button's id, or `default` when the notification itself is clicked (Linux). It waits on its own
/// thread until the notification is answered or closed.
pub fn notify_with_actions<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: &str,
    actions: &[(&str, &str)],
    on_action: impl FnOnce(&AppHandle<R>, &str) + Send + 'static,
) {
    if looking(app) {
        return;
    }
    #[cfg(any(windows, target_os = "linux"))]
    {
        let mut n = notify_rust::Notification::new();
        n.appname("Nhako Beam").summary(title).body(body).auto_icon();
        // The window plays Nhako Beam's own sound for it (sound.rs), so the desktop stays quiet.
        // Windows toasts are already silent without a sound name.
        #[cfg(target_os = "linux")]
        n.hint(notify_rust::Hint::SuppressSound(true));
        for (id, label) in actions {
            n.action(id, label);
        }
        // Clicking the notification itself (Linux; Windows has no separate event for it).
        n.action("default", "Open Nhako Beam");
        #[cfg(windows)]
        if installed() {
            n.app_id(&app.config().identifier);
        }
        let app = app.clone();
        std::thread::spawn(move || match n.show() {
            Ok(handle) => handle.wait_for_action(|action| on_action(&app, action)),
            Err(e) => warn!("couldn't show a notification: {e}"),
        });
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (actions, on_action);
        notify(app, title, body);
    }
}

/// Windows only shows toasts under the app's own name once it's installed (not from target/).
#[cfg(windows)]
fn installed() -> bool {
    let Ok(exe) = tauri::utils::platform::current_exe() else { return false };
    let dir = exe.parent().map(|d| d.display().to_string()).unwrap_or_default();
    !(dir.ends_with("\\target\\debug") || dir.ends_with("\\target\\release"))
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
pub fn take_send_paths(shell: State<'_, Shell>, host: State<'_, Host>) -> Vec<PathBuf> {
    let paths = std::mem::take(&mut *shell.send_queue.lock().expect("send queue lock"));
    host.offer(paths.iter().cloned());
    paths
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

/// Opens a web page in the default browser (the licenses screen links to each project).
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only web links can be opened.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
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
