//! Repository discovery: recursive scan of the configured roots plus the
//! manually added repos. Prunes excluded/hidden directories, detects
//! worktrees, skips submodules (their `.git` file points into
//! `<parent>/.git/modules/...`).

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::config::expand_tilde;
use crate::model::Config;

/// Normalize a path for storage/display: canonicalize when possible and
/// strip the Windows verbatim prefix (`\\?\C:\...`) that canonicalize adds.
pub fn normalize(path: &Path) -> PathBuf {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    #[cfg(windows)]
    {
        let s = canonical.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    canonical
}

/// Is `dir` a usable git repository (`.git` dir or file, not a submodule)?
fn is_repo(dir: &Path) -> bool {
    let dot_git = dir.join(".git");
    let Ok(meta) = std::fs::symlink_metadata(&dot_git) else {
        return false;
    };
    if meta.is_dir() {
        return true;
    }
    if !meta.is_file() {
        return false;
    }
    // `.git` file: worktree (usable) or submodule (skipped as noise).
    match std::fs::read_to_string(&dot_git) {
        Ok(content) => {
            let gitdir = content
                .lines()
                .find_map(|l| l.strip_prefix("gitdir:"))
                .map(|s| s.trim())
                .unwrap_or("");
            // Worktree gitdirs live in `.git/worktrees/`, submodules in `.git/modules/`.
            !gitdir.contains("/.git/modules/") && !gitdir.contains("\\.git\\modules\\")
        }
        Err(_) => false,
    }
}

/// Discover all repositories for the given config: roots + extras − hidden.
/// Returns the repos plus non-fatal warnings (missing roots, unreadable
/// directories) so the UI can explain "why don't I see my repo".
pub fn discover(config: &Config) -> (Vec<PathBuf>, Vec<String>) {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    fn add(path: PathBuf, found: &mut Vec<PathBuf>, seen: &mut Vec<PathBuf>) {
        let norm = normalize(&path);
        if !seen.iter().any(|p| p == &norm) {
            seen.push(norm.clone());
            found.push(norm);
        }
    }

    for root in &config.roots {
        let root = expand_tilde(root);
        let root_path = PathBuf::from(&root);
        if !root_path.is_dir() {
            warnings.push(format!("Корневая папка не найдена: {root}"));
            continue;
        }
        let (repos, errors) = scan_root(&root_path, config);
        if errors > 0 {
            warnings.push(format!(
                "{root}: пропущено недоступных путей: {errors}"
            ));
        }
        for repo in repos {
            add(repo, &mut found, &mut seen);
        }
    }
    for extra in &config.extra_repos {
        let path = PathBuf::from(expand_tilde(extra));
        if is_repo(&path) {
            add(path, &mut found, &mut seen);
        } else {
            warnings.push(format!("Не репозиторий (или недоступен): {extra}"));
        }
    }
    // Hidden entries are compared normalized: the user may enter an
    // uncanonicalized path (/tmp/... on macOS), while discovered repos
    // store canonical ones (/private/tmp/...).
    let hidden: Vec<String> = config
        .hidden_repos
        .iter()
        .map(|h| normalize(Path::new(&expand_tilde(h))).to_string_lossy().into_owned())
        .collect();
    found.retain(|p| !hidden.contains(&p.to_string_lossy().into_owned()));
    found.sort();
    (found, warnings)
}

fn scan_root(root: &Path, config: &Config) -> (Vec<PathBuf>, usize) {
    let excludes: Vec<String> = config.exclude_dirs.iter().cloned().collect();
    let mut repos = Vec::new();
    let mut errors = 0usize;
    // Manual iteration so excluded/hidden dirs are truly pruned
    // (skip_current_dir stops the descent — a plain `continue` would not).
    let mut it = WalkDir::new(root)
        .max_depth(config.max_depth.max(1) as usize)
        .follow_links(false)
        .into_iter();
    loop {
        let entry = match it.next() {
            None => break,
            Some(Ok(e)) => e,
            Some(Err(_)) => {
                errors += 1;
                continue;
            }
        };
        if entry.depth() > 0 && entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') || excludes.iter().any(|e| e == &name) {
                it.skip_current_dir();
                continue;
            }
            if is_repo(entry.path()) {
                repos.push(entry.path().to_path_buf());
                // Record the repo but keep descending: nested repos are wanted.
            }
        }
    }
    (repos, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Config;
    use std::process::Command;

    fn uniq_root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("pushwatch-scan-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("LC_ALL", "C")
            .output()
            .expect("git must be available");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_repo(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        git(dir, &["init", "--quiet"]);
    }

    fn paths(v: &[PathBuf]) -> Vec<String> {
        v.iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    fn config_for(root: &Path) -> Config {
        let mut config = Config::default();
        config.roots = vec![root.to_string_lossy().into_owned()];
        config.max_depth = 4;
        config
    }

    #[test]
    fn finds_repos_and_prunes_excluded_and_hidden() {
        let root = uniq_root("tree");
        init_repo(&root.join("alpha"));
        init_repo(&root.join("nested").join("beta"));
        init_repo(&root.join("node_modules").join("should-skip"));
        init_repo(&root.join(".hiddendir").join("also-skip"));

        let (found, _) = discover(&config_for(&root));
        let names = paths(&found);
        assert!(names.contains(&"alpha".to_string()), "{names:?}");
        assert!(names.contains(&"beta".to_string()), "{names:?}");
        assert!(!names.contains(&"should-skip".to_string()), "{names:?}");
        assert!(!names.contains(&"also-skip".to_string()), "{names:?}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn nested_repo_inside_repo_is_found() {
        let root = uniq_root("nest");
        init_repo(&root.join("outer"));
        init_repo(&root.join("outer").join("inner"));
        let (found, _) = discover(&config_for(&root));
        let names = paths(&found);
        assert!(names.contains(&"outer".to_string()));
        assert!(names.contains(&"inner".to_string()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn hidden_repos_are_filtered_even_with_uncanonical_path() {
        let root = uniq_root("hide");
        init_repo(&root.join("visible"));
        init_repo(&root.join("secret"));
        let mut config = config_for(&root);
        // Store the hidden entry via the *uncanonicalized* temp path: the
        // scanner must match it against its canonicalized results.
        config.hidden_repos = vec![root.join("secret").to_string_lossy().into_owned()];

        let (found, _) = discover(&config);
        let names = paths(&found);
        assert!(names.contains(&"visible".to_string()));
        assert!(!names.contains(&"secret".to_string()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn worktree_dot_git_file_is_a_repo() {
        let root = uniq_root("wt");
        init_repo(&root.join("main"));
        git(&root.join("main"), &["commit", "--allow-empty", "-q", "-m", "init", "--quiet"]);
        git(&root.join("main"), &["branch", "--quiet", "dev"]);
        git(
            &root.join("main"),
            &[
                "worktree",
                "add",
                "--quiet",
                &root.join("linked").to_string_lossy(),
                "dev",
            ],
        );

        let (found, _) = discover(&config_for(&root));
        let names = paths(&found);
        assert!(
            names.contains(&"linked".to_string()),
            "linked worktree must be discovered: {names:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn submodule_style_git_file_is_skipped() {
        let root = uniq_root("sub");
        // Simulate a submodule layout without cloning one: `.git` file
        // pointing into a parent's `.git/modules/` is submodule noise.
        let sub = root.join("vendor-lib");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join(".git"),
            "gitdir: ../.git/modules/vendor-lib\n",
        )
        .unwrap();

        let (found, _) = discover(&config_for(&root));
        assert!(
            found.is_empty(),
            "submodule-style .git file must not count as a repo: {found:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_root_produces_warning() {
        let root = uniq_root("warn");
        init_repo(&root.join("real"));
        let mut config = config_for(&root);
        config.roots.push("/definitely/not/there".to_string());

        let (found, warnings) = discover(&config);
        assert_eq!(paths(&found), vec!["real".to_string()]);
        assert!(warnings.iter().any(|w| w.contains("/definitely/not/there")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn extra_repo_outside_roots_is_added() {
        let root = uniq_root("hide2");
        init_repo(&root.join("visible"));
        let mut config = config_for(&root);
        let secret = normalize(&root.join("secret")).to_string_lossy().into_owned();
        config.hidden_repos = vec![secret];

        let (found, _) = discover(&config);
        let names = paths(&found);
        assert!(names.contains(&"visible".to_string()));
        assert!(!names.contains(&"secret".to_string()));

        let outside = uniq_root("outside");
        init_repo(&outside.join("manual"));
        config.extra_repos = vec![outside.join("manual").to_string_lossy().into_owned()];
        let (found, _) = discover(&config);
        let names = paths(&found);
        assert!(names.contains(&"manual".to_string()));
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&outside).ok();
    }
}
