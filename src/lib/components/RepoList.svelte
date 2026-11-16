<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { store } from "$lib/store.svelte";
  import RepoRow from "./RepoRow.svelte";
</script>

<div class="toolbar">
  <div class="tabs">
    <button class:active={store.filter === "problems"} onclick={() => (store.filter = "problems")}>
      {t("filter_problems")} <span class="count">{store.problemCount}</span>
    </button>
    <button class:active={store.filter === "all"} onclick={() => (store.filter = "all")}>
      {t("filter_all")} <span class="count">{store.repos.length}</span>
    </button>
  </div>
  <input type="text" placeholder={t("search_ph")} bind:value={store.search} />
</div>

<div class="list">
  {#if store.visibleRepos.length === 0}
    {#if store.repos.length === 0}
      <div class="empty">{t("empty_no_repos")}</div>
    {:else if store.filter === "problems" && store.problemCount === 0}
      <div class="empty ok">{t("empty_all_pushed")}</div>
    {:else}
      <div class="empty">{t("empty_filter")}</div>
    {/if}
  {:else}
    {#each store.visibleRepos as repo (repo.path)}
      <RepoRow {repo} />
    {/each}
  {/if}
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
  }
  .tabs {
    display: flex;
    gap: 6px;
  }
  .tabs button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .count {
    opacity: 0.7;
    margin-left: 4px;
  }
  .toolbar input {
    flex: 1;
    max-width: 340px;
    margin-left: auto;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 16px;
  }
  .empty {
    padding: 48px 20px;
    text-align: center;
    color: var(--dim);
  }
  .empty.ok {
    color: var(--green);
    font-size: 16px;
  }
</style>
