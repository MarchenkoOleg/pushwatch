<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n.svelte";
  import { store } from "$lib/store.svelte";
  import { formatScanTime } from "$lib/time";

  onMount(() => void store.loadHistory());

  const kindLabel: Record<string, () => string> = {
    scan: () => t("kind_scan"),
    push: () => "push",
    fetch: () => "fetch",
  };
</script>

<div class="history">
  <div class="toolbar">
    <button class="small" onclick={() => void store.loadHistory()}>{t("btn_refresh")}</button>
    <span class="hint">{t("hist_hint")}</span>
  </div>
  {#if store.history.length === 0}
    <div class="empty">{t("hist_empty")}</div>
  {:else}
    <table>
      <thead>
        <tr><th>{t("th_time")}</th><th>{t("th_kind")}</th><th>{t("th_event")}</th></tr>
      </thead>
      <tbody>
        {#each store.history as h, i (i)}
          <tr class:failed={h.ok === false}>
            <td class="time">{formatScanTime(h.ts)}</td>
            <td><span class="kind kind-{h.kind}">{kindLabel[h.kind]?.() ?? h.kind}</span></td>
            <td class="msg">{h.message}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .history {
    flex: 1;
    overflow-y: auto;
    padding: 8px 16px 24px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0 10px;
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
  }
  .empty {
    padding: 48px;
    text-align: center;
    color: var(--dim);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
  }
  th {
    text-align: left;
    color: var(--dim);
    font-weight: 500;
    padding: 4px 10px;
    border-bottom: 1px solid var(--border);
  }
  td {
    padding: 5px 10px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }
  tr.failed .msg {
    color: var(--error-fg);
  }
  .time {
    color: var(--dim);
    white-space: nowrap;
    width: 130px;
    font-variant-numeric: tabular-nums;
  }
  .kind {
    font-size: 10.5px;
    border-radius: 4px;
    padding: 1px 6px;
    border: 1px solid var(--border);
    color: var(--dim);
    white-space: nowrap;
  }
  .kind-push {
    color: var(--kind-push);
    border-color: var(--chip-green-border);
  }
  .kind-fetch {
    color: var(--kind-fetch);
    border-color: var(--behind-fg);
  }
  .msg {
    user-select: text;
  }
</style>
