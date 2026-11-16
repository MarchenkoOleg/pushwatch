<script lang="ts">
  import { t } from "$lib/i18n.svelte";
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
    if (store.pushQueue) return t("summary_push_queue", { cur: store.pushQueue.current, total: store.pushQueue.total, name: store.pushQueue.name });
    if (store.pushAllRunning) return t("summary_pushing");
    if (store.fetchingAll) return t("summary_fetching");
    if (store.scanning) return t("summary_scanning");
    if (!store.boot?.git_available) return t("summary_no_git");
    if (store.problemCount === 0) return t("summary_all_ok", { n: store.repos.length });
    return t("summary_problems", { n: store.problemCount, total: store.repos.length });
  });

  type ViewId = typeof store.view;
  const tabs: Array<{ id: ViewId; label: () => string }> = [
    { id: "list", label: () => t("tab_list") },
    { id: "machines", label: () => t("tab_machines") },
    { id: "history", label: () => t("tab_history") },
  ];

  function openTab(id: ViewId) {
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
    {#each tabs as tab (tab.id)}
      <button class="ghost tab" class:active={store.view === tab.id} onclick={() => openTab(tab.id)}>
        {tab.label()}
      </button>
    {/each}
  </nav>
  <span class="spacer"></span>
  {#if store.lastScanAt}
    <span class="time">{t("time_scan")} {formatScanTime(store.lastScanAt)}</span>
  {/if}
  <button
    title={t("title_fetch_all")}
    disabled={store.fetchingAll || store.scanning || !store.boot?.git_available || store.repos.length === 0}
    onclick={() => void store.fetchEverything()}
  >{t("btn_fetch_all")}</button>
  <button
    title={t("title_push_all")}
    disabled={store.pushAllRunning || store.scanning || !store.boot?.git_available || store.pushableCount === 0}
    onclick={() => void store.runPushAll()}
  >
    {#if store.pushQueue}
      {t("push_progress", { cur: store.pushQueue.current, total: store.pushQueue.total })}
    {:else}
      {store.pushableCount > 0 ? t("btn_push_all_n", { n: store.pushableCount }) : t("btn_push_all")}
    {/if}
  </button>
  <button
    disabled={store.scanning || !store.boot?.git_available}
    onclick={() => void store.scan()}
  >{t("btn_scan")}</button>
  <button class="ghost" onclick={() => (store.view = store.view === "settings" ? "list" : "settings")}>
    {t("btn_settings")}
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
