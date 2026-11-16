// Problem → chip mapping, shared between the repo list and the machines view.
// Chip glyphs are language-neutral; tooltips are localized.
import { t } from "./i18n.svelte";
import type { Problem, RepoStatus } from "./types";

export interface Chip {
  text: string;
  cls: string;
  title: string;
}

export function problemChips(repo: RepoStatus): Chip[] {
  const out: Chip[] = [];
  for (const p of repo.problems) {
    switch (p.kind) {
      case "ahead":
        out.push({
          text: `↑${p.ahead}`,
          cls: "red",
          title: t("chip_ahead_title", { branch: p.branch, n: p.ahead, upstream: p.upstream }),
        });
        break;
      case "diverged":
        out.push({
          text: `⇅${p.ahead}/${p.behind}`,
          cls: "red",
          title: t("chip_diverged_title", { branch: p.branch, upstream: p.upstream }),
        });
        break;
      case "dirty": {
        const changes = p.staged + p.unstaged + p.conflicts;
        if (changes > 0) {
          out.push({ text: `~${changes}`, cls: "yellow", title: t("chip_dirty_title", { n: changes }) });
        }
        if (p.untracked > 0) {
          out.push({ text: `?${p.untracked}`, cls: "yellow", title: t("chip_new_title", { n: p.untracked }) });
        }
        break;
      }
      case "no_upstream":
        out.push({
          text: `∅ ${p.branch}`,
          cls: "gray",
          title: t("chip_no_upstream_title"),
        });
        break;
      case "no_remote":
        out.push({ text: "no remote", cls: "purple", title: t("chip_no_remote_title") });
        break;
      case "upstream_gone":
        out.push({ text: `✕ ${p.branch}`, cls: "purple", title: t("chip_gone_title", { upstream: p.upstream }) });
        break;
      case "orphan":
        out.push({ text: `orphan ${p.count}`, cls: "purple", title: t("chip_orphan_title", { n: p.count }) });
        break;
      case "detached_head":
        out.push({ text: "detached", cls: "gray", title: t("chip_detached_title") });
        break;
    }
  }
  return out;
}

export function repoDotClass(repo: RepoStatus): string {
  if (repo.state === "error") return "gray";
  if (repo.state === "ok") return "green";
  return repo.problems.some(
    (p) =>
      p.kind === "ahead" ||
      p.kind === "diverged" ||
      p.kind === "orphan" ||
      p.kind === "upstream_gone" ||
      p.kind === "no_remote",
  )
    ? "red"
    : "yellow";
}
