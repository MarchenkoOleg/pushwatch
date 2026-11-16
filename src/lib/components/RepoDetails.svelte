<script lang="ts">
  import { revealPath } from "$lib/api";
  import { t } from "$lib/i18n.svelte";
  import { store } from "$lib/store.svelte";
  import { problemText } from "$lib/problems";
  import { formatScanTime } from "$lib/time";
  import type { BranchInfo, RepoStatus } from "$lib/types";

  let { repo }: { repo: RepoStatus } = $props();

  const pushState = $derived(store.pushes[repo.path]);
  const fetchState = $derived(store.fetches[repo.path]);
  let showAllFiles = $state(false);
  const shownFiles = $derived(showAllFiles ? repo.dirty_files : repo.dirty_files.slice(0, 12));

  /** Push always targets an explicit remote + branch: a bare `git push`
   * only pushes the *current* branch under push.default=simple and would
   * falsely report success for any other branch. */
  function pushTarget(b: BranchInfo): { remote: string | null; setUpstream: boolean } {
    const remote = b.remote ?? repo.remotes[0] ?? null;
    const setUpstream = !b.upstream || b.gone;
    return { remote, setUpstream };
  }

  /** Nothing to push when the branch tracks an existing upstream and has
   * no unpushed commits — a push would be a harmless no-op. */
  function pushable(b: BranchInfo): boolean {
    const { remote } = pushTarget(b);
    if (!remote) return false;
    if (b.gone || !b.upstream) return true;
    return b.ahead > 0;
  }

  function pushTitle(b: BranchInfo): string {
    const { remote, setUpstream } = pushTarget(b);
    if (!remote) return t("no_remote_title");
    if (!pushable(b)) return t("synced_title");
    return `git push${setUpstream ? " -u" : ""} ${remote} -- ${b.name}`;
  }
</script>

<div class="details">
  <div class="actions">
    <button
      class="small"
      disabled={fetchState?.running}
      title={t("title_fetch_repo")}
      onclick={() => void store.fetch(repo.path)}
    >
      {#if fetchState?.running}<span class="dot gray pulse"></span>{/if}
      Fetch
    </button>
    {#if repo.scanned_at}
      <span class="scanned">{t("time_scan")} {formatScanTime(repo.scanned_at)}</span>
    {/if}
    {#if pushState?.running || fetchState?.running}
      <span class="proc-label"><span class="dot gray pulse"></span> {pushState?.running ? t("running_push") : t("running_fetch")}</span>
    {/if}
  </div>

  {#if repo.error}
    <div class="error-box">{repo.error}</div>
  {/if}

  <div class="problems-list">
    {#each repo.problems as p}
      <div class="problem">{problemText(p)}</div>
    {/each}
    {#if repo.problems.length === 0 && !repo.error}
      <div class="problem ok">{t("no_problems")}</div>
    {/if}
  </div>

  {#if repo.dirty_files.length > 0}
    <div class="files">
      <div class="files-title">
        {t("files_title", { n: repo.dirty_files_total })}
        <button
          class="small ghost"
          onclick={() => (showAllFiles = !showAllFiles)}
        >{showAllFiles ? t("collapse") : t("show_all")}</button>
      </div>
      <ul>
        {#each shownFiles as f (f.label + f.path)}
          <li>
            <span class="label">{f.label}</span>
            <button
              class="file"
              title={t("title_reveal")}
              onclick={() => {
                const sep = repo.path.includes("\\") ? "\\" : "/";
                void revealPath(`${repo.path}${sep}${f.path.replaceAll("/", sep)}`);
              }}
            >{f.path}</button>
          </li>
        {/each}
        {#if repo.dirty_files_total > repo.dirty_files.length}
          <li class="more">{t("more_files", { n: repo.dirty_files_total - repo.dirty_files.length })}</li>
        {/if}
      </ul>
    </div>
  {/if}

  {#if repo.branches.length > 0}
    <table class="branches">
      <thead>
        <tr><th>{t("th_branch")}</th><th>{t("th_upstream")}</th><th>↑ / ↓</th><th></th></tr>
      </thead>
      <tbody>
        {#each repo.branches as b (b.name)}
          {@const target = pushTarget(b)}
          <tr>
            <td>
              {#if b.is_head}<span class="head-mark">●</span>{/if}
              {b.name}
              {#if b.gone}<span class="gone">{t("gone_mark")}</span>{/if}
            </td>
            <td>{b.upstream ?? "—"}</td>
            <td class="num" class:ahead={b.ahead > 0} class:behind={b.behind > 0}>
              {b.ahead} / {b.behind}
            </td>
            <td>
              <button
                class="small"
                disabled={pushState?.running || !pushable(b)}
                title={pushTitle(b)}
                onclick={() =>
                  store.push(repo.path, b.name, target.remote, target.setUpstream)}
              >Push</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#if repo.stashes > 0}
    <div class="note">{t("stash_note", { n: repo.stashes })}</div>
  {/if}

  {#if (pushState?.lines.length ?? 0) + (fetchState?.lines.length ?? 0) > 0}
    <div class="proc-output" class:failed={pushState?.ok === false || fetchState?.ok === false}>
      <div class="proc-title">{t("proc_title")}</div>
      <pre>{(pushState?.lines ?? []).concat(fetchState?.lines ?? []).join("\n") || "…"}</pre>
    </div>
  {/if}
</div>

<style>
  .details {
    padding: 10px 14px 14px 32px;
    border-top: 1px dashed var(--border);
    background: var(--details-bg);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .proc-label {
    color: var(--dim);
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .error-box {
    background: var(--error-bg);
    border: 1px solid var(--error-border);
    color: var(--error-fg);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 12px;
    white-space: pre-wrap;
    user-select: text;
  }
  .problems-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }
  .problem.ok {
    color: var(--green);
  }
  .files {
    font-size: 12.5px;
  }
  .files-title {
    color: var(--dim);
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .files ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 220px;
    overflow-y: auto;
  }
  .files li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 1px 0;
  }
  .label {
    font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
    font-size: 11px;
    color: var(--yellow);
    width: 26px;
    flex-shrink: 0;
    text-align: right;
  }
  button.file {
    background: none;
    border: none;
    padding: 0;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    user-select: text;
  }
  button.file:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  li.more {
    color: var(--dim);
  }
  .branches {
    width: 100%;
    border-collapse: collapse;
    font-size: 12.5px;
  }
  .branches th {
    text-align: left;
    color: var(--dim);
    font-weight: 500;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
  }
  .branches td {
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
  }
  .head-mark {
    color: var(--accent);
    margin-right: 4px;
  }
  .gone {
    color: var(--gone-fg);
    font-size: 11px;
    margin-left: 6px;
  }
  td.num {
    font-variant-numeric: tabular-nums;
  }
  td.ahead {
    color: var(--error-fg);
  }
  td.behind {
    color: var(--behind-fg);
  }
  .note {
    color: var(--dim);
    font-size: 12px;
  }
  .proc-output {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    overflow: hidden;
  }
  .proc-output.failed {
    border-color: var(--error-border);
  }
  .proc-title {
    padding: 6px 10px;
    font-size: 12px;
    color: var(--dim);
    border-bottom: 1px solid var(--border);
  }
  pre {
    margin: 0;
    padding: 8px 10px;
    max-height: 200px;
    overflow-y: auto;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--dim);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  .scanned {
    color: var(--dim);
    font-size: 11px;
  }
</style>
