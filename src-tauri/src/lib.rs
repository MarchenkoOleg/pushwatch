mod commands;
mod config;
mod engine;
mod fetch;
mod gitbin;
mod history;
mod model;
mod parsers;
mod push;
mod scan;
mod scanner;
mod state;
mod tray;

use std::time::Duration;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

/// Background scheduler: every 20 s check whether the configured scan
/// interval has elapsed and kick a scan if so.
fn spawn_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(20));
        let due = {
            let state = app.state::<state::AppState>();
            let inner = state.0.lock().unwrap();
            if inner.scanning {
                continue;
            }
            inner
                .last_scan
                .map_or(true, |t| t.elapsed().as_secs() >= inner.config.scan_interval_secs)
        };
        if due {
            scan::start_scan(&app);
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single instance must be registered first: a second launch
        // (double-click etc.) just reveals the existing window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Menu-bar (tray) app: no dock icon on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let state = state::AppState::init(app.handle())
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            app.manage(state);

            tray::build(app)?;

            // The window is created invisible (tauri.conf.json) to avoid a
            // flash when start_hidden is on; show it here for normal runs.
            if !app
                .state::<state::AppState>()
                .0
                .lock()
                .unwrap()
                .config
                .start_hidden
            {
                tray::show_main_window(app.handle());
            }

            scan::start_scan(app.handle());
            spawn_scheduler(app.handle().clone());
            scan::spawn_watcher(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_boot,
            commands::save_config,
            commands::start_scan,
            commands::push_branch,
            commands::set_autostart,
            commands::reveal_path,
            commands::fetch_repo,
            commands::fetch_all,
            commands::push_all,
            commands::get_history,
            commands::get_machines,
        ])
        .on_window_event(|window, event| {
            // Closing the window keeps the app in the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
