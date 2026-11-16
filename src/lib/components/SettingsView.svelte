<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { saveConfig, setAutostart } from "$lib/api";
  import { store } from "$lib/store.svelte";
  import type { Config, ThemePref } from "$lib/types";

  // Draft copy: edits are local until "Сохранить" persists + rescans.
  const draft = $state<Config>(
    JSON.parse(JSON.stringify(store.config())) as Config,
  );
  let saving = $state(false);
  let autostartEnabled = $state(store.boot?.autostart ?? false);
  let newExclude = $state("");

  const themeOptions: Array<[ThemePref, string]> = [
    ["auto", "Авто (как в системе)"],
    ["light", "Светлая"],
    ["dark", "Тёмная"],
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
      store.notify(`Не удалось сохранить настройки: ${String(e)}`);
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
  <h2>Настройки</h2>

  <section>
    <h3>Корневые папки для поиска репозиториев</h3>
    {#each draft.roots as root (root)}
      <div class="line">
        <span class="mono">{root}</span>
        <button class="small" onclick={() => (draft.roots = draft.roots.filter((r) => r !== root))}>Убрать</button>
      </div>
    {/each}
    <button class="small" onclick={() => void pickFolder("roots")}>+ Добавить папку…</button>
    {#if draft.roots.length === 0}
      <p class="hint">Пока папок нет — сканировать нечего.</p>
    {/if}
  </section>

  <section>
    <h3>Дополнительные репозитории (вне корней)</h3>
    {#each draft.extra_repos as extra (extra)}
      <div class="line">
        <span class="mono">{extra}</span>
        <button class="small" onclick={() => (draft.extra_repos = draft.extra_repos.filter((r) => r !== extra))}>Убрать</button>
      </div>
    {/each}
    <button class="small" onclick={() => void pickFolder("extra_repos")}>+ Добавить репозиторий…</button>
  </section>

  <section>
    <h3>Исключённые папки (не спускаться внутрь)</h3>
    <div class="tags">
      {#each draft.exclude_dirs as ex (ex)}
        <span class="tag">
          {ex}
          <button class="x" title="Убрать" onclick={() => (draft.exclude_dirs = draft.exclude_dirs.filter((e) => e !== ex))}>✕</button>
        </span>
      {/each}
    </div>
    <div class="line">
      <input
        type="text"
        placeholder="имя папки, например vendor"
        bind:value={newExclude}
        onkeydown={(e) => e.key === "Enter" && addExclude()}
      />
      <button class="small" onclick={addExclude}>+ Добавить</button>
    </div>
  </section>

  <section>
    <h3>Скрытые репозитории</h3>
    {#if draft.hidden_repos.length === 0}
      <p class="hint">Нет скрытых.</p>
    {:else}
      {#each draft.hidden_repos as hidden (hidden)}
        <div class="line">
          <span class="mono">{hidden}</span>
          <button class="small" onclick={() => (draft.hidden_repos = draft.hidden_repos.filter((r) => r !== hidden))}>Вернуть</button>
        </div>
      {/each}
    {/if}
  </section>

  <section>
    <h3>Сканирование и автоматика</h3>
    <div class="line">
      <label for="interval">Интервал, сек</label>
      <input id="interval" type="number" min="60" max="86400" bind:value={draft.scan_interval_secs} />
    </div>
    <div class="line">
      <label for="depth">Глубина поиска, уровней</label>
      <input id="depth" type="number" min="1" max="32" bind:value={draft.max_depth} />
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={draft.watch_fs} />
        Реагировать на изменения .git (мгновенный перескан после коммитов)
      </label>
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={draft.auto_fetch} />
        git fetch --all перед каждым фоновым сканом (сетевой трафик)
      </label>
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={draft.notify_on_problems} />
        Системные уведомления при появлении незапушенных коммитов
      </label>
    </div>
    <div class="line">
      <span class="lbl">Тема</span>
      {#each themeOptions as [value, label] (value)}
        <label class="radio">
          <input type="radio" name="theme" value={value} bind:group={draft.theme} />
          {label}
        </label>
      {/each}
    </div>
    <div class="line">
      <label>
        <input type="checkbox" bind:checked={autostartEnabled} onchange={() => void toggleAutostart()} />
        Запускать при входе в систему
      </label>
    </div>
  </section>

  <section>
    <h3>Синхронизация между машинами</h3>
    <p class="hint">
      Папка, куда каждый скан кладёт snapshot этой машины
      (snapshot-&lt;id&gt;.json) и откуда читаются snapshot других машин.
      Синхронизируйте её Syncthing / iCloud / приватным git-репозиторием —
      вкладка «Машины» покажет сводку.
    </p>
    <div class="line">
      <span class="mono">{draft.sync_dir ?? "— не настроено —"}</span>
      <span class="spacer"></span>
      <button class="small" onclick={() => void pickSyncDir()}>Выбрать папку…</button>
      {#if draft.sync_dir}
        <button class="small" onclick={() => { draft.sync_dir = null; void save(); }}>Отключить</button>
      {/if}
    </div>
  </section>

  <section>
    <h3>Эта машина</h3>
    <p class="hint">
      имя: {store.boot?.machine_name} · id: <span class="mono">{store.boot?.machine_id.slice(0, 8)}…</span>
      {#if store.boot?.git_version}
        · {store.boot.git_version}
      {/if}
    </p>
  </section>

  <div class="actions">
    <button class="primary" disabled={saving} onclick={() => void save()}>
      {saving ? "Сохранение…" : "Сохранить и пересканировать"}
    </button>
    <span class="hint">Изменения корней/исключений применяются после сохранения.</span>
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
