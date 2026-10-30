//! The tray popover (360 x 520): a compact home next to the tray icon with your devices, what's
//! moving, and "Open Nhako Beam". docs/design/screens-and-states.md, section 9.1.
//!
//! Windows opens it with a left click on the tray icon. Linux panels don't report tray clicks, so
//! it's the first item in the tray menu there, and it opens next to the pointer. "Send with Nhako
//! Beam" in the file manager opens it too while the main window is closed, to pick a device. It's the same
//! Svelte app (`index.html?view=popover`), on the same engine events as the main window.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindowBuilder};
use tracing::warn;

use crate::shell;

pub const LABEL: &str = "popover";
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 520.0;
/// Space between the popover and the tray icon or the screen edge.
const GAP: i32 = 12;

/// Opens the popover next to `at` (the pointer, in physical pixels), or hides it if it's showing.
pub fn toggle<R: Runtime>(app: &AppHandle<R>, at: PhysicalPosition<f64>) {
    if let Some(w) = app.get_webview_window(LABEL)
        && w.is_visible().unwrap_or(false)
    {
        let _ = w.hide();
        return;
    }
    show(app, at);
}

/// Opens the popover next to `at`, or moves it there if it's open already.
pub fn show<R: Runtime>(app: &AppHandle<R>, at: PhysicalPosition<f64>) {
    let window = match app.get_webview_window(LABEL) {
        Some(w) => w,
        None => match WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html?view=popover".into()))
            .title("Nhako Beam")
            .inner_size(WIDTH, HEIGHT)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .decorations(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                warn!("couldn't open the tray popover: {e}");
                shell::show_window(app);
                return;
            }
        },
    };
    if let Ok(Some(monitor)) = app.monitor_from_point(at.x, at.y) {
        let scale = monitor.scale_factor();
        let (w, h) = ((WIDTH * scale) as i32, (HEIGHT * scale) as i32);
        let area = monitor.work_area();
        let (left, top) = (area.position.x, area.position.y);
        let (right, bottom) = (left + area.size.width as i32, top + area.size.height as i32);
        let gap = (GAP as f64 * scale) as i32;
        let (x, y) = place(at.x as i32, at.y as i32, (w, h), (left, top, right, bottom), gap);
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    let _ = window.show();
    let _ = window.set_focus();
}

/// Where the popover's top-left corner goes: centred on the pointer, above it when the pointer is
/// in the lower half of the screen (a taskbar at the bottom), below it otherwise, and never off
/// the usable area.
fn place(px: i32, py: i32, (w, h): (i32, i32), (left, top, right, bottom): (i32, i32, i32, i32), gap: i32) -> (i32, i32) {
    let x = (px - w / 2).clamp(left + gap, (right - w - gap).max(left + gap));
    let below = py < (top + bottom) / 2;
    let y = if below { py + gap } else { py - h - gap };
    (x, y.clamp(top + gap, (bottom - h - gap).max(top + gap)))
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
}

/// "Open Nhako Beam" in the popover.
#[tauri::command]
pub fn open_main(app: AppHandle) {
    hide(&app);
    shell::show_window(&app);
}

/// Files dropped on the popover but not on a device: the main window asks where to send them.
#[tauri::command]
pub fn hand_to_main(app: AppHandle, paths: Vec<PathBuf>) {
    hide(&app);
    app.state::<shell::Shell>().queue_send(&app, paths, shell::MAIN);
    shell::show_window(&app);
}

#[cfg(test)]
mod tests {
    use super::place;

    #[test]
    fn sits_next_to_the_pointer_and_stays_on_screen() {
        let screen = (0, 0, 1920, 1040); // a 40 px taskbar at the bottom
        let size = (360, 520);
        // Taskbar at the bottom right: above the pointer, pulled in from the right edge.
        assert_eq!(place(1880, 1060, size, screen, 12), (1548, 508));
        // A panel at the top (GNOME, KDE at the top): below the pointer.
        assert_eq!(place(960, 10, size, screen, 12), (780, 22));
        // Near the left edge.
        assert_eq!(place(5, 1000, size, screen, 12), (12, 468));
    }
}
