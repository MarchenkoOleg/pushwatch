<script lang="ts">
  import { problemChips, repoDotClass } from "$lib/chips";
  import { store } from "$lib/store.svelte";
  import { formatScanTime } from "$lib/time";
  import type { MachineSnapshot } from "$lib/types";

  let expanded = $state<string | null>(null);

  function machineState(m: MachineSnapshot): { cls: string; label: string } {
    const unpushed = m.repos.filter((r) =>
      r.problems.some(
        (p) =>
          p.kind === "ahead" ||
          p.kind === "diverged" ||
          p.kind === "orphan" ||
          p.kind === "upstream_gone" ||
          p.kind === "no_remote",
      ),
    ).length;
    const problems = m.repos.filter((r) => r.state === "problem").length;
    if (m.repos.some((r) => r.state === "error")) return { cls: "gray", label: "есть ошибки" };
    if (unpushed > 0) return { cls: "red", label: `${unpushed} с незапушенными` };
    if (problems > 0) return { cls: "yellow", label: `${problems} проблемных` };
    return { cls: "green", label: `всё запушено · ${m.repos.length} репо` };
  }

  function ageHours(m: MachineSnapshot): number {
    return (Date.now() - new Date(m.generated_at).getTime()) / 3_600_000;
  }
</script>

<div class="machines">
  {#if !store.boot?.config.sync_dir}
    <div class="empty">
      Сводка по машинам отключена — укажите папку синхронизации
      (Syncthing / iCloud / приватный git-репозиторий) в настройках.
      Каждый скан кладёт туда snapshot этой машины, pushwatch читает все.
    </div>
  {:else if store.machines.length === 0}
    <div class="empty">Пока ни одного снапшота в папке синхронизации…</div>
  {:else}
    {#each store.machines as m (m.machine_id)}
      {@const st = machineState(m)}
      {@const stale = ageHours(m) > 48}
      <div class="machine" class:stale>
        <div
          class="m-head"
          class:own={m.machine_id === store.boot?.machine_id}
          role="button"
          tabindex="0"
          onclick={() => (expanded = expanded === m.machine_id ? null : m.machine_id)}
          onkeydown={(e) => e.key === "Enter" && (expanded = expanded === m.machine_id ? null : m.machine_id)}
        >
          <span class="dot {st.cls}"></span>
          <span class="m-name">{m.machine_name}</span>
          {#if m.machine_id === store.boot?.machine_id}
            <span class="own-tag">эта машина</span>
          {/if}
          {#if stale}
            <span class="stale-tag" title="Снапшот старше 2 суток">устарел</span>
          {/if}
          <span class="m-state">{st.label}</span>
          <span class="spacer"></span>
          <span class="m-time">{formatScanTime(m.generated_at)}</span>
          <span class="chevron">{expanded === m.machine_id ? "▾" : "▸"}</span>
        </div>
        {#if expanded === m.machine_id}
          <div class="m-repos">
            {#each m.repos as repo (repo.path)}
              <div class="r-row" class:r-problem={repo.state === "problem"}>
                <span class="dot {repoDotClass(repo)}"></span>
                <span class="r-name">{repo.name}</span>
                <span class="spacer"></span>
                <span class="chips">
                  {#each problemChips(repo) as c, i (i)}
                    <span class="chip {c.cls}" title={c.title}>{c.text}</span>
                  {/each}
                </span>
              </div>
            {:else}
              <div class="r-empty">нет репозиториев</div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .machines {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }
  .empty {
    padding: 48px 24px;
    text-align: center;
    color: var(--dim);
    line-height: 1.6;
  }
  .machine {
    border: 1px solid var(--border);
    border-radius: 8px;
    margin: 6px;
    background: var(--panel);
    overflow: hidden;
  }
  .machine.stale {
    opacity: 0.55;
  }
  .m-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    cursor: pointer;
  }
  .m-head.own {
    background: var(--accent-soft);
  }
  .m-name {
    font-weight: 600;
  }
  .own-tag {
    font-size: 10.5px;
    color: var(--accent);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 1px 6px;
  }
  .stale-tag {
    font-size: 10.5px;
    color: var(--yellow);
    border: 1px solid var(--yellow);
    border-radius: 4px;
    padding: 1px 6px;
  }
  .m-state {
    color: var(--dim);
    font-size: 12.5px;
  }
  .spacer {
    flex: 1;
  }
  .m-time {
    color: var(--dim);
    font-size: 11.5px;
  }
  .chevron {
    color: var(--dim);
    font-size: 12px;
    width: 12px;
  }
  .m-repos {
    border-top: 1px dashed var(--border);
    background: var(--details-bg);
    padding: 4px 12px 8px;
  }
  .r-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-size: 13px;
  }
  .r-problem .r-name {
    font-weight: 600;
  }
  .r-name {
    white-space: nowrap;
  }
  .chips {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .r-empty {
    color: var(--dim);
    font-size: 12px;
    padding: 6px 0;
  }
</style>
