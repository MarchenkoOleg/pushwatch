// TS mirror of the Rust model (src-tauri/src/model.rs, serde snake_case).
// Keep in sync — the snapshot shape is the future cross-machine contract.

export type RepoState = "ok" | "problem" | "error";

export interface BranchInfo {
  name: string;
  upstream: string | null;
  /** Exact remote name (from %(upstream:remotename)) — the push target. */
  remote: string | null;
  ahead: number;
  behind: number;
  gone: boolean;
  is_head: boolean;
}

export type Problem =
  | { kind: "dirty"; staged: number; unstaged: number; untracked: number; conflicts: number }
  | { kind: "ahead"; branch: string; upstream: string; ahead: number }
  | { kind: "diverged"; branch: string; upstream: string; ahead: number; behind: number }
  | { kind: "no_upstream"; branch: string }
  | { kind: "no_remote"; branches: number }
  | { kind: "upstream_gone"; branch: string; upstream: string }
  | { kind: "orphan"; count: number }
  | { kind: "detached_head" };

export interface DirtyFile {
  /** Two-letter status, e.g. `.M`, `M.`, `MM`, `??`, `uu`, `R.`. */
  label: string;
  path: string;
}

export interface RepoStatus {
  path: string;
  name: string;
  state: RepoState;
  problems: Problem[];
  branches: BranchInfo[];
  /** Configured remote names (`git remote`) — push targets for no-upstream branches. */
  remotes: string[];
  dirty_files: DirtyFile[];
  dirty_files_total: number;
  current_branch: string | null;
  detached: boolean;
  stashes: number;
  scanned_at: string | null;
  error: string | null;
}

export interface MachineSnapshot {
  machine_id: string;
  machine_name: string;
  generated_at: string;
  repos: RepoStatus[];
}

export type ThemePref = "auto" | "light" | "dark";

export type LangPref = "auto" | "ru" | "en";

export interface Config {
  machine_id: string;
  roots: string[];
  exclude_dirs: string[];
  extra_repos: string[];
  hidden_repos: string[];
  scan_interval_secs: number;
  max_depth: number;
  start_hidden: boolean;
  theme: ThemePref;
  lang: LangPref;
  sync_dir: string | null;
  auto_fetch: boolean;
  watch_fs: boolean;
  notify_on_problems: boolean;
}

export interface BootState {
  config: Config;
  snapshot: MachineSnapshot | null;
  scanning: boolean;
  machine_name: string;
  machine_id: string;
  git_available: boolean;
  git_version: string | null;
  autostart: boolean;
}

export interface ScanDonePayload {
  generated_at: string;
  repo_count: number;
  problem_count: number;
  /** All discovered paths — the UI prunes its list against this. */
  paths: string[];
  /** Non-fatal discovery warnings (missing roots, unreadable dirs). */
  warnings: string[];
}

export interface PushLinePayload {
  path: string;
  line: string;
}

export interface PushDonePayload {
  path: string;
  ok: boolean;
  code: number;
}

export interface PushQueuePayload {
  current: number;
  total: number;
  path: string;
  name: string;
}

export interface PushAllDonePayload {
  pushed: number;
  failed: number;
  total: number;
}

export interface FetchDonePayload {
  path: string;
  ok: boolean;
}

export interface FetchAllDonePayload {
  total: number;
  failed: number;
}

export interface HistoryEntry {
  ts: string;
  kind: string;
  message: string;
  ok: boolean | null;
}
