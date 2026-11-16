import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  BootState,
  Config,
  FetchAllDonePayload,
  FetchDonePayload,
  HistoryEntry,
  MachineSnapshot,
  PushAllDonePayload,
  PushDonePayload,
  PushLinePayload,
  PushQueuePayload,
  RepoStatus,
  ScanDonePayload,
} from "./types";

// Commands
export const getBoot = () => invoke<BootState>("get_boot");
export const saveConfig = (config: Config) => invoke<Config>("save_config", { config });
export const startScan = () => invoke<boolean>("start_scan");
export const pushBranch = (
  path: string,
  branch: string,
  remote: string | null,
  setUpstream: boolean,
) => invoke<void>("push_branch", { path, branch, remote, setUpstream });
export const setAutostart = (enabled: boolean) => invoke<void>("set_autostart", { enabled });
export const revealPath = (path: string) => invoke<void>("reveal_path", { path });
export const fetchRepo = (path: string) => invoke<void>("fetch_repo", { path });
export const fetchAll = () => invoke<void>("fetch_all");
export const pushAll = () => invoke<void>("push_all");
export const getHistory = () => invoke<HistoryEntry[]>("get_history");
export const getMachines = () => invoke<MachineSnapshot[]>("get_machines");

// Events
export const listenScanStart = (cb: () => void) =>
  listen<null>("scan://start", () => cb());
export const listenScanRepo = (cb: (status: RepoStatus) => void) =>
  listen<{ status: RepoStatus }>("scan://repo", (e) => cb(e.payload.status));
export const listenScanDone = (cb: (p: ScanDonePayload) => void) =>
  listen<ScanDonePayload>("scan://done", (e) => cb(e.payload));
export const listenPushLine = (cb: (p: PushLinePayload) => void) =>
  listen<PushLinePayload>("push://line", (e) => cb(e.payload));
export const listenPushDone = (cb: (p: PushDonePayload) => void) =>
  listen<PushDonePayload>("push://done", (e) => cb(e.payload));
export const listenPushQueue = (cb: (p: PushQueuePayload) => void) =>
  listen<PushQueuePayload>("push://queue", (e) => cb(e.payload));
export const listenPushAllDone = (cb: (p: PushAllDonePayload) => void) =>
  listen<PushAllDonePayload>("push://all_done", (e) => cb(e.payload));
export const listenFetchDone = (cb: (p: FetchDonePayload) => void) =>
  listen<FetchDonePayload>("fetch://done", (e) => cb(e.payload));
export const listenFetchAllDone = (cb: (p: FetchAllDonePayload) => void) =>
  listen<FetchAllDonePayload>("fetch://all_done", (e) => cb(e.payload));
