//! Push runner: spawns `git push` in the repository, streams combined
//! output lines to the frontend as events, one push per repo at a time.
//! Also drives the batch "push all" queue.
//!
//! Auth is delegated entirely to the system git (ssh-agent, credential
//! helper) — identical behavior to pushing from a terminal, except that
//! interactive credential prompts are disabled so a push can never hang
//! the app: without `GIT_TERMINAL_PROMPT=0` a passphrase-less SSH key or
//! a GUI-context credential prompt would block the thread forever.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tauri::{AppHandle, Emitter, Manager};

use crate::gitbin;
use crate::model::{
    MachineSnapshot, PushAllDoneEvent, PushDoneEvent, PushLineEvent, PushQueueEvent, Problem,
};
use crate::state::AppState;

/// Build the exact argv for a push. Always pushes a named branch to a
/// named remote — a bare `git push` would only push the *current* branch
/// under `push.default=simple` and report success for any other branch.
/// `--` guards against branch names starting with `-`.
pub fn push_args(branch: &str, remote: Option<&str>, set_upstream: bool) -> Vec<String> {
    let mut args = vec!["push".to_string()];
    match remote.filter(|r| !r.trim().is_empty()) {
        Some(remote) => {
            if set_upstream {
                args.push("-u".to_string());
            }
            args.push(remote.to_string());
            args.push("--".to_string());
            args.push(branch.to_string());
        }
        // No remote known — fall back to default push (current branch).
        // The UI disables the button for remoteless repos anyway.
        None => {}
    }
    args
}

/// One queued push action derived from the snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct PushItem {
    pub path: String,
    pub branch: String,
    pub remote: Option<String>,
    pub set_upstream: bool,
}

/// Derive the push queue from a snapshot: every ahead/no-upstream/
/// upstream-gone branch gets an explicit push item. Diverged branches are
/// skipped — a plain push would be rejected non-fast-forward; those need
/// a pull first. Dirty-only/orphan-only repos have nothing to push.
pub fn collect_pushable(snapshot: &MachineSnapshot) -> Vec<PushItem> {
    let mut items = Vec::new();
    for repo in &snapshot.repos {
        if repo.state != crate::model::RepoState::Problem || repo.remotes.is_empty() {
            continue;
        }
        for problem in &repo.problems {
            let (branch, set_upstream) = match problem {
                Problem::Ahead { branch, .. } => (branch, false),
                Problem::NoUpstream { branch } => (branch, true),
                Problem::UpstreamGone { branch, .. } => (branch, true),
                // Diverged: pushing would be rejected; needs pull first.
                Problem::Diverged { .. } => continue,
                _ => continue,
            };
            let branch_info = repo.branches.iter().find(|b| &b.name == branch);
            let remote = branch_info
                .and_then(|b| b.remote.clone())
                .or_else(|| repo.remotes.first().cloned());
            if let Some(remote) = remote {
                items.push(PushItem {
                    path: repo.path.clone(),
                    branch: branch.clone(),
                    remote: Some(remote),
                    set_upstream,
                });
            }
        }
    }
    items
}

/// Run one push synchronously in `repo_path`, streaming lines to the UI.
/// Returns (ok, exit-code-like).
fn run_push(app: &AppHandle, repo_path: &Path, args: &[String], path_key: &str) -> (bool, i32) {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    gitbin::hide_console(&mut cmd);
    cmd.args(args);

    let mut outcome: Result<std::process::ExitStatus, String> =
        Err("не удалось запустить git".into());
    if let Ok(mut child) = cmd.spawn() {
        // stdout/stderr are piped; each stream is drained by its own
        // thread so git never blocks on a full pipe buffer.
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
                "push://line",
                PushLineEvent {
                    path: path_key.to_string(),
                    line: err.clone(),
                },
            );
            (false, -1)
        }
    }
}

fn spawn_line_stream<R: Read + Send + 'static>(
    stream: R,
    app: AppHandle,
    path: String,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let _ = app.emit("push://line", PushLineEvent { path: path.clone(), line });
        }
    })
}

/// Start a push for `path`. `set_upstream=true` adds `-u` (branches that
/// have never been pushed or whose upstream is gone). Returns immediately;
/// progress arrives as `push://line`, completion as `push://done` events.
pub fn push_branch(
    app: &AppHandle,
    path: String,
    branch: String,
    remote: Option<String>,
    set_upstream: bool,
) -> Result<(), String> {
    let repo_path = PathBuf::from(&path);
    if !repo_path.is_dir() {
        return Err("путь не существует".into());
    }
    {
        let state = app.state::<AppState>();
        let mut inner = state.0.lock().unwrap();
        if inner.pushing.contains(&path) || inner.push_all_running {
            return Err("push для этого репозитория уже выполняется".into());
        }
        inner.pushing.insert(path.clone());
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let args = push_args(&branch, remote.as_deref(), set_upstream);
        let (ok, code) = run_push(&app, &repo_path, &args, &path);
        let _ = app.emit("push://done", PushDoneEvent { path: path.clone(), ok, code });
        crate::history::append(
            &app.state::<AppState>().0.lock().unwrap().history_path,
            "push",
            &format!(
                "{}: {} → {}{}",
                Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone()),
                branch,
                remote.as_deref().unwrap_or("?"),
                if set_upstream { " (-u)" } else { "" }
            ),
            Some(ok),
        );
        let state = app.state::<AppState>();
        state.0.lock().unwrap().pushing.remove(&path);
    });

    Ok(())
}

/// Batch "push all": derive the queue from the latest snapshot and run it
/// sequentially (one remote operation at a time). Progress arrives as
/// `push://queue` per item, summary as `push://all_done`, then a rescan.
pub fn push_all(app: &AppHandle) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut inner = state.0.lock().unwrap();
        if inner.push_all_running {
            return Err("push all уже выполняется".into());
        }
        let queue = inner
            .snapshot
            .as_ref()
            .map(collect_pushable)
            .unwrap_or_default();
        if queue.is_empty() {
            return Err("нечего пушить — все ветки отправлены".into());
        }
        inner.push_all_running = true;
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let (queue, history_path) = {
            let state = app.state::<AppState>();
            let inner = state.0.lock().unwrap();
            (
                inner.snapshot.as_ref().map(collect_pushable).unwrap_or_default(),
                inner.history_path.clone(),
            )
        };
        let total = queue.len();
        let mut pushed = 0usize;
        let mut failed = 0usize;

        for (i, item) in queue.iter().enumerate() {
            // Skip repos with a user-initiated push in flight.
            let busy = {
                let state = app.state::<AppState>();
                let mut inner = state.0.lock().unwrap();
                let busy = inner.pushing.contains(&item.path);
                if !busy {
                    inner.pushing.insert(item.path.clone());
                }
                busy
            };
            if busy {
                failed += 1;
                continue;
            }

            let name = Path::new(&item.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| item.path.clone());
            let _ = app.emit(
                "push://queue",
                PushQueueEvent {
                    current: i + 1,
                    total,
                    path: item.path.clone(),
                    name: name.clone(),
                },
            );

            let args = push_args(&item.branch, item.remote.as_deref(), item.set_upstream);
            let repo_path = PathBuf::from(&item.path);
            let (ok, _) = run_push(&app, &repo_path, &args, &item.path);
            if ok {
                pushed += 1;
            } else {
                failed += 1;
            }
            crate::history::append(
                &history_path,
                "push",
                &format!(
                    "{}: {} → {}{} (batch)",
                    name,
                    item.branch,
                    item.remote.as_deref().unwrap_or("?"),
                    if item.set_upstream { " (-u)" } else { "" }
                ),
                Some(ok),
            );
            let _ = app.emit(
                "push://done",
                PushDoneEvent {
                    path: item.path.clone(),
                    ok,
                    code: if ok { 0 } else { -1 },
                },
            );

            let state = app.state::<AppState>();
            state.0.lock().unwrap().pushing.remove(&item.path);
        }

        let _ = app.emit(
            "push://all_done",
            PushAllDoneEvent {
                pushed,
                failed,
                total,
            },
        );
        crate::history::append(
            &history_path,
            "push",
            &format!("batch push: отправлено {pushed} из {total}, ошибок: {failed}"),
            Some(failed == 0),
        );

        let state = app.state::<AppState>();
        state.0.lock().unwrap().push_all_running = false;
        crate::scan::start_scan(&app);
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BranchInfo, Problem, RepoState, RepoStatus};

    fn repo(path: &str, state: RepoState, problems: Vec<Problem>, branches: Vec<BranchInfo>, remotes: Vec<String>) -> RepoStatus {
        RepoStatus {
            path: path.to_string(),
            name: path.to_string(),
            state,
            problems,
            branches,
            remotes,
            dirty_files: Vec::new(),
            dirty_files_total: 0,
            current_branch: None,
            detached: false,
            stashes: 0,
            scanned_at: None,
            error: None,
        }
    }

    fn branch(name: &str, remote: Option<&str>) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            upstream: remote.map(|r| format!("{r}/{name}")),
            remote: remote.map(|r| r.to_string()),
            ahead: 0,
            behind: 0,
            gone: false,
            is_head: false,
        }
    }

    fn snapshot(repos: Vec<RepoStatus>) -> MachineSnapshot {
        MachineSnapshot {
            machine_id: "t".into(),
            machine_name: "t".into(),
            generated_at: chrono::Utc::now(),
            repos,
        }
    }

    #[test]
    fn ahead_no_upstream_and_gone_are_queued() {
        let snap = snapshot(vec![
            repo(
                "/r/a",
                RepoState::Problem,
                vec![Problem::Ahead {
                    branch: "main".into(),
                    upstream: "origin/main".into(),
                    ahead: 2,
                }],
                vec![branch("main", Some("origin"))],
                vec!["origin".into()],
            ),
            repo(
                "/r/b",
                RepoState::Problem,
                vec![Problem::NoUpstream { branch: "feature".into() }],
                vec![branch("feature", None)],
                vec!["gitlab".into()],
            ),
            repo(
                "/r/c",
                RepoState::Problem,
                vec![Problem::UpstreamGone {
                    branch: "old".into(),
                    upstream: "origin/old".into(),
                }],
                vec![branch("old", Some("origin"))],
                vec!["origin".into()],
            ),
        ]);
        let items = collect_pushable(&snap);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], PushItem {
            path: "/r/a".into(),
            branch: "main".into(),
            remote: Some("origin".into()),
            set_upstream: false,
        });
        // No upstream → -u and first configured remote.
        assert_eq!(items[1], PushItem {
            path: "/r/b".into(),
            branch: "feature".into(),
            remote: Some("gitlab".into()),
            set_upstream: true,
        });
        assert_eq!(items[2], PushItem {
            path: "/r/c".into(),
            branch: "old".into(),
            remote: Some("origin".into()),
            set_upstream: true,
        });
    }

    #[test]
    fn diverged_dirty_orphan_and_ok_are_not_queued() {
        let snap = snapshot(vec![
            repo(
                "/r/div",
                RepoState::Problem,
                vec![Problem::Diverged {
                    branch: "main".into(),
                    upstream: "origin/main".into(),
                    ahead: 1,
                    behind: 3,
                }],
                vec![branch("main", Some("origin"))],
                vec!["origin".into()],
            ),
            repo(
                "/r/dirty",
                RepoState::Problem,
                vec![Problem::Dirty {
                    staged: 0,
                    unstaged: 2,
                    untracked: 0,
                    conflicts: 0,
                }],
                vec![branch("main", Some("origin"))],
                vec!["origin".into()],
            ),
            repo(
                "/r/orph",
                RepoState::Problem,
                vec![Problem::Orphan { count: 1 }],
                vec![branch("main", Some("origin"))],
                vec!["origin".into()],
            ),
            repo("/r/ok", RepoState::Ok, vec![], vec![], vec!["origin".into()]),
        ]);
        assert!(collect_pushable(&snap).is_empty());
    }

    #[test]
    fn remoteless_repos_are_not_queued() {
        let snap = snapshot(vec![repo(
            "/r/norem",
            RepoState::Problem,
            vec![Problem::NoRemote { branches: 1 }],
            vec![branch("main", None)],
            vec![],
        )]);
        assert!(collect_pushable(&snap).is_empty());
    }
}
