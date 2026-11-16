//! Parsers for git porcelain output. Pure functions — no I/O, fully unit-tested.
//!
//! Locale safety: the engine runs git with `LC_ALL=C`, so messages parsed
//! here (`[ahead 2]` etc.) are always English regardless of user locale.

use crate::model::StatusCounts;

/// Everything `git status --porcelain=v2 --branch` tells us about HEAD.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PorcelainV2 {
    pub head_oid: Option<String>,
    /// Short branch name; `None` when detached.
    pub current_branch: Option<String>,
    pub detached: bool,
    /// Short upstream name for the current branch.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub counts: StatusCounts,
    pub stashes: u32,
    /// Per-file entries: (XY label, path). Paths are raw (spaces preserved).
    pub files: Vec<(String, String)>,
}

/// Split off `n` whitespace-separated tokens, return (first token, rest).
/// The rest preserves spaces — porcelain v2 keeps the path as the last
/// field, unescaped. With `splitn(n + 1, ' ')` the remainder is exactly
/// the last piece; skip the n-1 middle tokens after consuming the first.
fn take_fields(rest: &str, n: usize) -> (String, String) {
    let mut it = rest.splitn(n + 1, ' ');
    let first = it.next().unwrap_or("").to_string();
    let remainder = it.skip(n.saturating_sub(1)).next().unwrap_or("").to_string();
    (first, remainder)
}

/// Parse `git status --porcelain=v2 --branch [--untracked-files=normal]` output.
pub fn parse_porcelain_v2(output: &str) -> PorcelainV2 {
    let mut out = PorcelainV2::default();
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            parse_header(rest, &mut out);
        } else if let Some(path) = line.strip_prefix("? ") {
            if !path.is_empty() {
                out.counts.untracked += 1;
                out.files.push(("??".to_string(), path.to_string()));
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path> — 7 tokens then path.
            let (xy, path) = take_fields(rest, 7);
            if xy_is_staged(rest) {
                out.counts.staged += 1;
            }
            if xy_is_unstaged(rest) {
                out.counts.unstaged += 1;
            }
            if !path.is_empty() {
                out.files.push((xy, path));
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <X><score> <path><TAB><orig>
            // — X and score are concatenated into ONE token ("R100").
            let (xy, paths) = take_fields(rest, 8);
            let path = paths.split('\t').next().unwrap_or("").to_string();
            if xy_is_staged(rest) {
                out.counts.staged += 1;
            }
            if xy_is_unstaged(rest) {
                out.counts.unstaged += 1;
            }
            if !path.is_empty() {
                out.files.push((xy, path));
            }
        } else if let Some(rest) = line.strip_prefix("u ") {
            // u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path> — 9 tokens.
            let (xy, path) = take_fields(rest, 9);
            out.counts.conflicts += 1;
            if !path.is_empty() {
                out.files.push((xy, path));
            }
        }
        // "! " ignored entries never appear without --ignored; skip anything else.
    }
    out
}

fn parse_header(rest: &str, out: &mut PorcelainV2) {
    let mut parts = rest.splitn(2, ' ');
    let key = parts.next().unwrap_or("");
    let value = parts.next().unwrap_or("").trim();
    match key {
        "branch.oid" => out.head_oid = Some(value.to_string()),
        "branch.head" => {
            if value == "(detached)" {
                out.detached = true;
                out.current_branch = None;
            } else {
                out.current_branch = Some(value.to_string());
            }
        }
        "branch.upstream" => out.upstream = Some(value.to_string()),
        "branch.ab" => {
            for token in value.split_whitespace() {
                if let Some(n) = token.strip_prefix('+').and_then(|s| s.parse().ok()) {
                    out.ahead = n;
                } else if let Some(n) = token.strip_prefix('-').and_then(|s| s.parse().ok()) {
                    out.behind = n;
                }
            }
        }
        "stash" => out.stashes = value.parse().unwrap_or(0),
        _ => {}
    }
}

/// For entries `1 XY ...` / `2 XY ...`: first field after the prefix is XY.
fn xy_of(rest: &str) -> (char, char) {
    let mut it = rest.split_whitespace();
    let xy = it.next().unwrap_or("..");
    let mut chars = xy.chars();
    let x = chars.next().unwrap_or('.');
    let y = chars.next().unwrap_or('.');
    (x, y)
}

fn xy_is_staged(rest: &str) -> bool {
    let (x, _) = xy_of(rest);
    x != '.' && x != '?'
}

fn xy_is_unstaged(rest: &str) -> bool {
    let (_, y) = xy_of(rest);
    y != '.' && y != '?'
}

/// Raw per-branch data from `git for-each-ref refs/heads`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ForEachRefBranch {
    pub name: String,
    pub upstream: Option<String>,
    /// Exact remote name from `%(upstream:remotename)` (handles remotes
    /// whose names contain `/` — never guess by splitting the upstream).
    pub remote: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub gone: bool,
    pub is_head: bool,
}

/// Field separator for for-each-ref: ASCII unit separator, which cannot
/// appear in ref names.
pub const FER_FIELD_SEP: char = '\u{1f}';

/// Format string used by the engine so parsing stays in sync with invocation.
pub const FER_FORMAT: &str = "%(refname:short)%1f%(upstream:short)%1f%(upstream:track)%1f%(HEAD)%1f%(upstream:remotename)";

/// Parse `for-each-ref --format=<FER_FORMAT> refs/heads` output.
pub fn parse_for_each_ref(output: &str) -> Vec<ForEachRefBranch> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split(FER_FIELD_SEP).collect();
            let (ahead, behind, gone) = parse_track(fields.get(2).copied().unwrap_or(""));
            ForEachRefBranch {
                name: fields.first().copied().unwrap_or("").trim().to_string(),
                upstream: fields
                    .get(1)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                remote: fields
                    .get(4)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                ahead,
                behind,
                gone,
                is_head: fields.get(3).map(|s| s.trim() == "*").unwrap_or(false),
            }
        })
        .collect()
}

/// Parse `%(upstream:track)` — e.g. ``, `[ahead 2]`, `[behind 3]`,
/// `[ahead 1, behind 4]`, `[gone]`. Locale-independent at `LC_ALL=C`.
pub fn parse_track(track: &str) -> (u32, u32, bool) {
    let mut ahead = 0;
    let mut behind = 0;
    let mut gone = false;
    let inner = track.trim().trim_start_matches('[').trim_end_matches(']');
    for token in inner.split(',') {
        let token = token.trim();
        if let Some(n) = token.strip_prefix("ahead ").and_then(|s| s.trim().parse().ok()) {
            ahead = n;
        } else if let Some(n) = token.strip_prefix("behind ").and_then(|s| s.trim().parse().ok()) {
            behind = n;
        } else if token == "gone" {
            gone = true;
        }
    }
    (ahead, behind, gone)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_STATUS: &str = "\
# branch.oid 8f4b1e0c9d2a3b4c5d6e7f8091a2b3c4d5e6f708
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -1
# stash 3
1 .M N... 000000 000000 000000 000000 000000 src/main.rs
1 M. N... 000000 000000 000000 000000 000000 src/lib.rs
2 R. N... 000000 000000 000000 000000 000000 R100 new.rs\told.rs
? junk.txt
? notes.md
";

    #[test]
    fn porcelain_v2_full_sample() {
        let p = parse_porcelain_v2(SAMPLE_STATUS);
        assert_eq!(p.current_branch.as_deref(), Some("main"));
        assert_eq!(p.upstream.as_deref(), Some("origin/main"));
        assert_eq!(p.ahead, 2);
        assert_eq!(p.behind, 1);
        assert_eq!(p.stashes, 3);
        assert_eq!(p.counts.staged, 2, "M. + R.");
        assert_eq!(p.counts.unstaged, 1, ".M");
        assert_eq!(p.counts.untracked, 2);
        assert_eq!(p.counts.conflicts, 0);
        assert!(!p.detached);
    }

    #[test]
    fn porcelain_v2_detached_no_upstream() {
        let out = "\
# branch.oid 8f4b1e0
# branch.head (detached)
1 M. N... 000000 000000 000000 000000 000000 a.txt
";
        let p = parse_porcelain_v2(out);
        assert!(p.detached);
        assert!(p.current_branch.is_none());
        assert!(p.upstream.is_none());
        assert_eq!(p.ahead, 0);
        assert_eq!(p.counts.staged, 1);
    }

    #[test]
    fn porcelain_v2_unmerged_counts_as_conflict() {
        let out = "\
# branch.head main
u uu N... 000000 000000 000000 000000 000000 000000 000000 c.txt
1 .M N... 000000 000000 000000 000000 000000 b.txt
";
        let p = parse_porcelain_v2(out);
        assert_eq!(p.counts.conflicts, 1);
        assert_eq!(p.counts.unstaged, 1);
        assert_eq!(p.counts.staged, 0);
    }

    #[test]
    fn porcelain_v2_files_with_paths() {
        let out = "\
# branch.head main
1 .M N... 000000 000000 000000 000000 000000 src/main.rs
2 R. N... 000000 000000 000000 000000 000000 R100 new name.rs\told name.rs
? docs/my notes.md
u uu N... 000000 000000 000000 000000 000000 000000 000000 conflicted.txt
";
        let p = parse_porcelain_v2(out);
        assert_eq!(p.files.len(), 4);
        // porcelain v2 uses '.' for "unmodified" — worktree-only change is ".M".
        assert_eq!(p.files[0], (".M".to_string(), "src/main.rs".to_string()));
        assert_eq!(p.files[1], ("R.".to_string(), "new name.rs".to_string()), "orig path dropped");
        assert_eq!(p.files[2], ("??".to_string(), "docs/my notes.md".to_string()), "spaces in path kept");
        assert_eq!(p.files[3].0, "uu");
        assert_eq!(p.files[3].1, "conflicted.txt");
    }

    #[test]
    fn porcelain_v2_empty_is_clean() {
        let p = parse_porcelain_v2("");
        assert_eq!(p.counts.total(), 0);
        assert_eq!(p.current_branch, None);
        assert!(!p.detached);
    }

    #[test]
    fn track_variants() {
        assert_eq!(parse_track(""), (0, 0, false));
        assert_eq!(parse_track("[ahead 2]"), (2, 0, false));
        assert_eq!(parse_track("[behind 3]"), (0, 3, false));
        assert_eq!(parse_track("[ahead 1, behind 4]"), (1, 4, false));
        assert_eq!(parse_track("[gone]"), (0, 0, true));
        // Garbage must not panic; unknown tokens are ignored.
        assert_eq!(parse_track("[???]"), (0, 0, false));
    }

    #[test]
    fn for_each_ref_parses_fields() {
        let sep = FER_FIELD_SEP;
        let out = format!(
            "main{0}origin/main{0}[ahead 2]{0}*{0}origin\nfeature{0}{0}{0} {0}\ngone-branch{0}origin/deleted{0}[gone]{0} {0}origin\n",
            sep
        );
        let branches = parse_for_each_ref(&out);
        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].name, "main");
        assert_eq!(branches[0].upstream.as_deref(), Some("origin/main"));
        assert_eq!(branches[0].remote.as_deref(), Some("origin"));
        assert_eq!(branches[0].ahead, 2);
        assert!(branches[0].is_head);
        assert!(branches[1].upstream.is_none());
        assert!(branches[1].remote.is_none());
        assert_eq!(branches[1].ahead, 0);
        assert!(!branches[1].is_head);
        assert!(branches[2].gone);
        assert_eq!(branches[2].remote.as_deref(), Some("origin"));
    }

    #[test]
    fn for_each_ref_remote_name_with_slash() {
        let sep = FER_FIELD_SEP;
        // Remote named "team/fork": naive splitting of "team/fork/main"
        // would guess "team"; %(upstream:remotename) must be exact.
        let out = format!("main{0}team/fork/main{0}[ahead 1]{0}*{0}team/fork\n", sep);
        let b = &parse_for_each_ref(&out)[0];
        assert_eq!(b.remote.as_deref(), Some("team/fork"));
    }
}
