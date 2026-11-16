import type { Problem } from "./types";

export function problemText(p: Problem): string {
  switch (p.kind) {
    case "dirty": {
      const parts: string[] = [];
      const changes = p.staged + p.unstaged + p.conflicts;
      if (changes) parts.push(`изменённых: ${changes}`);
      if (p.untracked) parts.push(`новых: ${p.untracked}`);
      return `Незакоммиченные изменения (${parts.join(", ")})`;
    }
    case "ahead":
      return `Ветка ${p.branch}: ${p.ahead} коммит(ов) не запушено в ${p.upstream}`;
    case "diverged":
      return `Ветка ${p.branch} расходится с ${p.upstream}: впереди ${p.ahead}, позади ${p.behind} — нужен pull --rebase перед push`;
    case "no_upstream":
      return `Ветка ${p.branch} ни разу не запушена`;
    case "no_remote":
      return `Remote не настроен — ${p.branches} ветка(и) существуют только на этой машине`;
    case "upstream_gone":
      return `Remote-ветка ${p.upstream} отсутствует (удалена или ещё не получена) — локальная ветка ${p.branch}`;
    case "orphan":
      return `${p.count} коммит(ов) не видны ни на одном remote`;
    case "detached_head":
      return "HEAD в detached-состоянии (вне ветки)";
  }
}
