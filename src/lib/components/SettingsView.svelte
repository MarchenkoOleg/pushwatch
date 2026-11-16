<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { t } from "$lib/i18n.svelte";
  import { saveConfig, setAutostart } from "$lib/api";
  import { store } from "$lib/store.svelte";
  import type { Config, LangPref, ThemePref } from "$lib/types";

  // Draft copy: edits are local until "Save" persists + rescans.
  const draft = $state<Config>(
    JSON.parse(JSON.stringify(store.config())) as Config,
  );
  let saving = $state(false);
  let autostartEnabled = $state(store.boot?.autostart ?? false);
  let newExclude = $state("");

  const themeOptions: Array<[ThemePref, () => string]> = [
    ["auto", () => t("theme_auto")],
    ["light", () => t("theme_light")],
    ["dark", () => t("theme_dark")],
  ];
  const langOptions: Array<[LangPref, () => string]> = [
    ["auto", () => t("lang_auto")],
    ["ru", () => t("lang_ru")],
    ["en", () => t("lang_en")],
  ];

  async function save() {
    // Empty number inputs bind as null and serde would reject them —
    // coerce back into the valid range before sending.
    draft.scan_interval_secs = Number.isFinite(Number(draft.scan_interval_secs))
      ? Number(draft.scan_interval_secs)
      : 300;
    draft.max_depth = Number.isFinite(Number(draft.max_depth)) ? Number(draft.max_depth) : 6;
    saving = true;
    try {
      const saved = await saveConfig($state.snapshot(draft) as Config);
      store.applyConfig(saved);
      await store.scan();
    } catch (e) {
      store.notify(t("err_save", { e: String(e) }));
    } finally {
      saving = false;
    }
  }

  async function pickFolder(into: "roots" | "extra_repos") {
    const path = await open({ directory: true, multiple: false });
    if (typeof path === "string" && !draft[into].includes(path)) {
      draft[into] = [...draft[into], path];
      await save();
    }
  }

  async function pickSyncDir() {
    const path = await open({ directory: true, multiple: false });
    if (typeof path === "string") {
      draft.sync_dir = path;
      await save();
    }
  }

  function addExclude() {
    const v = newExclude.trim();
    if (v && !draft.exclude_dirs.includes(v)) {
      draft.exclude_dirs = [...draft.exclude_dirs, v];
    }
    newExclude = "";
  }

  async function toggleAutostart() {
    try {
      await setAutostart(autostartEnabled);
    } catch {
      autostartEnabled = !autostartEnabled;
    }
  }
</script>

<div class="settings">
  <h2>{t("st_title")}</h2>

  <section>
    <h3>{t("sec_roots")}</h3>
    {#each draft.roots as root (root)}
      <div class="line">
        <span class="mono">{root}</span>
        <button class="small" onclick={() => (draft.roots = draft.roots.filter((r) => r !== root))}>{t("btn_remove")}</button>
      </div>
    {/each}
    <button class="small" onclick={() => void pickFolder("roots")}>{t("btn_add_folder")}</button>
    {#if draft.roots.length === 0}
      <p class="hint">{t("hint_no_roots")}</p>
    {/if}
  </section>

  <section>
    <h3>{t("sec_extras")}</h3>
    {#each draft.extra_repos as extra (extra)}
      <div class="line">
        <span class="mono">{extra}</span>
        <button class="small" onclick={() => (draft.extra_repos = draft.extra_repos.filter((r) => r !== extra))}>{t("btn_remove")}</button>
      </div>
    {/each}
    <button class="small" onclick={() => void pickFolder("extra_repos")}>{t("btn_add_repo")}</button>
  </section>

  <section>
    <h3>{t("sec_excludes")}</h3>
    <div class="tags">
      {#each draft.exclude_dirs as ex (ex)}
        <span class="tag">
          {ex}
          <button class="x" title={t("btn_remove")} onclick={() => (draft.exclude_dirs = draft.exclude_dirs.filter((e) => e !== ex))}>✕</button>
        </span>
      {/each}
    </div>
    <div class="line">
      <input
        type="text"
        placeholder={t("excl_ph")}
        bind:value={newExclude}
        onkeydown={(e) => e.key === "Enter" && addExclude()}
      />
      <button class="small" onclick={addExclude}>{t("btn_add")}</button>
    </div>
  </section>

  <section>
    <h3>{t("sec_hidden")}</h3>
    {#if draft.hidden_repos.length === 0}
      <p class="hint">{t("hint_no_hidden")}</p>
    {:else}
      {#each draft.hidden_repos as hidden (hidden)}
        <div class="line">
          <span class="mono">{hidden}</span>
          <button class="small" onclick={() => (draft.hidden_repos = draft.hidden_repos.filter((r) => r !== hidden))}>{t("btn_restore")}</button>
        </div>
      {/each}
    {/if}
  </section>

  <section>
    <h3>{t("sec_scan")}</h3>
    <div class="line">
      <label for="interval">{t("lbl_interval")}</label>
      <input id="interval" type="number" min="60" max="86400" bind:value={draft.scan_interval_secs} />
    </div>
    <div class="line">
      <label for="depth">{t("lbl_depth")}</label>
      <input id="depth" type="number" min="1" max="32" bind:value={draft.max_depth} />
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={draft.watch_fs} />
        {t("lbl_watch_fs")}
      </label>
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={draft.auto_fetch} />
        {t("lbl_auto_fetch")}
      </label>
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={draft.notify_on_problems} />
        {t("lbl_notify")}
      </label>
    </div>
    <div class="line">
      <span class="lbl">{t("lbl_theme")}</span>
      {#each themeOptions as [value, label] (value)}
        <label class="radio">
          <input type="radio" name="theme" value={value} bind:group={draft.theme} />
          {label()}
        </label>
      {/each}
    </div>
    <div class="line">
      <span class="lbl">{t("lbl_lang")}</span>
      {#each langOptions as [value, label] (value)}
        <label class="radio">
          <input type="radio" name="lang" value={value} bind:group={draft.lang} />
          {label()}
        </label>
      {/each}
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={autostartEnabled} onchange={() => void toggleAutostart()} />
        {t("lbl_autostart")}
      </label>
    </div>
  </section>

  <section>
    <h3>{t("sec_sync")}</h3>
    <p class="hint">
      {t("hint_sync")}
    </p>
    <div class="line">
      <span class="mono">{draft.sync_dir ?? t("sync_unset")}</span>
      <span class="spacer"></span>
      <button class="small" onclick={() => void pickSyncDir()}>{t("btn_pick_sync")}</button>
      {#if draft.sync_dir}
        <button class="small" onclick={() => { draft.sync_dir = null; void save(); }}>{t("btn_sync_disable")}</button>
      {/if}
    </div>
  </section>

  <section>
    <h3>{t("sec_machine")}</h3>
    <p class="hint">
      {t("m_name_label")}: {store.boot?.machine_name} · id: <span class="mono">{store.boot?.machine_id.slice(0, 8)}…</span>
      {#if store.boot?.git_version}
        · {store.boot.git_version}
      {/if}
    </p>
  </section>

  <div class="actions">
    <button class="primary" disabled={saving} onclick={() => void save()}>
      {saving ? t("btn_saving") : t("btn_save")}
    </button>
    <span class="hint">{t("hint_save")}</span>
  </div>
</div>

<style>
  .settings {
    flex: 1;
    overflow-y: auto;
    padding: 8px 24px 32px;
  }
  h2 {
    font-size: 18px;
    margin: 8px 0 4px;
  }
  h3 {
    font-size: 14px;
    margin: 0 0 8px;
    color: var(--accent);
  }
  section {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 14px 16px;
    margin: 12px 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0;
  }
  .line .spacer {
    flex: 1;
  }
  label {
    color: var(--dim);
  }
  .mono {
    font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
    font-size: 12px;
    color: var(--dim);
    user-select: text;
    word-break: break-all;
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
    margin: 4px 0;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 3px 8px;
    font-size: 12px;
  }
  button.x {
    background: none;
    border: none;
    color: var(--dim);
    padding: 0 2px;
    font-size: 11px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 8px;
  }
  input[type="number"] {
    width: 110px;
  }
</style>
