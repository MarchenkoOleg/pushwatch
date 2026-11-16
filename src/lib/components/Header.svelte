<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { formatScanTime } from "$lib/time";

  const dotCls = $derived(
    store.scanning
      ? "gray pulse"
      : !store.boot?.git_available
        ? "gray"
        : store.problemCount > 0
          ? store.hasUnpushed
            ? "red"
            : "yellow"
          : "green",
  );

  const summary = $derived.by(() => {
    if (store.pushQueue) return `Пуш ${store.pushQueue.current}/${store.pushQueue.total}: ${store.pushQueue.name}`;
    if (store.pushAllRunning) return "Пуш выполняется…";
    if (store.fetchingAll) return "Fetch всё…";
    if (store.scanning) return "Сканирование…";
    if (!store.boot?.git_available) return "git недоступен";
    if (store.problemCount === 0) return `Всё запушено · ${store.repos.length} репо`;
    return `${store.problemCount} проблемных из ${store.repos.length}`;
  });

  const tabs: Array<{ id: typeof store.view; label: string }> = [
    { id: "list", label: "Список" },
    { id: "machines", label: "Машины" },
    { id: "history", label: "История" },
  ];

  function openTab(id: (typeof tabs)[number]["id"]) {
    store.view = id;
    if (id === "history") void store.loadHistory();
    if (id === "machines") void store.refreshMachines();
  }
</script>

<header>
  <span class="dot {dotCls}"></span>
  <span class="title">pushwatch</span>
  <span class="summary">{summary}</span>
  <nav>
    {#each tabs as t (t.id)}
      <button class="ghost tab" class:active={store.view === t.id} onclick={() => openTab(t.id)}>
        {t.label}
      </button>
    {/each}
  </nav>
  <span class="spacer"></span>
  {#if store.lastScanAt}
    <span class="time">скан: {formatScanTime(store.lastScanAt)}</span>
  {/if}
  <button
    title="git fetch --all --prune по всем репозиториям"
    disabled={store.fetchingAll || store.scanning || !store.boot?.git_available || store.repos.length === 0}
    onclick={() => void store.fetchEverything()}
  >Fetch всё</button>
  <button
    title="Последовательно запушить все ветки с незапушенными коммитами"
    disabled={store.pushAllRunning || store.scanning || !store.boot?.git_available || store.pushableCount === 0}
    onclick={() => void store.runPushAll()}
  >
    {#if store.pushQueue}
      Пуш {store.pushQueue.current}/{store.pushQueue.total}
    {:else}
      Запушить всё{store.pushableCount > 0 ? ` (${store.pushableCount})` : ""}
    {/if}
  </button>
  <button
    disabled={store.scanning || !store.boot?.git_available}
    onclick={() => void store.scan()}
  >Сканировать</button>
  <button class="ghost" onclick={() => (store.view = store.view === "settings" ? "list" : "settings")}>
    Настройки
  </button>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  .title {
    font-weight: 700;
    font-size: 15px;
    margin-right: 2px;
  }
  .summary {
    color: var(--dim);
    font-size: 13px;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  nav {
    display: flex;
    gap: 2px;
    margin-left: 8px;
  }
  button.tab.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .spacer {
    flex: 1;
  }
  .time {
    color: var(--dim);
    font-size: 12px;
  }
</style>
