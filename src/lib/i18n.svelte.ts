// UI language: reactive ru/en dictionary. `pref` mirrors Config.lang from
// the backend ("auto" follows navigator.language); store.init() and
// store.applyConfig() keep it in sync. t() reads `pref` while rendering,
// so every component re-renders instantly when the language changes.
// The backend has its own dictionary in `src-tauri/src/lang.rs` — keep the
// two in sync when adding strings.

export type UiLang = "ru" | "en";
export type LangPref = "auto" | UiLang;

let pref = $state<LangPref>("auto");

export function setLangPref(value: LangPref) {
  pref = value;
}

export function uiLang(): UiLang {
  if (pref !== "auto") return pref;
  const nav =
    typeof navigator !== "undefined" ? navigator.language.toLowerCase() : "en";
  return nav.startsWith("ru") ? "ru" : "en";
}

const RU = {
  // Header
  tab_list: "Список",
  tab_machines: "Машины",
  tab_history: "История",
  btn_settings: "Настройки",
  summary_push_queue: "Пуш {cur}/{total}: {name}",
  summary_pushing: "Пуш выполняется…",
  summary_fetching: "Fetch всё…",
  summary_scanning: "Сканирование…",
  summary_no_git: "git недоступен",
  summary_all_ok: "Всё запушено · {n} репо",
  summary_problems: "{n} проблемных из {total}",
  time_scan: "скан:",
  btn_fetch_all: "Fetch всё",
  title_fetch_all: "git fetch --all --prune по всем репозиториям",
  btn_push_all: "Запушить всё",
  btn_push_all_n: "Запушить всё ({n})",
  title_push_all: "Последовательно запушить все ветки с незапушенными коммитами",
  push_progress: "Пуш {cur}/{total}",
  btn_scan: "Сканировать",

  // Repo list
  filter_problems: "Проблемные",
  filter_all: "Все",
  search_ph: "Поиск по имени или пути…",
  empty_no_repos: "Репозитории не найдены. Добавьте корневые папки в настройках.",
  empty_all_pushed: "✓ Всё запушено",
  empty_filter: "Ничего не найдено по фильтру.",

  // Repo row
  title_reveal: "Показать в файловом менеджере",
  title_hide: "Скрыть репозиторий",
  err_hide: "Не удалось скрыть репозиторий: {e}",

  // Repo details
  title_fetch_repo: "git fetch --all --prune — обновить данные о remote",
  running_push: "push выполняется…",
  running_fetch: "fetch выполняется…",
  no_problems: "Проблем нет — всё отправлено",
  files_title: "Изменённые файлы ({n})",
  show_all: "показать все",
  collapse: "свернуть",
  more_files: "… и ещё {n}",
  th_branch: "Ветка",
  th_upstream: "Upstream",
  gone_mark: "remote удалён",
  no_remote_title: "нет настроенного remote",
  synced_title: "нечего пушить — ветка синхронизирована с upstream",
  proc_title: "Вывод git",
  stash_note: "stash: {n}",

  // Problem texts
  prob_dirty: "Незакоммиченные изменения ({parts})",
  prob_changes: "изменённых: {n}",
  prob_new: "новых: {n}",
  prob_ahead: "Ветка {branch}: {n} коммит(ов) не запушено в {upstream}",
  prob_diverged:
    "Ветка {branch} расходится с {upstream}: впереди {ahead}, позади {behind} — нужен pull --rebase перед push",
  prob_no_upstream: "Ветка {branch} ни разу не запушена",
  prob_no_remote: "Remote не настроен — {n} ветка(и) существуют только на этой машине",
  prob_gone: "Remote-ветка {upstream} отсутствует (удалена или ещё не получена) — локальная ветка {branch}",
  prob_orphan: "{n} коммит(ов) не видны ни на одном remote",
  prob_detached: "HEAD в detached-состоянии (вне ветки)",

  // Chip titles
  chip_ahead_title: "{branch}: {n} не запушено в {upstream}",
  chip_diverged_title: "{branch}: расходится с {upstream} — нужен pull --rebase",
  chip_dirty_title: "изменённых файлов: {n}",
  chip_new_title: "новых (untracked): {n}",
  chip_no_upstream_title: "ветка ни разу не запушена — upstream не задан",
  chip_no_remote_title: "remote не настроен — репозиторий живёт только на этой машине",
  chip_gone_title: "remote-ветка {upstream} удалена или не получена",
  chip_orphan_title: "коммитов вне всех remote и веток: {n}",
  chip_detached_title: "HEAD вне ветки",

  // Machines view
  m_state_error: "есть ошибки",
  m_state_unpushed: "{n} с незапушенными",
  m_state_problems: "{n} проблемных",
  m_state_ok: "всё запушено · {n} репо",
  m_own: "эта машина",
  m_stale: "устарел",
  m_stale_title: "Снапшот старше 2 суток",
  m_empty_nosync:
    "Сводка по машинам отключена — укажите папку синхронизации (Syncthing / iCloud / приватный git-репозиторий) в настройках. Каждый скан кладёт туда snapshot этой машины, pushwatch читает все.",
  m_empty_none: "Пока ни одного снапшота в папке синхронизации…",
  m_no_repos: "нет репозиториев",

  // History view
  btn_refresh: "Обновить",
  hist_hint: "Последние 200 событий",
  hist_empty: "История пуста — сделайте скан или push.",
  kind_scan: "скан",
  th_time: "Время",
  th_kind: "Тип",
  th_event: "Событие",

  // Banner / loading
  banner_no_git: "git не найден в PATH. Установите git и перезапустите pushwatch.",
  banner_no_roots: "Укажите корневые папки с репозиториями",
  loading: "Загрузка…",

  // Settings
  st_title: "Настройки",
  sec_roots: "Корневые папки для поиска репозиториев",
  btn_remove: "Убрать",
  btn_add_folder: "+ Добавить папку…",
  hint_no_roots: "Пока папок нет — сканировать нечего.",
  sec_extras: "Дополнительные репозитории (вне корней)",
  btn_add_repo: "+ Добавить репозиторий…",
  sec_excludes: "Исключённые папки (не спускаться внутрь)",
  excl_ph: "имя папки, например vendor",
  btn_add: "+ Добавить",
  sec_hidden: "Скрытые репозитории",
  hint_no_hidden: "Нет скрытых.",
  btn_restore: "Вернуть",
  sec_scan: "Сканирование и автоматика",
  lbl_interval: "Интервал, сек",
  lbl_depth: "Глубина поиска, уровней",
  lbl_watch_fs: "Реагировать на изменения .git (мгновенный перескан после коммитов)",
  lbl_auto_fetch: "git fetch --all перед каждым фоновым сканом (сетевой трафик)",
  lbl_notify: "Системные уведомления при появлении незапушенных коммитов",
  lbl_theme: "Тема",
  theme_auto: "Авто (как в системе)",
  theme_light: "Светлая",
  theme_dark: "Тёмная",
  lbl_lang: "Язык",
  lang_auto: "Авто (как в системе)",
  lang_ru: "Русский",
  lang_en: "English",
  lbl_autostart: "Запускать при входе в систему",
  sec_sync: "Синхронизация между машинами",
  hint_sync:
    "Папка, куда каждый скан кладёт snapshot этой машины (snapshot-<id>.json) и откуда читаются snapshot других машин. Синхронизируйте её Syncthing / iCloud / приватным git-репозиторием — вкладка «Машины» покажет сводку.",
  sync_unset: "— не настроено —",
  btn_pick_sync: "Выбрать папку…",
  btn_sync_disable: "Отключить",
  sec_machine: "Эта машина",
  m_name_label: "имя",
  btn_save: "Сохранить и пересканировать",
  btn_saving: "Сохранение…",
  hint_save: "Изменения корней/исключений применяются после сохранения.",
  err_save: "Не удалось сохранить настройки: {e}",

  // Toasts
  toast_pushed: "Запушено: {p} из {t}",
  toast_push_done: "Пуш завершён: отправлено {p}, ошибок {f} из {t}",
  toast_fetch_ok: "Fetch выполнен: {n} репо",
  toast_fetch_partial: "Fetch выполнен: {n} репо, ошибок {m}",
  toast_fetch_not_started: "Fetch не запущен: {e}",
} as const;

const EN: Record<keyof typeof RU, string> = {
  // Header
  tab_list: "List",
  tab_machines: "Machines",
  tab_history: "History",
  btn_settings: "Settings",
  summary_push_queue: "Push {cur}/{total}: {name}",
  summary_pushing: "Pushing…",
  summary_fetching: "Fetching all…",
  summary_scanning: "Scanning…",
  summary_no_git: "git unavailable",
  summary_all_ok: "All pushed · {n} repos",
  summary_problems: "{n} of {total} repos have problems",
  time_scan: "scan:",
  btn_fetch_all: "Fetch all",
  title_fetch_all: "git fetch --all --prune on all repositories",
  btn_push_all: "Push all",
  btn_push_all_n: "Push all ({n})",
  title_push_all: "Push every branch with unpushed commits, one by one",
  push_progress: "Push {cur}/{total}",
  btn_scan: "Scan",

  // Repo list
  filter_problems: "Problems",
  filter_all: "All",
  search_ph: "Search by name or path…",
  empty_no_repos: "No repositories found. Add root folders in settings.",
  empty_all_pushed: "✓ All pushed",
  empty_filter: "Nothing matches the filter.",

  // Repo row
  title_reveal: "Reveal in file manager",
  title_hide: "Hide repository",
  err_hide: "Failed to hide repository: {e}",

  // Repo details
  title_fetch_repo: "git fetch --all --prune — refresh remote data",
  running_push: "push running…",
  running_fetch: "fetch running…",
  no_problems: "No problems — everything is pushed",
  files_title: "Changed files ({n})",
  show_all: "show all",
  collapse: "collapse",
  more_files: "… and {n} more",
  th_branch: "Branch",
  th_upstream: "Upstream",
  gone_mark: "remote deleted",
  no_remote_title: "no remote configured",
  synced_title: "nothing to push — branch is in sync with upstream",
  proc_title: "git output",
  stash_note: "stash: {n}",

  // Problem texts
  prob_dirty: "Uncommitted changes ({parts})",
  prob_changes: "modified: {n}",
  prob_new: "new: {n}",
  prob_ahead: "Branch {branch}: {n} unpushed commit(s) ahead of {upstream}",
  prob_diverged:
    "Branch {branch} diverged from {upstream}: {ahead} ahead, {behind} behind — pull --rebase before push",
  prob_no_upstream: "Branch {branch} has never been pushed",
  prob_no_remote: "No remote configured — {n} branch(es) exist only on this machine",
  prob_gone: "Remote branch {upstream} is missing (deleted or not fetched) — local branch {branch}",
  prob_orphan: "{n} commit(s) are not visible on any remote",
  prob_detached: "HEAD is detached (not on any branch)",

  // Chip titles
  chip_ahead_title: "{branch}: {n} unpushed to {upstream}",
  chip_diverged_title: "{branch}: diverged from {upstream} — pull --rebase needed",
  chip_dirty_title: "changed files: {n}",
  chip_new_title: "new (untracked): {n}",
  chip_no_upstream_title: "branch never pushed — no upstream set",
  chip_no_remote_title: "no remote — repository lives only on this machine",
  chip_gone_title: "remote branch {upstream} deleted or not fetched",
  chip_orphan_title: "commits outside all remotes and branches: {n}",
  chip_detached_title: "HEAD not on any branch",

  // Machines view
  m_state_error: "has errors",
  m_state_unpushed: "{n} with unpushed commits",
  m_state_problems: "{n} with problems",
  m_state_ok: "all pushed · {n} repos",
  m_own: "this machine",
  m_stale: "stale",
  m_stale_title: "Snapshot older than 2 days",
  m_empty_nosync:
    "Cross-machine summary is off — set a sync folder (Syncthing / iCloud / private git repository) in settings. Every scan writes this machine's snapshot there; pushwatch reads them all.",
  m_empty_none: "No snapshots in the sync folder yet…",
  m_no_repos: "no repositories",

  // History view
  btn_refresh: "Refresh",
  hist_hint: "Last 200 events",
  hist_empty: "History is empty — run a scan or push.",
  kind_scan: "scan",
  th_time: "Time",
  th_kind: "Kind",
  th_event: "Event",

  // Banner / loading
  banner_no_git: "git not found in PATH. Install git and restart pushwatch.",
  banner_no_roots: "Add root folders with your repositories",
  loading: "Loading…",

  // Settings
  st_title: "Settings",
  sec_roots: "Root folders to search for repositories",
  btn_remove: "Remove",
  btn_add_folder: "+ Add folder…",
  hint_no_roots: "No folders yet — nothing to scan.",
  sec_extras: "Extra repositories (outside the roots)",
  btn_add_repo: "+ Add repository…",
  sec_excludes: "Excluded folders (never descended into)",
  excl_ph: "folder name, e.g. vendor",
  btn_add: "+ Add",
  sec_hidden: "Hidden repositories",
  hint_no_hidden: "None hidden.",
  btn_restore: "Restore",
  sec_scan: "Scanning & automation",
  lbl_interval: "Interval, sec",
  lbl_depth: "Search depth, levels",
  lbl_watch_fs: "React to .git changes (rescan right after commits)",
  lbl_auto_fetch: "git fetch --all before every background scan (network traffic)",
  lbl_notify: "System notifications when unpushed commits appear",
  lbl_theme: "Theme",
  theme_auto: "Auto (follow system)",
  theme_light: "Light",
  theme_dark: "Dark",
  lbl_lang: "Language",
  lang_auto: "Auto (follow system)",
  lang_ru: "Русский",
  lang_en: "English",
  lbl_autostart: "Launch at login",
  sec_sync: "Cross-machine sync",
  hint_sync:
    "The folder where every scan writes this machine's snapshot (snapshot-<id>.json) and from which other machines' snapshots are read. Sync it via Syncthing / iCloud / a private git repository — the Machines tab shows the summary.",
  sync_unset: "— not set —",
  btn_pick_sync: "Choose folder…",
  btn_sync_disable: "Disable",
  sec_machine: "This machine",
  m_name_label: "name",
  btn_save: "Save and rescan",
  btn_saving: "Saving…",
  hint_save: "Root/exclusion changes apply after saving.",
  err_save: "Failed to save settings: {e}",

  // Toasts
  toast_pushed: "Pushed: {p} of {t}",
  toast_push_done: "Push finished: {p} pushed, {f} failed of {t}",
  toast_fetch_ok: "Fetch done: {n} repos",
  toast_fetch_partial: "Fetch done: {n} repos, {m} failed",
  toast_fetch_not_started: "Fetch not started: {e}",
};

export function t<K extends keyof typeof RU>(
  key: K,
  params?: Record<string, string | number>,
): string {
  let s: string = (uiLang() === "ru" ? RU : EN)[key];
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}
