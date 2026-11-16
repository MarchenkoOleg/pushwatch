import { t } from "./i18n.svelte";
import type { Problem } from "./types";

export function problemText(p: Problem): string {
  switch (p.kind) {
    case "dirty": {
      const parts: string[] = [];
      const changes = p.staged + p.unstaged + p.conflicts;
      if (changes) parts.push(t("prob_changes", { n: changes }));
      if (p.untracked) parts.push(t("prob_new", { n: p.untracked }));
      return t("prob_dirty", { parts: parts.join(", ") });
    }
    case "ahead":
      return t("prob_ahead", { branch: p.branch, n: p.ahead, upstream: p.upstream });
    case "diverged":
      return t("prob_diverged", { branch: p.branch, upstream: p.upstream, ahead: p.ahead, behind: p.behind });
    case "no_upstream":
      return t("prob_no_upstream", { branch: p.branch });
    case "no_remote":
      return t("prob_no_remote", { n: p.branches });
    case "upstream_gone":
      return t("prob_gone", { upstream: p.upstream, branch: p.branch });
    case "orphan":
      return t("prob_orphan", { n: p.count });
    case "detached_head":
      return t("prob_detached");
  }
}
