<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { store } from "$lib/store.svelte";
</script>

{#if store.boot && !store.boot.git_available}
  <div class="banner red">
    {t("banner_no_git")}
  </div>
{:else if store.boot && store.boot.config.roots.length === 0 && store.repos.length === 0}
  <div class="banner yellow">
    <span>{t("banner_no_roots")}</span>
    <button class="small" onclick={() => (store.view = "settings")}>{t("btn_settings")}</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 16px;
    font-size: 13px;
    border-bottom: 1px solid var(--border);
  }
  .banner.red {
    background: var(--error-bg);
    color: var(--error-fg);
  }
  .banner.yellow {
    background: var(--banner-yellow-bg);
    color: var(--banner-yellow-fg);
  }
</style>
