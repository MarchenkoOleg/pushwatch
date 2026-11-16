//! Data model. Everything here is serde-serializable because the whole
//! `MachineSnapshot` is the portable contract for the future cross-machine
//! aggregation: v1 writes it to disk, later machines can sync these files.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One local branch and its relation to the upstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BranchInfo {
    pub name: String,
    /// Short upstream name, e.g. `origin/main`. `None` — branch never pushed.
    pub upstream: Option<String>,
    /// Exact remote name the branch pushes to (`%(upstream:remotename)`).
    pub remote: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// Upstream is configured but the remote branch no longer exists.
    pub gone: bool,
    /// Currently checked out branch (for-each-ref `%(HEAD)` marker).
    pub is_head: bool,
}

/// Counters of the working copy (from `git status --porcelain=v2`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StatusCounts {
    pub staged: u32,
    pub unstaged: u32,
    pub untracked: u32,
    pub conflicts: u32,
}

impl StatusCounts {
    pub fn total(&self) -> u32 {
        self.staged + self.unstaged + self.untracked + self.conflicts
    }
}

/// Everything the app considers "not safe yet" about one repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Problem {
    /// Working copy has staged/unstaged/untracked/conflicted files.
    Dirty {
        staged: u32,
        unstaged: u32,
        untracked: u32,
        conflicts: u32,
    },
    /// Local branch has commits the upstream does not have.
    Ahead {
        branch: String,
        upstream: String,
        ahead: u32,
    },
    /// Branch has unpushed commits AND the upstream moved on — a plain
    /// push will be rejected; needs pull --rebase (or merge) first.
    Diverged {
        branch: String,
        upstream: String,
        ahead: u32,
        behind: u32,
    },
    /// Local branch was never pushed (upstream not set) while remotes exist.
    NoUpstream { branch: String },
    /// No remote configured at all: the whole repository exists only here.
    NoRemote { branches: u32 },
    /// Upstream configured, but the remote branch was deleted.
    UpstreamGone { branch: String, upstream: String },
    /// Commits reachable from HEAD but from no remote ref at all.
    /// Catches detached HEAD leftovers and commits on top of force-pushes.
    Orphan { count: u32 },
    /// HEAD is not on any branch.
    DetachedHead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoState {
    Ok,
    Problem,
    Error,
}

/// One entry of the dirty worktree: XY status letters + path
/// (from `git status --porcelain=v2`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DirtyFile {
    /// Two-letter status, e.g. `M `, ` M`, `MM`, `??`, `UU`, `R `.
    pub label: String,
    pub path: String,
}

/// Status of a single repository at scan time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepoStatus {
    pub path: String,
    pub name: String,
    pub state: RepoState,
    pub problems: Vec<Problem>,
    pub branches: Vec<BranchInfo>,
    /// Configured remote names (`git remote`) — used by the Push action
    /// to pick the target remote for branches without upstream.
    pub remotes: Vec<String>,
    /// Dirty worktree entries (capped, see `dirty_files_total`).
    pub dirty_files: Vec<DirtyFile>,
    /// Full count of dirty entries (>= dirty_files.len() when capped).
    pub dirty_files_total: u32,
    pub current_branch: Option<String>,
    pub detached: bool,
    pub stashes: u32,
    /// `None` = repo discovered but not scanned yet in this session.
    pub scanned_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Portable per-machine snapshot. This file is the future sync contract:
/// sync the directory of snapshots (Syncthing/iCloud/private repo) and any
/// aggregator can show the state of all machines without re-scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MachineSnapshot {
    pub machine_id: String,
    pub machine_name: String,
    pub generated_at: DateTime<Utc>,
    pub repos: Vec<RepoStatus>,
}

impl MachineSnapshot {
    /// Number of repositories in a problematic state.
    pub fn problem_repo_count(&self) -> usize {
        self.repos.iter().filter(|r| r.state == RepoState::Problem).count()
    }

    /// Repositories with actually unpushed commits (ahead/diverged/orphan/
    /// upstream-gone/no-remote) — drives notifications and severity.
    pub fn unpushed_repo_count(&self) -> usize {
        self.repos
            .iter()
            .filter(|r| {
                r.problems.iter().any(|p| {
                    matches!(
                        p,
                        Problem::Ahead { .. }
                            | Problem::Diverged { .. }
                            | Problem::Orphan { .. }
                            | Problem::UpstreamGone { .. }
                            | Problem::NoRemote { .. }
                    )
                })
            })
            .count()
    }

    /// Repositories with actually unpushed commits (ahead/diverged/orphan/
    /// no-remote) — the "red" severity. Dirty/no-upstream-only repos are "yellow".
    pub fn has_unpushed_commits(&self) -> bool {
        self.repos.iter().any(|r| {
            r.problems.iter().any(|p| {
                matches!(
                    p,
                    Problem::Ahead { .. }
                        | Problem::Diverged { .. }
                        | Problem::Orphan { .. }
                        | Problem::UpstreamGone { .. }
                        | Problem::NoRemote { .. }
                )
            })
        })
    }

    pub fn worst_repo_state(&self) -> Option<RepoState> {
        if self.repos.iter().any(|r| r.state == RepoState::Error) {
            return Some(RepoState::Error);
        }
        if self.problem_repo_count() > 0 {
            return Some(RepoState::Problem);
        }
        if self.repos.iter().any(|r| r.state == RepoState::Ok) {
            return Some(RepoState::Ok);
        }
        None
    }
}

/// UI theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

/// UI language preference. `Auto` follows the OS locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    #[default]
    Auto,
    Ru,
    En,
}

/// App settings, persisted as JSON in the OS config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct Config {
    /// Stable per-machine id, generated on first run.
    pub machine_id: String,
    /// Directories to scan recursively for repositories.
    pub roots: Vec<String>,
    /// Directory names pruned during the scan (never descended into).
    pub exclude_dirs: Vec<String>,
    /// Manually added repositories outside the roots.
    pub extra_repos: Vec<String>,
    /// Discovered repositories the user chose to hide.
    pub hidden_repos: Vec<String>,
    /// Background scan period in seconds.
    pub scan_interval_secs: u64,
    /// Max directory depth below each root.
    pub max_depth: u32,
    /// Start with the window hidden (tray-only).
    pub start_hidden: bool,
    /// UI theme: follow the OS (auto) or force light/dark.
    pub theme: Theme,
    /// UI language: follow the OS locale (auto) or force ru/en.
    pub lang: Lang,
    /// Shared folder for cross-machine snapshots (Syncthing/iCloud/private
    /// git repo). `None` — aggregation disabled, local-only mode.
    pub sync_dir: Option<String>,
    /// Fetch all remotes before every background scan (network traffic;
    /// gives true ahead/behind and UpstreamGone accuracy).
    pub auto_fetch: bool,
    /// React to `.git` changes on disk (commits, branches, checkouts) and
    /// rescan shortly after — near-instant tray updates.
    pub watch_fs: bool,
    /// System notifications when unpushed work appears or everything
    /// becomes pushed.
    pub notify_on_problems: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            machine_id: uuid::Uuid::new_v4().to_string(),
            roots: Vec::new(),
            exclude_dirs: [
                "node_modules",
                "target",
                "vendor",
                "venv",
                ".venv",
                "__pycache__",
                "Pods",
                "DerivedData",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            extra_repos: Vec::new(),
            hidden_repos: Vec::new(),
            scan_interval_secs: 300,
            max_depth: 6,
            start_hidden: false,
            theme: Theme::Auto,
            lang: Lang::Auto,
            sync_dir: None,
            auto_fetch: false,
            watch_fs: true,
            notify_on_problems: true,
        }
    }
}

impl Config {
    pub fn clamp(&mut self) {
        self.scan_interval_secs = self.scan_interval_secs.clamp(60, 86_400);
        self.max_depth = self.max_depth.clamp(1, 32);
    }
}

/// Payload of `scan://repo` events.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ScanRepoEvent {
    pub status: RepoStatus,
}

/// Payload of `scan://done`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ScanDoneEvent {
    pub generated_at: DateTime<Utc>,
    pub repo_count: usize,
    pub problem_count: usize,
    /// All discovered repo paths — the frontend prunes its list against
    /// this so deleted/hidden repos never linger with stale statuses.
    pub paths: Vec<String>,
    /// Non-fatal discovery warnings (missing roots, unreadable dirs).
    pub warnings: Vec<String>,
}

/// Lines streamed from a running `git push`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PushLineEvent {
    pub path: String,
    pub line: String,
}

/// Final result of a `git push`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PushDoneEvent {
    pub path: String,
    pub ok: bool,
    pub code: i32,
}

/// Progress of the batch "push all" queue.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PushQueueEvent {
    pub current: usize,
    pub total: usize,
    pub path: String,
    pub name: String,
}

/// Summary of the whole batch push.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PushAllDoneEvent {
    pub pushed: usize,
    pub failed: usize,
    pub total: usize,
}

/// Final result of a single `git fetch`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FetchDoneEvent {
    pub path: String,
    pub ok: bool,
}

/// Summary of a "fetch everything" pass.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FetchAllDoneEvent {
    pub total: usize,
    pub failed: usize,
}

/// One line of the activity log (scans and pushes).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HistoryEntry {
    pub ts: DateTime<Utc>,
    /// "scan" | "push" | "fetch"
    pub kind: String,
    pub message: String,
    pub ok: Option<bool>,
}
