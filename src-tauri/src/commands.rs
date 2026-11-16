//! Tauri commands exposed to the frontend.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::config::expand_tilde;
use crate::model::{Config, HistoryEntry, MachineSnapshot};
use crate::push;
use crate::scan;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BootState {
    pub config: Config,
    pub snapshot: Option<MachineSnapshot>,
    pub scanning: bool,
    pub machine_name: String,
    pub machine_id: String,
    pub git_available: bool,
    pub git_version: Option<String>,
    pub autostart: bool,
}

#[tauri::command]
pub fn get_boot(app: AppHandle) -> BootState {
    use tauri_plugin_autostart::ManagerExt;
    let state = app.state::<AppState>();
    let inner = state.0.lock().unwrap();
    BootState {
        config: inner.config.clone(),
        snapshot: inner.snapshot.clone(),
        scanning: inner.scanning,
        machine_name: scan::machine_name(),
        machine_id: inner.config.machine_id.clone(),
        git_available: inner.git_version.is_some(),
        git_version: inner.git_version.clone(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

#[tauri::command]
pub fn save_config(app: AppHandle, mut config: Config) -> Result<Config, String> {
    config.clamp();
    config.roots = normalize_list(&config.roots);
    config.extra_repos = normalize_list(&config.extra_repos);
    {
        let state = app.state::<AppState>();
        let mut inner = state.0.lock().unwrap();
        let path = inner.config_path.clone();
        inner.config = config.clone();
        drop(inner);
        config.save(&path)?;
    }
    Ok(config)
}

fn normalize_list(list: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in list {
        let expanded = expand_tilde(item);
        if !expanded.is_empty() && !out.contains(&expanded) {
            out.push(expanded);
        }
    }
    out
}

#[tauri::command]
pub fn start_scan(app: AppHandle) -> bool {
    scan::start_scan(&app)
}

#[tauri::command]
pub fn push_branch(
    app: AppHandle,
    path: String,
    branch: String,
    remote: Option<String>,
    set_upstream: bool,
) -> Result<(), String> {
    push::push_branch(&app, path, branch, remote, set_upstream)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fetch_repo(app: AppHandle, path: String) -> Result<(), String> {
    crate::fetch::fetch_repo(&app, path)
}

#[tauri::command]
pub fn fetch_all(app: AppHandle) {
    crate::fetch::fetch_all(&app)
}

#[tauri::command]
pub fn push_all(app: AppHandle) -> Result<(), String> {
    crate::push::push_all(&app)
}

#[tauri::command]
pub fn get_history(app: AppHandle) -> Vec<HistoryEntry> {
    let state = app.state::<AppState>();
    let path = state.0.lock().unwrap().history_path.clone();
    crate::history::read(&path, 200)
}

/// All machines known to the aggregation: own snapshot (fresh, from
/// state) + every `snapshot-*.json` found in the sync dir. Machines are
/// keyed by machine_id — synced duplicates of the own machine are skipped
/// in favor of the in-memory snapshot.
#[tauri::command]
pub fn get_machines(app: AppHandle) -> Vec<MachineSnapshot> {
    let (sync_dir, own) = {
        let state = app.state::<AppState>();
        let inner = state.0.lock().unwrap();
        (
            inner.config.sync_dir.as_ref().map(|d| expand_tilde(d)),
            inner.snapshot.clone(),
        )
    };

    let mut machines: Vec<MachineSnapshot> = Vec::new();
    if let Some(dir) = sync_dir {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.starts_with("snapshot-") || !name.ends_with(".json") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(snap) = serde_json::from_str::<MachineSnapshot>(&content) {
                        machines.push(snap);
                    }
                }
            }
        }
    }
    // Own machine: the in-memory snapshot wins over a possibly stale file.
    if let Some(own) = own {
        machines.retain(|m| m.machine_id != own.machine_id);
        machines.push(own);
    }
    machines.sort_by(|a, b| a.machine_name.cmp(&b.machine_name));
    machines
}
