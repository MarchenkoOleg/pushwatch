//! Scan orchestration: (optional fetch) → discovery → parallel status
//! collection → snapshot persist (+sync dir) → notifications → tray update.
//! Results stream to the UI per-repo as they finish.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use rayon::prelude::*;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::expand_tilde;
use crate::engine;
use crate::fetch;
use crate::model::{MachineSnapshot, ScanDoneEvent, ScanRepoEvent};
use crate::scanner;
use crate::state::AppState;
use crate::tray;
use notify::Watcher;

/// Start a background scan unless one is already running.
/// Returns `false` if a scan is in progress.
pub fn start_scan(app: &AppHandle) -> bool {
    {
        let state = app.state::<AppState>();
        let mut inner = state.0.lock().unwrap();
        if inner.scanning {
            return false;
        }
        inner.scanning = true;
    }
    let _ = app.emit("scan://start", ());
    let handle = app.clone();
    std::thread::spawn(move || run_scan(handle));
    true
}

pub fn machine_name() -> String {
    let host = gethostname::gethostname().to_string_lossy().into_owned();
    if host.trim().is_empty() {
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown-machine".into())
    } else {
        host
    }
}

fn run_scan(app: AppHandle) {
    let started = Instant::now();
    let (config, git_available) = {
        let state = app.state::<AppState>();
        let inner = state.0.lock().unwrap();
        (inner.config.clone(), inner.git_version.is_some())
    };
    let machine_name = machine_name();

    // Optional network pass: with auto_fetch the background scan refreshes
    // remote-tracking refs first, so ahead/behind are real.
    if config.auto_fetch && git_available {
        let (pre_paths, _) = scanner::discover(&config);
        let paths: Vec<String> = pre_paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        fetch::run_fetch_pass(&app, &paths);
    }

    let (repo_paths, warnings) = scanner::discover(&config);
    let results = Mutex::new(Vec::with_capacity(repo_paths.len()));
    let emit_app = app.clone();
    repo_paths.par_iter().for_each(|p| {
        let status = engine::collect_repo(p);
        results.lock().unwrap().push(status.clone());
        let _ = emit_app.emit("scan://repo", ScanRepoEvent { status });
    });
    let mut repos = results.into_inner().unwrap();
    repos.sort_by(|a, b| a.path.cmp(&b.path));

    let prev_unpushed = {
        let state = app.state::<AppState>();
        let inner = state.0.lock().unwrap();
        inner.snapshot.as_ref().map(|s| s.unpushed_repo_count()).unwrap_or(0)
    };

    let snapshot = MachineSnapshot {
        machine_id: config.machine_id.clone(),
        machine_name,
        generated_at: Utc::now(),
        repos,
    };

    {
        let state = app.state::<AppState>();
        {
            let mut inner = state.0.lock().unwrap();
            inner.snapshot = Some(snapshot.clone());
            inner.scanning = false;
            inner.last_scan = Some(Instant::now());
        }
        state.persist_snapshot(&snapshot);
    }

    let _ = app.emit(
        "scan://done",
        ScanDoneEvent {
            generated_at: snapshot.generated_at,
            repo_count: snapshot.repos.len(),
            problem_count: snapshot.problem_repo_count(),
            paths: snapshot.repos.iter().map(|r| r.path.clone()).collect(),
            warnings,
        },
    );
    tray::update(&app, Some(&snapshot), !git_available);

    let elapsed = started.elapsed().as_secs();
    {
        let state = app.state::<AppState>();
        let history_path = state.0.lock().unwrap().history_path.clone();
        crate::history::append(
            &history_path,
            "scan",
            &crate::lang::hist_scan(
                config.lang,
                snapshot.repos.len(),
                snapshot.problem_repo_count(),
                snapshot.unpushed_repo_count(),
                elapsed,
            ),
            None,
        );
    }
    if config.sync_dir.is_some() {
        write_sync_snapshot(&app, &snapshot);
    }
    if config.notify_on_problems {
        notify_if_changed(&app, prev_unpushed, &snapshot, config.lang);
    }
}

/// Push this machine's snapshot into the shared sync folder — the
/// portable contract for the future/actual cross-machine aggregation.
fn write_sync_snapshot(app: &AppHandle, snapshot: &MachineSnapshot) {
    let sync_dir = {
        let state = app.state::<AppState>();
        let inner = state.0.lock().unwrap();
        inner.config.sync_dir.as_ref().map(|d| expand_tilde(d))
    };
    let Some(dir) = sync_dir else { return };
    let dir_path = std::path::PathBuf::from(dir);
    if dir_path.as_os_str().is_empty() {
        return;
    }
    if std::fs::create_dir_all(&dir_path).is_err() {
        return;
    }
    let file = dir_path.join(format!("snapshot-{}.json", snapshot.machine_id));
    if let Ok(json) = serde_json::to_string_pretty(snapshot) {
        // Write-through-temp-then-rename keeps readers (other machines'
        // aggregators) from ever seeing a partial file.
        let tmp = dir_path.join(format!(".tmp-snapshot-{}", snapshot.machine_id));
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &file);
        }
    }
}

/// System notification on transitions of the unpushed-repo count.
fn notify_if_changed(app: &AppHandle, prev: usize, snapshot: &MachineSnapshot, lang: crate::model::Lang) {
    use tauri_plugin_notification::NotificationExt;
    let now = snapshot.unpushed_repo_count();
    let body = if now > prev {
        let names: Vec<String> = snapshot
            .repos
            .iter()
            .filter(|r| {
                r.problems.iter().any(|p| {
                    matches!(
                        p,
                        crate::model::Problem::Ahead { .. }
                            | crate::model::Problem::Diverged { .. }
                            | crate::model::Problem::Orphan { .. }
                            | crate::model::Problem::UpstreamGone { .. }
                            | crate::model::Problem::NoRemote { .. }
                    )
                })
            })
            .map(|r| r.name.clone())
            .take(3)
            .collect();
        crate::lang::notify_unpushed(lang, now, &names.join(", "))
    } else if prev > 0 && now == 0 {
        crate::lang::tr(lang).notify_all_pushed.to_string()
    } else {
        return;
    };
    let _ = app
        .notification()
        .builder()
        .title("pushwatch")
        .body(&body)
        .show();
}

/// Background watcher: react to `.git` changes on disk (commits, branches,
/// checkouts) with a short debounce, instead of waiting for the next timer
/// tick. The watcher is rebuilt each cycle so config changes are picked up.
pub fn spawn_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        let roots: Option<Vec<String>> = {
            let state = app.state::<AppState>();
            let inner = state.0.lock().unwrap();
            if !inner.config.watch_fs {
                None
            } else {
                let mut all = inner.config.roots.clone();
                all.extend(inner.config.extra_repos.iter().cloned());
                Some(all)
            }
        };
        let Some(roots) = roots else {
            std::thread::sleep(Duration::from_secs(30));
            continue;
        };

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(_) => {
                std::thread::sleep(Duration::from_secs(30));
                continue;
            }
        };
        for root in &roots {
            let path = std::path::PathBuf::from(expand_tilde(root));
            if path.is_dir() {
                let _ = watcher.watch(&path, notify::RecursiveMode::Recursive);
            }
        }

        // Collect events until the stream has been quiet for 3 s, a scan
        // is due, or the cycle deadline passes (rebuild picks up config).
        let deadline = Instant::now() + Duration::from_secs(120);
        let mut pending = false;
        let mut last_relevant = Instant::now();
        loop {
            if pending && last_relevant.elapsed() >= Duration::from_secs(3) {
                break;
            }
            if !pending && Instant::now() >= deadline {
                break;
            }
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Ok(event)) => {
                    if event.paths.iter().any(|p| {
                        p.components().any(|c| c.as_os_str().to_string_lossy() == ".git")
                    }) {
                        pending = true;
                        last_relevant = Instant::now();
                    }
                }
                Ok(Err(_)) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(watcher);

        if pending {
            start_scan(&app);
            // Let the scan settle before watching again.
            std::thread::sleep(Duration::from_secs(5));
        }
    });
}
