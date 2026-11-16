//! Tray icon + menu. Icon color encodes severity:
//! green — everything pushed; yellow — only dirty/no-upstream repos;
//! red — repos with actually unpushed commits; gray — errors/unknown.

use std::sync::OnceLock;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

use crate::model::{MachineSnapshot, RepoState};
use crate::scan;

const TRAY_ID: &str = "pushwatch-tray";
static AUTOSTART_ITEM: OnceLock<CheckMenuItem<tauri::Wry>> = OnceLock::new();

pub fn build(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "pw-open", "Открыть pushwatch", true, None::<&str>)?;
    let scan_item = MenuItem::with_id(app, "pw-scan", "Сканировать сейчас", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let autostart_enabled = autostart_enabled(app.handle());
    let autostart = CheckMenuItem::with_id(
        app,
        "pw-autostart",
        "Запускать при входе",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let _ = AUTOSTART_ITEM.set(autostart.clone());
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "pw-quit", "Выход", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open, &scan_item, &sep1, &autostart, &sep2, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon(circle_icon(113, 113, 122))
        .tooltip("pushwatch")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "pw-open" => show_main_window(app),
            "pw-scan" => {
                scan::start_scan(app);
            }
            "pw-autostart" => toggle_autostart(app),
            "pw-quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn autostart_enabled(app: &AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

fn toggle_autostart(app: &AppHandle) {
    use tauri_plugin_autostart::ManagerExt;
    let enable = !autostart_enabled(app);
    let result = if enable {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    if result.is_ok() {
        if let Some(item) = AUTOSTART_ITEM.get() {
            let _ = item.set_checked(enable);
        }
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Refresh the tray icon/tooltip from the latest snapshot.
pub fn update(app: &AppHandle, snapshot: Option<&MachineSnapshot>, git_missing: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    let color = if git_missing {
        (113, 113, 122) // gray
    } else if let Some(s) = snapshot {
        if s.problem_repo_count() > 0 {
            if s.has_unpushed_commits() {
                (239, 68, 68) // red: commits exist that no remote has
            } else {
                (234, 179, 8) // yellow: dirty/unpushed-branch noise only
            }
        } else {
            match s.worst_repo_state() {
                Some(RepoState::Error) => (113, 113, 122),
                _ => (34, 197, 94), // green
            }
        }
    } else {
        (113, 113, 122)
    };
    let _ = tray.set_icon(Some(circle_icon(color.0, color.1, color.2)));

    let problems = snapshot.map(|s| s.problem_repo_count()).unwrap_or(0);
    let total = snapshot.map(|s| s.repos.len()).unwrap_or(0);
    let _ = tray.set_tooltip(Some(format!("pushwatch: {problems} проблемных из {total}")));

    // Menu-bar text with the problem count; macOS-only tray feature.
    #[cfg(target_os = "macos")]
    {
        let _ = tray.set_title(if problems > 0 { Some(format!("{problems}")) } else { None });
    }
}

/// Antialiased filled-circle RGBA icon generated in-process — no icon
/// files needed and colors adapt to state at runtime. 44×44 is the
/// @2x size Tauri recommends for macOS menu-bar icons.
fn circle_icon(r: u8, g: u8, b: u8) -> Image<'static> {
    const SIZE: usize = 44;
    let center = (SIZE as f32 - 1.0) / 2.0;
    let radius = 18.0;
    let aa = 1.2f32;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = ((radius + aa / 2.0 - dist) / aa).clamp(0.0, 1.0);
            let i = (y * SIZE + x) * 4;
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = (alpha * 255.0) as u8;
        }
    }
    Image::new_owned(rgba, SIZE as u32, SIZE as u32)
}
