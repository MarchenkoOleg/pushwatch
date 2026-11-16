<script lang="ts">
  import { store } from "$lib/store.svelte";
  import RepoRow from "./RepoRow.svelte";
</script>

<div class="toolbar">
  <div class="tabs">
    <button class:active={store.filter === "problems"} onclick={() => (store.filter = "problems")}>
      Проблемные <span class="count">{store.problemCount}</span>
    </button>
    <button class:active={store.filter === "all"} onclick={() => (store.filter = "all")}>
      Все <span class="count">{store.repos.length}</span>
    </button>
  </div>
  <input type="text" placeholder="Поиск по имени или пути…" bind:value={store.search} />
</div>

<div class="list">
  {#if store.visibleRepos.length === 0}
    {#if store.repos.length === 0}
      <div class="empty">Репозитории не найдены. Добавьте корневые папки в настройках.</div>
    {:else if store.filter === "problems" && store.problemCount === 0}
      <div class="empty ok">✓ Всё запушено</div>
    {:else}
      <div class="empty">Ничего не найдено по фильтру.</div>
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
