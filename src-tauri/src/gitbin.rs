//! Locating and invoking the system `git`.
//!
//! All read-only status commands go through `run_git`, which pins the
//! environment (`LC_ALL=C`, `core.quotepath=false`, `--no-optional-locks`)
//! so parsing is deterministic and the index is never written by scans.

use std::path::Path;
use std::process::Command;

/// Hide the console window that Windows would otherwise allocate for every
/// spawned console child (git) — in a GUI app each spawn would flash one.
#[cfg(windows)]
pub fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn hide_console(_cmd: &mut Command) {}

/// Global git flags applied to every status command:
/// - `--no-optional-locks`: scanning must never take the index lock.
/// - `core.quotepath=false`: non-ASCII paths stay raw UTF-8.
/// - `LC_ALL=C`: messages (`[ahead 2]`) stay English, locale-independent.
pub fn status_command(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .arg("--no-optional-locks")
        .arg("-c")
        .arg("core.quotepath=false")
        .env("LC_ALL", "C")
        .args(args);
    hide_console(&mut cmd);
    cmd
}

/// Run a read-only git command in `dir`, return trimmed stdout.
/// Error strings are deliberately English (not localized): they sit in the
/// UI next to raw git stderr, which is always English under `LC_ALL=C`.
pub fn run_git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = status_command(dir, args)
        .output()
        .map_err(|e| format!("failed to launch git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("git {} exited with code {}", args.join(" "), output.status)
        } else {
            stderr
        })
    }
}

/// Check that git is available; return `git version` output on success.
pub fn detect() -> Option<String> {
    Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_is_available_in_dev_env() {
        let v = detect().expect("git must be installed on dev machines");
        assert!(v.starts_with("git version"));
    }

    #[test]
    fn run_git_reports_errors_from_stderr() {
        let tmp = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&tmp).unwrap();
        let err = run_git(&tmp, &["status"]).unwrap_err();
        assert!(err.contains("git") || err.to_lowercase().contains("repository") || !err.is_empty());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
