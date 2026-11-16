// Problem → chip mapping, shared between the repo list and the machines view.
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
          title: `${p.branch}: ${p.ahead} не запушено в ${p.upstream}`,
        });
        break;
      case "diverged":
        out.push({
          text: `⇅${p.ahead}/${p.behind}`,
          cls: "red",
          title: `${p.branch}: расходится с ${p.upstream} — нужен pull --rebase`,
        });
        break;
      case "dirty": {
        const changes = p.staged + p.unstaged + p.conflicts;
        if (changes > 0) {
          out.push({ text: `~${changes}`, cls: "yellow", title: `изменённых файлов: ${changes}` });
        }
        if (p.untracked > 0) {
          out.push({ text: `?${p.untracked}`, cls: "yellow", title: `новых (untracked): ${p.untracked}` });
        }
        break;
      }
      case "no_upstream":
        out.push({
          text: `∅ ${p.branch}`,
          cls: "gray",
          title: "ветка ни разу не запушена — upstream не задан",
        });
        break;
      case "no_remote":
        out.push({ text: "no remote", cls: "purple", title: "remote не настроен — репозиторий живёт только на этой машине" });
        break;
      case "upstream_gone":
        out.push({ text: `✕ ${p.branch}`, cls: "purple", title: `remote-ветка ${p.upstream} удалена или не получена` });
        break;
      case "orphan":
        out.push({ text: `orphan ${p.count}`, cls: "purple", title: `коммитов вне всех remote и веток: ${p.count}` });
        break;
      case "detached_head":
        out.push({ text: "detached", cls: "gray", title: "HEAD вне ветки" });
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
