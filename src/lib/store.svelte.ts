// Global reactive store (Svelte 5 runes). Single source of truth for the UI.
import { setLangPref, t } from "$lib/i18n.svelte";
import {
  fetchAll,
  fetchRepo,
  getBoot,
  getHistory,
  getMachines,
  listenFetchAllDone,
  listenFetchDone,
  listenPushAllDone,
  listenPushDone,
  listenPushLine,
  listenPushQueue,
  listenScanDone,
  listenScanRepo,
  listenScanStart,
  pushAll,
  pushBranch,
  startScan,
} from "./api";
import type {
  BootState,
  Config,
  HistoryEntry,
  MachineSnapshot,
  RepoStatus,
} from "./types";

export interface ProcState {
  lines: string[];
  running: boolean;
  ok: boolean | null;
}

function severity(r: RepoStatus): number {
  if (r.state === "error") return 0; // unreadable repos need attention most
  if (r.state !== "ok") {
    const unpushed = r.problems.some(
      (p) =>
        p.kind === "ahead" ||
        p.kind === "diverged" ||
        p.kind === "orphan" ||
        p.kind === "upstream_gone" ||
        p.kind === "no_remote",
    );
    return unpushed ? 1 : 2;
  }
  return 3;
}

function hasUnpushedProblems(r: RepoStatus): boolean {
  return r.problems.some(
    (p) =>
      p.kind === "ahead" ||
      p.kind === "diverged" ||
      p.kind === "orphan" ||
      p.kind === "upstream_gone" ||
      p.kind === "no_remote",
  );
}

class AppStore {
  ready = $state(false);
  boot = $state<BootState | null>(null);
  scanning = $state(false);
  repos = $state<RepoStatus[]>([]);
  lastScanAt = $state<string | null>(null);
  pushes = $state<Record<string, ProcState>>({});
  fetches = $state<Record<string, ProcState>>({});
  toast = $state<string | null>(null);
  view = $state<"list" | "machines" | "history" | "settings">("list");
  filter = $state<"problems" | "all">("problems");
  search = $state("");
  expandedPath = $state<string | null>(null);
  machines = $state<MachineSnapshot[]>([]);
  history = $state<HistoryEntry[]>([]);
  fetchingAll = $state(false);
  pushAllRunning = $state(false);
  pushQueue = $state<{ current: number; total: number; name: string } | null>(null);

  #pendingScan = false;
  #toastTimer: ReturnType<typeof setTimeout> | null = null;

  problemCount = $derived(this.repos.filter((r) => r.state !== "ok").length);

  hasUnpushed = $derived(this.repos.some(hasUnpushedProblems));

  pushableCount = $derived(
    this.repos.filter(
      (r) =>
        r.remotes.length > 0 &&
        r.problems.some(
          (p) =>
            p.kind === "ahead" || p.kind === "no_upstream" || p.kind === "upstream_gone",
        ),
    ).length,
  );

  visibleRepos = $derived.by(() => {
    const q = this.search.trim().toLowerCase();
    let list = this.repos;
    if (this.filter === "problems") list = list.filter((r) => r.state !== "ok");
    if (q) {
      list = list.filter(
        (r) => r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q),
      );
    }
    return [...list].sort(
      (a, b) => severity(a) - severity(b) || a.name.localeCompare(b.name),
    );
  });

  async init() {
    this.boot = await getBoot();
    this.repos = this.boot.snapshot?.repos ?? [];
    this.lastScanAt = this.boot.snapshot?.generated_at ?? null;
    this.scanning = this.boot.scanning;
    setLangPref(this.boot.config.lang);

    await Promise.all([
      listenScanStart(() => {
        this.scanning = true;
      }),
      listenScanRepo((status) => this.upsert(status)),
      listenScanDone((p) => {
        this.scanning = false;
        this.lastScanAt = p.generated_at;
        // Prune stale entries: repos deleted from disk or hidden via
        // settings would otherwise linger with old statuses while the
        // tray (computed from the fresh snapshot) shows green.
        const alive = new Set(p.paths);
        this.repos = this.repos.filter((r) => alive.has(r.path));
        if (p.warnings.length > 0) {
          this.notify(p.warnings.join(" · "));
        }
        if (this.boot?.config.sync_dir) {
          void this.refreshMachines();
        }
        // A scan requested while another was in flight was rejected —
        // run it now (e.g. right after saving new settings).
        if (this.#pendingScan) {
          this.#pendingScan = false;
          setTimeout(() => void this.scan(), 200);
        }
      }),
      listenPushLine((p) => {
        const st = this.pushes[p.path];
        if (st) {
          st.lines.push(p.line);
        }
      }),
      listenPushDone((p) => {
        const st = this.pushes[p.path];
        if (st) {
          st.running = false;
          st.ok = p.ok;
        }
        // Single pushes refresh right away; the batch queue triggers its
        // own rescan after the whole queue finishes.
        if (!this.pushAllRunning) {
          setTimeout(() => void this.scan(), 500);
        }
      }),
      listenPushQueue((p) => {
        this.pushQueue = { current: p.current, total: p.total, name: p.name };
      }),
      listenPushAllDone((p) => {
        this.pushAllRunning = false;
        this.pushQueue = null;
        this.notify(
          p.failed === 0
            ? t("toast_pushed", { p: p.pushed, t: p.total })
            : t("toast_push_done", { p: p.pushed, f: p.failed, t: p.total }),
        );
      }),
      listenFetchDone((p) => {
        const st = this.fetches[p.path];
        if (st) {
          st.running = false;
          st.ok = p.ok;
        }
      }),
      listenFetchAllDone((p) => {
        this.fetchingAll = false;
        this.notify(
          p.failed === 0
            ? t("toast_fetch_ok", { n: p.total })
            : t("toast_fetch_partial", { n: p.total - p.failed, m: p.failed }),
        );
      }),
    ]);

    this.ready = true;
    if (this.boot.git_available && this.boot.config.roots.length === 0 && this.repos.length === 0) {
      this.view = "settings";
    }
    if (this.boot.config.sync_dir) {
      void this.refreshMachines();
    }
    if (this.boot.git_available) {
      await this.scan();
    }
  }

  upsert(status: RepoStatus) {
    const i = this.repos.findIndex((r) => r.path === status.path);
    if (i >= 0) {
      this.repos[i] = status;
    } else {
      this.repos.push(status);
    }
  }

  notify(message: string) {
    this.toast = message;
    if (this.#toastTimer) clearTimeout(this.#toastTimer);
    this.#toastTimer = setTimeout(() => (this.toast = null), 6000);
  }

  async scan() {
    const started = await startScan();
    if (started) {
      this.scanning = true;
    } else {
      // Another scan is in flight; queue one so this request is not lost.
      this.#pendingScan = true;
    }
  }

  async fetch(path: string) {
    this.fetches[path] = { lines: [], running: true, ok: null };
    try {
      await fetchRepo(path);
    } catch (e) {
      const st = this.fetches[path];
      if (st) {
        st.lines.push(String(e));
        st.running = false;
        st.ok = false;
      }
    }
  }

  async fetchEverything() {
    if (this.fetchingAll) return;
    this.fetchingAll = true;
    try {
      await fetchAll();
    } catch (e) {
      this.fetchingAll = false;
      this.notify(t("toast_fetch_not_started", { e: String(e) }));
    }
  }

  async runPushAll() {
    if (this.pushAllRunning) return;
    try {
      await pushAll();
      this.pushAllRunning = true;
    } catch (e) {
      // "нечего пушить" and "уже выполняется" arrive as errors — show them.
      this.notify(String(e));
    }
  }

  async refreshMachines() {
    try {
      this.machines = await getMachines();
    } catch {
      // sync dir may not exist yet — silently keep the previous list
    }
  }

  async loadHistory() {
    try {
      this.history = await getHistory();
    } catch {
      this.history = [];
    }
  }

  async push(path: string, branch: string, remote: string | null, setUpstream: boolean) {
    this.pushes[path] = { lines: [], running: true, ok: null };
    try {
      await pushBranch(path, branch, remote, setUpstream);
    } catch (e) {
      const st = this.pushes[path];
      if (st) {
        st.lines.push(String(e));
        st.running = false;
        st.ok = false;
      }
    }
  }

  config(): Config {
    return this.boot!.config;
  }

  applyConfig(config: Config) {
    if (this.boot) {
      this.boot.config = config;
    }
    setLangPref(config.lang);
  }
}

export const store = new AppStore();
