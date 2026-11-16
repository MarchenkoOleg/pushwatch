//! Status engine: collects the full unpushed-state of one repository
//! with 3–4 read-only git calls. Runs on rayon workers — must never
//! block on network (all commands are purely local).

use std::path::Path;

use chrono::Utc;

use crate::gitbin;
use crate::model::{BranchInfo, DirtyFile, Problem, RepoState, RepoStatus, StatusCounts};

/// Cap for the per-repo dirty file list sent to the UI.
const DIRTY_FILES_CAP: usize = 50;

/// OS junk files that can never be meaningful untracked work — they must
/// not make a repository look dirty (`.DS_Store` on macOS, `Thumbs.db` /
/// `desktop.ini` on Windows, AppleDouble `._*` resource forks).
const JUNK_BASENAMES: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini"];

fn is_junk_untracked(path: &str) -> bool {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    JUNK_BASENAMES.contains(&base) || base.starts_with("._")
}

/// Drop junk untracked entries from the parsed status, adjusting counters.
fn strip_junk(files: &mut Vec<(String, String)>, counts: &mut StatusCounts) {
    files.retain(|(label, path)| {
        let junk = label == "??" && is_junk_untracked(path);
        if junk {
            counts.untracked = counts.untracked.saturating_sub(1);
        }
        !junk
    });
}
use crate::parsers::{parse_for_each_ref, parse_porcelain_v2, FER_FORMAT};

/// Collect the status of one repository. Never panics: any git failure
/// becomes `RepoState::Error` with the message preserved for the UI.
pub fn collect_repo(path: &Path) -> RepoStatus {
    let path_str = path.to_string_lossy().into_owned();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str.clone());
    let now = Utc::now();
    match collect_inner(path) {
        Ok((
            state,
            problems,
            branches,
            remotes,
            dirty_files,
            dirty_files_total,
            current_branch,
            detached,
            stashes,
        )) => RepoStatus {
            path: path_str,
            name,
            state,
            problems,
            branches,
            remotes,
            dirty_files,
            dirty_files_total,
            current_branch,
            detached,
            stashes,
            scanned_at: Some(now),
            error: None,
        },
        Err(error) => RepoStatus {
            path: path_str,
            name,
            state: RepoState::Error,
            problems: Vec::new(),
            branches: Vec::new(),
            remotes: Vec::new(),
            dirty_files: Vec::new(),
            dirty_files_total: 0,
            current_branch: None,
            detached: false,
            stashes: 0,
            scanned_at: Some(now),
            error: Some(error),
        },
    }
}

#[allow(clippy::type_complexity)]
type Collected = (
    RepoState,
    Vec<Problem>,
    Vec<BranchInfo>,
    Vec<String>,
    Vec<DirtyFile>,
    u32,
    Option<String>,
    bool,
    u32,
);

fn collect_inner(path: &Path) -> Result<Collected, String> {
    let status_out = gitbin::run_git(
        path,
        &["status", "--porcelain=v2", "--branch", "--untracked-files=normal"],
    )?;
    let mut p = parse_porcelain_v2(&status_out);
    // OS junk (`.DS_Store` etc.) must not make a repo look dirty.
    strip_junk(&mut p.files, &mut p.counts);

    let fer_out = gitbin::run_git(
        path,
        &["for-each-ref", &format!("--format={FER_FORMAT}"), "refs/heads"],
    )?;
    let raw_branches = parse_for_each_ref(&fer_out);

    // Configured remotes (`git remote`) — the source of truth for "can
    // this repo push at all". refs/remotes emptiness alone would mislabel
    // a configured-but-never-fetched remote as "no remote".
    let remotes: Vec<String> = gitbin::run_git(path, &["remote"])?
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Whether any remote-tracking ref has been fetched at all — needed to
    // skip the orphan rev-list safely (with no refs, `--not --remotes`
    // would count every commit as orphan).
    let remote_refs_out = gitbin::run_git(
        path,
        &["for-each-ref", "--count=1", "--format=%(refname)", "refs/remotes"],
    )?;
    let has_remote_refs = !remote_refs_out.is_empty();

    // Unborn HEAD (fresh clone/init, zero commits): porcelain reports
    // `(initial)`; rev-list HEAD would exit 128 and doom the whole repo.
    let unborn = p.head_oid.as_deref() == Some("(initial)") || raw_branches.is_empty();

    // Orphan commits: reachable from HEAD but from neither any remote ref
    // nor any local branch. `--branches` makes the count 0 for attached
    // HEAD (those commits are reachable from their branch — the Ahead
    // problem already covers them) and catches true strays: detached
    // leftovers and commits force-pushed over.
    let head_synced = !p.detached && p.upstream.is_some() && p.ahead == 0;
    let mut orphan_count = 0u32;
    if has_remote_refs && !head_synced && !unborn {
        if let Ok(out) = gitbin::run_git(
            path,
            &["rev-list", "--count", "HEAD", "--not", "--remotes", "--branches"],
        ) {
            orphan_count = out.lines().next().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
        }
        // A rev-list failure degrades to orphan=0 instead of failing the
        // whole repo — status/branch data above stays valid.
    }

    let mut branches: Vec<BranchInfo> = raw_branches
        .iter()
        .map(|b| BranchInfo {
            name: b.name.clone(),
            upstream: b.upstream.clone(),
            remote: b.remote.clone(),
            ahead: if b.gone { 0 } else { b.ahead },
            behind: if b.gone { 0 } else { b.behind },
            gone: b.gone,
            is_head: b.is_head,
        })
        .collect();
    // Interesting branches (unpushed/diverged/gone/never-pushed) first,
    // current branch pinned at the top, then alphabetical.
    fn interest(b: &BranchInfo) -> u8 {
        if b.is_head {
            2
        } else if b.ahead > 0 || b.behind > 0 || b.gone || b.upstream.is_none() {
            1
        } else {
            0
        }
    }
    branches.sort_by(|a, b| {
        interest(b)
            .cmp(&interest(a))
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut problems = Vec::new();
    let counts = p.counts;
    if counts.total() > 0 {
        problems.push(Problem::Dirty {
            staged: counts.staged,
            unstaged: counts.unstaged,
            untracked: counts.untracked,
            conflicts: counts.conflicts,
        });
    }
    if p.detached {
        problems.push(Problem::DetachedHead);
    }
    for b in &raw_branches {
        match &b.upstream {
            Some(upstream) => {
                if b.gone {
                    problems.push(Problem::UpstreamGone {
                        branch: b.name.clone(),
                        upstream: upstream.clone(),
                    });
                } else if b.ahead > 0 && b.behind > 0 {
                    problems.push(Problem::Diverged {
                        branch: b.name.clone(),
                        upstream: upstream.clone(),
                        ahead: b.ahead,
                        behind: b.behind,
                    });
                } else if b.ahead > 0 {
                    problems.push(Problem::Ahead {
                        branch: b.name.clone(),
                        upstream: upstream.clone(),
                        ahead: b.ahead,
                    });
                }
            }
            None => {
                // Branch without upstream is only a problem when a remote
                // is configured at all — otherwise NoRemote covers the repo.
                if !remotes.is_empty() {
                    problems.push(Problem::NoUpstream {
                        branch: b.name.clone(),
                    });
                }
            }
        }
    }
    if orphan_count > 0 {
        problems.push(Problem::Orphan {
            count: orphan_count,
        });
    }
    // A repository with commits but no configured remote cannot be pushed
    // anywhere — that is the loudest possible "not pushed" case.
    if remotes.is_empty() && !raw_branches.is_empty() {
        problems.push(Problem::NoRemote {
            branches: raw_branches.len() as u32,
        });
    }

    let state = if problems.is_empty() {
        RepoState::Ok
    } else {
        RepoState::Problem
    };

    let dirty_files: Vec<DirtyFile> = p
        .files
        .iter()
        .take(DIRTY_FILES_CAP)
        .map(|(label, path)| DirtyFile {
            label: label.clone(),
            path: path.clone(),
        })
        .collect();
    let dirty_files_total = p.files.len() as u32;

    Ok((
        state,
        problems,
        branches,
        remotes,
        dirty_files,
        dirty_files_total,
        p.current_branch,
        p.detached,
        p.stashes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Plain git invocation for test setup (mutations are fine here).
    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("LC_ALL", "C")
            .output()
            .expect("git must be available for tests");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn commit_file(dir: &Path, name: &str, msg: &str) {
        std::fs::write(dir.join(name), format!("{msg}\n")).unwrap();
        git(dir, &["add", name]);
        git(dir, &["commit", "-m", msg, "--quiet"]);
    }

    struct Fixture {
        root: PathBuf,
        work: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    /// `origin.git` (bare) + `work` clone with one pushed commit on `main`.
    fn fresh_repo() -> Fixture {
        let root = std::env::temp_dir().join(format!("pushwatch-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let origin = root.join("origin.git");
        let work = root.join("work");
        git(&root, &["init", "--bare", "--quiet", &origin.to_string_lossy()]);
        git(&root, &["clone", "--quiet", &origin.to_string_lossy(), "work"]);
        git(&work, &["config", "user.email", "test@pushwatch.local"]);
        git(&work, &["config", "user.name", "Pushwatch Test"]);
        // Clone of an empty repo has no branch yet; create and push main.
        std::fs::write(work.join("readme.md"), "init\n").unwrap();
        git(&work, &["add", "."]);
        git(&work, &["commit", "-m", "initial", "--quiet"]);
        git(&work, &["push", "--quiet", "-u", "origin", "HEAD:main"]);
        // Upstream name must be origin/main regardless of local default branch.
        let head = git_capture(&work, &["rev-parse", "--abbrev-ref", "HEAD"]);
        if head != "main" {
            git(&work, &["branch", "--quiet", "-m", "main"]);
        }
        Fixture { root, work }
    }

    fn git_capture(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("LC_ALL", "C")
            .output()
            .expect("git must be available");
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn problem_of<'a>(st: &'a RepoStatus, pred: impl Fn(&Problem) -> bool) -> Option<&'a Problem> {
        st.problems.iter().find(|p| pred(p))
    }

    #[test]
    fn clean_pushed_repo_is_ok() {
        let f = fresh_repo();
        let st = collect_repo(&f.work);
        assert_eq!(st.state, RepoState::Ok, "problems: {:?}", st.problems);
        assert_eq!(st.current_branch.as_deref(), Some("main"));
        let main = st.branches.iter().find(|b| b.name == "main").unwrap();
        assert_eq!(main.upstream.as_deref(), Some("origin/main"));
        assert_eq!(main.ahead, 0);
        assert_eq!(main.behind, 0);
        assert!(!main.gone);
        assert!(main.is_head);
    }

    #[test]
    fn local_commit_is_ahead_without_orphan_dup() {
        let f = fresh_repo();
        commit_file(&f.work, "a.txt", "second");
        let st = collect_repo(&f.work);
        assert_eq!(st.state, RepoState::Problem);
        let ahead = problem_of(&st, |p| matches!(p, Problem::Ahead { .. }))
            .expect("Ahead problem expected");
        match ahead {
            Problem::Ahead { branch, ahead, .. } => {
                assert_eq!(branch, "main");
                assert_eq!(*ahead, 1);
            }
            _ => unreachable!(),
        }
        // Attached HEAD: those commits are reachable from the local branch,
        // so Orphan must NOT duplicate Ahead for the same commits.
        assert!(
            problem_of(&st, |p| matches!(p, Problem::Orphan { .. })).is_none(),
            "Orphan must not fire for attached-HEAD ahead commits: {:?}",
            st.problems
        );
        assert_eq!(st.remotes, vec!["origin".to_string()]);
    }

    #[test]
    fn second_branch_ahead_shows_only_ahead() {
        let f = fresh_repo();
        git(&f.work, &["switch", "--quiet", "-c", "feature"]);
        commit_file(&f.work, "f.txt", "feature work");
        git(&f.work, &["push", "--quiet", "-u", "origin", "feature"]);
        commit_file(&f.work, "f2.txt", "more feature work");
        let st = collect_repo(&f.work);
        let ahead = problem_of(&st, |p| matches!(p, Problem::Ahead { branch, ahead, .. }
            if branch == "feature" && *ahead == 1));
        assert!(ahead.is_some(), "Ahead expected: {:?}", st.problems);
        assert!(
            problem_of(&st, |p| matches!(p, Problem::Orphan { .. })).is_none(),
            "no Orphan for branch-reachable commits: {:?}",
            st.problems
        );
    }

    #[test]
    fn utf8_branch_name_roundtrips() {
        let f = fresh_repo();
        git(&f.work, &["switch", "--quiet", "-c", "ветка-мир"]);
        commit_file(&f.work, "u.txt", "utf8");
        let st = collect_repo(&f.work);
        assert!(
            problem_of(&st, |p| matches!(p, Problem::NoUpstream { branch } if branch == "ветка-мир"))
                .is_some(),
            "UTF-8 branch name must survive parsing: {:?}",
            st.problems
        );
        assert!(st.branches.iter().any(|b| b.name == "ветка-мир"));
    }

    #[test]
    fn diverged_branch_detected_after_remote_moves() {
        let f = fresh_repo();
        // Second clone pushes a commit behind our back.
        let work2 = f.root.join("work2");
        git(
            &f.root,
            &[
                "clone",
                "--quiet",
                &f.root.join("origin.git").to_string_lossy(),
                "work2",
            ],
        );
        git(&work2, &["config", "user.email", "t@t"]);
        git(&work2, &["config", "user.name", "t"]);
        // Clone of a bare repo whose HEAD points at an unborn branch
        // checks out nothing — pin main explicitly.
        git(&work2, &["checkout", "--quiet", "-B", "main", "origin/main"]);
        commit_file(&work2, "remote.txt", "remote change");
        git(&work2, &["push", "--quiet", "origin", "HEAD:main"]);

        // Local unpushed commit.
        commit_file(&f.work, "local.txt", "local change");
        // Fetch brings in the remote-tracking ref that moved ahead.
        git(&f.work, &["fetch", "--quiet", "origin"]);

        let st = collect_repo(&f.work);
        let div = problem_of(&st, |p| matches!(p, Problem::Diverged { branch, ahead, behind, .. }
            if branch == "main" && *ahead == 1 && *behind == 1));
        assert!(div.is_some(), "Diverged expected: {:?}", st.problems);
        assert!(
            problem_of(&st, |p| matches!(p, Problem::Ahead { .. })).is_none(),
            "diverged must not also be Ahead: {:?}",
            st.problems
        );
    }

    #[test]
    fn dirty_files_listed_with_labels() {
        let f = fresh_repo();
        std::fs::write(f.work.join("readme.md"), "modified\n").unwrap();
        std::fs::write(f.work.join("untracked file.txt"), "new\n").unwrap();
        let st = collect_repo(&f.work);
        assert_eq!(st.dirty_files_total, 2);
        assert!(st.dirty_files.iter().any(|f| f.label == ".M" && f.path == "readme.md"));
        assert!(
            st.dirty_files
                .iter()
                .any(|f| f.label == "??" && f.path == "untracked file.txt")
        );
    }

    #[test]
    fn junk_untracked_files_do_not_make_repo_dirty() {
        let f = fresh_repo();
        std::fs::write(f.work.join(".DS_Store"), "junk").unwrap();
        std::fs::write(f.work.join("._resource"), "junk").unwrap();
        let st = collect_repo(&f.work);
        assert_eq!(
            st.state,
            RepoState::Ok,
            "only-junk untracked files must not be a problem: {:?}",
            st.problems
        );
        assert!(st.dirty_files.is_empty());
        assert_eq!(st.dirty_files_total, 0);

        // A real untracked file still counts, junk stays hidden.
        std::fs::write(f.work.join("real.txt"), "work").unwrap();
        let st = collect_repo(&f.work);
        assert_eq!(st.dirty_files_total, 1);
        assert!(st.dirty_files.iter().all(|f| f.path == "real.txt"));
    }

    #[test]
    fn branches_sorted_interesting_first() {
        let f = fresh_repo();
        // `feature` never pushed (interesting), `arch` synced (boring).
        git(&f.work, &["switch", "--quiet", "-c", "feature"]);
        commit_file(&f.work, "f.txt", "feature work");
        git(&f.work, &["push", "--quiet", "-u", "origin", "feature"]);
        git(&f.work, &["switch", "--quiet", "-c", "arch"]);
        commit_file(&f.work, "a.txt", "arch work");
        git(&f.work, &["push", "--quiet", "-u", "origin", "arch"]);
        // Local unpushed commit on arch → interesting.
        commit_file(&f.work, "a2.txt", "arch work 2");
        git(&f.work, &["switch", "--quiet", "main"]);

        let st = collect_repo(&f.work);
        let names: Vec<&str> = st.branches.iter().map(|b| b.name.as_str()).collect();
        // main (HEAD, synced), arch (ahead 1), feature (synced, boring) last.
        assert_eq!(
            names,
            vec!["main", "arch", "feature"],
            "interesting branches must come first: {names:?}"
        );
    }

    #[test]
    fn unborn_head_with_configured_remote_is_ok() {        let root = std::env::temp_dir().join(format!("pushwatch-unborn-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let repo = root.join("unborn");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "--quiet"]);
        git(&repo, &["remote", "add", "origin", "https://example.com/x.git"]);
        let st = collect_repo(&repo);
        assert_eq!(
            st.state,
            RepoState::Ok,
            "unborn repo with no commits must not error or NoRemote: {:?} err={:?}",
            st.problems,
            st.error
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn dirty_worktree_detected() {
        let f = fresh_repo();
        std::fs::write(f.work.join("readme.md"), "modified\n").unwrap();
        std::fs::write(f.work.join("new.txt"), "untracked\n").unwrap();
        let st = collect_repo(&f.work);
        let dirty = problem_of(&st, |p| matches!(p, Problem::Dirty { .. })).unwrap();
        match dirty {
            Problem::Dirty { unstaged, untracked, staged, conflicts } => {
                assert_eq!(*unstaged, 1);
                assert_eq!(*untracked, 1);
                assert_eq!(*staged, 0);
                assert_eq!(*conflicts, 0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn unpushed_branch_without_upstream() {
        let f = fresh_repo();
        git(&f.work, &["switch", "--quiet", "-c", "feature"]);
        commit_file(&f.work, "f.txt", "feature work");
        let st = collect_repo(&f.work);
        assert!(
            problem_of(&st, |p| matches!(p, Problem::NoUpstream { branch } if branch == "feature"))
                .is_some(),
            "NoUpstream expected: {:?}",
            st.problems
        );
    }

    #[test]
    fn remoteless_repo_is_no_remote_problem() {
        let root = std::env::temp_dir().join(format!("pushwatch-solo-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let repo = root.join("solo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "--quiet"]);
        git(&repo, &["config", "user.email", "t@t"]);
        git(&repo, &["config", "user.name", "t"]);
        commit_file(&repo, "x.txt", "solo");
        let st = collect_repo(&repo);
        // No `no_upstream` noise, but an explicit no_remote problem.
        assert!(
            problem_of(&st, |p| matches!(p, Problem::NoUpstream { .. })).is_none(),
            "no_upstream must not fire for remoteless repo: {:?}",
            st.problems
        );
        assert!(
            problem_of(&st, |p| matches!(p, Problem::NoRemote { branches: 1 })).is_some(),
            "NoRemote expected: {:?}",
            st.problems
        );
        assert_eq!(st.state, RepoState::Problem);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_remoteless_repo_is_ok() {
        let root = std::env::temp_dir().join(format!("pushwatch-empty-{}", uuid::Uuid::new_v4()));
        let repo = root.join("empty");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "--quiet"]);
        let st = collect_repo(&repo);
        assert_eq!(
            st.state,
            RepoState::Ok,
            "empty remoteless repo must not be a problem: {:?}",
            st.problems
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn deleted_remote_branch_is_upstream_gone() {
        let f = fresh_repo();
        git(&f.work, &["switch", "--quiet", "-c", "feature"]);
        commit_file(&f.work, "f.txt", "feature work");
        git(&f.work, &["push", "--quiet", "-u", "origin", "feature"]);
        git(&f.work, &["push", "--quiet", "origin", ":feature"]);
        git(&f.work, &["switch", "--quiet", "main"]);
        let st = collect_repo(&f.work);
        assert!(
            problem_of(&st, |p| matches!(p, Problem::UpstreamGone { branch, .. } if branch == "feature"))
                .is_some(),
            "UpstreamGone expected: {:?}",
            st.problems
        );
    }

    #[test]
    fn detached_head_is_reported() {
        let f = fresh_repo();
        commit_file(&f.work, "a.txt", "second");
        let oid = git_capture(&f.work, &["rev-parse", "HEAD"]);
        // Move branch back and leave HEAD detached on the new commit.
        git(&f.work, &["reset", "--hard", "--quiet", "HEAD~1"]);
        git(&f.work, &["checkout", "--quiet", "--detach", &oid]);
        let st = collect_repo(&f.work);
        assert!(st.detached);
        assert!(st.current_branch.is_none());
        assert!(problem_of(&st, |p| matches!(p, Problem::DetachedHead)).is_some());
        // The commit exists on no branch and no remote → orphan.
        assert!(problem_of(&st, |p| matches!(p, Problem::Orphan { count: 1 })).is_some());
    }

    #[test]
    fn broken_repo_is_error_not_panic() {
        let root = std::env::temp_dir().join(format!("pushwatch-broken-{}", uuid::Uuid::new_v4()));
        let repo = root.join("broken");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".git").join("HEAD"), "garbage").unwrap();
        let st = collect_repo(&repo);
        assert_eq!(st.state, RepoState::Error);
        assert!(st.error.is_some());
        std::fs::remove_dir_all(&root).ok();
    }
}
