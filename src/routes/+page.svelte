<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "$lib/store.svelte";
  import Banner from "$lib/components/Banner.svelte";
  import Header from "$lib/components/Header.svelte";
  import HistoryView from "$lib/components/HistoryView.svelte";
  import MachinesView from "$lib/components/MachinesView.svelte";
  import RepoList from "$lib/components/RepoList.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";

  onMount(() => void store.init());
</script>

{#if store.ready}
  <Banner />
  <Header />
  {#if store.view === "settings"}
    <SettingsView />
  {:else if store.view === "machines"}
    <MachinesView />
  {:else if store.view === "history"}
    <HistoryView />
  {:else}
    <RepoList />
  {/if}
  {#if store.toast}
    <div class="toast">{store.toast}</div>
  {/if}
{:else}
  <div class="loading">Загрузка…</div>
{/if}

<style>
  .loading {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--dim);
  }
  .toast {
    position: fixed;
    bottom: 14px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--toast-bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 16px;
    font-size: 12.5px;
    max-width: 80%;
    box-shadow: 0 6px 20px var(--shadow);
    z-index: 10;
  }
</style>
