//! Global application state guarded by a single mutex. All sections are
//! short-lived lock scopes; heavy work (scans, pushes) happens outside.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Manager};

use crate::gitbin;
use crate::model::{Config, MachineSnapshot};

pub struct Inner {
    pub config: Config,
    pub snapshot: Option<MachineSnapshot>,
    pub scanning: bool,
    /// Repositories with a push currently in flight.
    pub pushing: HashSet<String>,
    /// Repositories with a fetch currently in flight.
    pub fetching: HashSet<String>,
    /// True while the batch "push all" queue is running.
    pub push_all_running: bool,
    pub config_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub history_path: PathBuf,
    pub last_scan: Option<Instant>,
    pub git_version: Option<String>,
}

pub struct AppState(pub Mutex<Inner>);

impl AppState {
    pub fn init(app: &AppHandle) -> Result<Self, String> {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())?;
        let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        let config_path = config_dir.join("config.json");
        let snapshot_path = data_dir.join("snapshot.json");
        let history_path = data_dir.join("history.jsonl");
        let config = Config::load(&config_path);
        let snapshot = std::fs::read_to_string(&snapshot_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        Ok(Self(Mutex::new(Inner {
            config,
            snapshot,
            scanning: false,
            pushing: HashSet::new(),
            fetching: HashSet::new(),
            push_all_running: false,
            config_path,
            snapshot_path,
            history_path,
            last_scan: None,
            git_version: gitbin::detect(),
        })))
    }

    /// Persist the snapshot to disk — this file is the future
    /// cross-machine sync contract.
    pub fn persist_snapshot(&self, snapshot: &MachineSnapshot) {
        if let Ok(json) = serde_json::to_string_pretty(snapshot) {
            let _ = std::fs::write(&self.0.lock().unwrap().snapshot_path, json);
        }
    }
}
