<script lang="ts">
  import { revealPath, saveConfig } from "$lib/api";
  import { problemChips, repoDotClass } from "$lib/chips";
  import { t } from "$lib/i18n.svelte";
  import { store } from "$lib/store.svelte";
  import type { RepoStatus } from "$lib/types";
  import RepoDetails from "./RepoDetails.svelte";

  let { repo }: { repo: RepoStatus } = $props();

  const open = $derived(store.expandedPath === repo.path);
  const dotCls = $derived(repoDotClass(repo));
  const chips = $derived(problemChips(repo));

  async function hide() {
    const cfg = store.config();
    if (!cfg.hidden_repos.includes(repo.path)) {
      cfg.hidden_repos = [...cfg.hidden_repos, repo.path];
    }
    try {
      const saved = await saveConfig(cfg);
      store.applyConfig(saved);
      store.repos = store.repos.filter((r) => r.path !== repo.path);
    } catch (e) {
      // Revert the optimistic change so memory matches disk.
      cfg.hidden_repos = cfg.hidden_repos.filter((p) => p !== repo.path);
      store.notify(t("err_hide", { e: String(e) }));
    }
  }
</script>

<div class="row-wrap">
  <div
    class="row"
    class:open
    onclick={() => (store.expandedPath = open ? null : repo.path)}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === "Enter" && (store.expandedPath = open ? null : repo.path)}
  >
    <span class="dot {dotCls}"></span>
    <span class="name">{repo.name}</span>
    <span class="path">{repo.path}</span>
    <span class="spacer"></span>
    <span class="chips">
      {#each chips as c, i (i)}
        <span class="chip {c.cls}" title={c.title}>{c.text}</span>
      {/each}
    </span>
    <button
      class="icon"
      title={t("title_reveal")}
      onclick={(e) => {
        e.stopPropagation();
        void revealPath(repo.path);
      }}
    >↗</button>
    <button
      class="icon"
      title={t("title_hide")}
      onclick={(e) => {
        e.stopPropagation();
        void hide();
      }}
    >✕</button>
    <span class="chevron">{open ? "▾" : "▸"}</span>
  </div>
  {#if open}
    <RepoDetails {repo} />
  {/if}
</div>

<style>
  .row-wrap {
    border: 1px solid var(--border);
    border-radius: 8px;
    margin: 6px 8px;
    background: var(--panel);
    overflow: hidden;
  }
  .row-wrap:has(.row.open) {
    border-color: var(--btn-hover-border);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    cursor: pointer;
  }
  .row:hover {
    background: var(--panel-hover);
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
  }
  .path {
    color: var(--dim);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 40%;
  }
  .spacer {
    flex: 1;
  }
  .chips {
    display: flex;
    gap: 5px;
    flex-shrink: 0;
  }
  button.icon {
    background: transparent;
    border: none;
    padding: 2px 6px;
    color: var(--dim);
    font-size: 13px;
  }
  button.icon:hover {
    color: var(--text);
  }
  .chevron {
    color: var(--dim);
    font-size: 12px;
    width: 12px;
  }
</style>
