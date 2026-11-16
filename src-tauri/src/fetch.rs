//! Fetch runner: `git fetch --all --prune` per repository or for every
//! discovered repo. Fetches are network operations — failures are reported
//! per-repo and never break the scan.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter, Manager};

use crate::gitbin;
use crate::model::{FetchAllDoneEvent, FetchDoneEvent, PushLineEvent};
use crate::state::AppState;

pub fn fetch_args() -> [&'static str; 3] {
    ["fetch", "--all", "--prune"]
}

fn spawn_line_stream<R: Read + Send + 'static>(
    stream: R,
    app: AppHandle,
    path: String,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let _ = app.emit("fetch://line", PushLineEvent { path: path.clone(), line });
        }
    })
}

/// Run fetch synchronously in `repo_path`, streaming lines to the UI.
fn run_fetch(app: &AppHandle, repo_path: &Path, path_key: &str) -> (bool, i32) {
    let lang = {
        let state = app.state::<AppState>();
        let inner = state.0.lock().unwrap();
        inner.config.lang
    };
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    gitbin::hide_console(&mut cmd);
    cmd.args(fetch_args());

    let mut outcome: Result<std::process::ExitStatus, String> =
        Err(crate::lang::tr(lang).err_spawn_git.into());
    if let Ok(mut child) = cmd.spawn() {
        let out_thread = child
            .stdout
            .take()
            .map(|s| spawn_line_stream(s, app.clone(), path_key.to_string()));
        let err_thread = child
            .stderr
            .take()
            .map(|s| spawn_line_stream(s, app.clone(), path_key.to_string()));
        let wait = child.wait().map_err(|e| e.to_string());
        if let Some(h) = out_thread {
            let _ = h.join();
        }
        if let Some(h) = err_thread {
            let _ = h.join();
        }
        outcome = wait;
    }

    match &outcome {
        Ok(status) => (status.success(), status.code().unwrap_or(-1)),
        Err(err) => {
            let _ = app.emit(
                "fetch://line",
                PushLineEvent {
                    path: path_key.to_string(),
                    line: err.clone(),
                },
            );
            (false, -1)
        }
    }
}

/// Synchronous bounded-concurrency fetch pass over the given repos.
/// Returns the number of failed fetches.
pub fn run_fetch_pass(app: &AppHandle, paths: &[String]) -> usize {
    if paths.is_empty() {
        return 0;
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("fetch pool");
    let failed = std::sync::atomic::AtomicUsize::new(0);
    pool.install(|| {
        paths.par_iter().for_each(|path| {
            let repo_path = PathBuf::from(path);
            let (ok, _) = run_fetch(app, &repo_path, path);
            if !ok {
                failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            let _ = app.emit("fetch://done", FetchDoneEvent { path: path.clone(), ok });
        });
    });
    failed.load(std::sync::atomic::Ordering::Relaxed)
}

/// Fetch every discovered repository, then trigger a rescan. Runs on a
/// background thread; per-repo results arrive as `fetch://done`, the
/// summary as `fetch://all_done`.
pub fn fetch_all(app: &AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        let repo_paths: Vec<String> = {
            let state = handle.state::<AppState>();
            let inner = state.0.lock().unwrap();
            inner
                .snapshot
                .as_ref()
                .map(|s| s.repos.iter().map(|r| r.path.clone()).collect())
                .unwrap_or_default()
        };
        if repo_paths.is_empty() {
            return;
        }

        let total = repo_paths.len();
        let failed = run_fetch_pass(&handle, &repo_paths);
        let _ = handle.emit(
            "fetch://all_done",
            FetchAllDoneEvent { total, failed },
        );
        crate::history::append(
            &handle.state::<AppState>().0.lock().unwrap().history_path,
            "fetch",
            &crate::lang::hist_fetch_all(
                {
                    let state = handle.state::<AppState>();
                    state.0.lock().unwrap().config.lang
                },
                total,
                failed,
            ),
            Some(failed == 0),
        );
        // Fresh remote refs → refresh statuses right away.
        crate::scan::start_scan(&handle);
    });
}

/// Fetch a single repository (async: spawns a thread, emits `fetch://done`).
pub fn fetch_repo(app: &AppHandle, path: String) -> Result<(), String> {
    let repo_path = PathBuf::from(&path);
    let s = crate::lang::tr({
        let state = app.state::<AppState>();
        let inner = state.0.lock().unwrap();
        inner.config.lang
    });
    if !repo_path.is_dir() {
        return Err(s.err_path_missing.into());
    }
    {
        let state = app.state::<AppState>();
        let mut inner = state.0.lock().unwrap();
        if inner.fetching.contains(&path) {
            return Err(s.err_fetch_busy.into());
        }
        inner.fetching.insert(path.clone());
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let (ok, _) = run_fetch(&app, &repo_path, &path);
        let _ = app.emit("fetch://done", FetchDoneEvent { path: path.clone(), ok });
        let state = app.state::<AppState>();
        state.0.lock().unwrap().fetching.remove(&path);
    });
    Ok(())
}
