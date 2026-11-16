//! Config persistence and path helpers.

use std::path::{Path, PathBuf};

use crate::model::Config;

/// Expand a leading `~` into the user's home directory.
pub fn expand_tilde(p: &str) -> String {
    let p = p.trim();
    if p == "~" {
        return home().to_string_lossy().into_owned();
    }
    if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
        return home().join(rest).to_string_lossy().into_owned();
    }
    p.to_string()
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Config {
    /// Load from disk; a missing file yields defaults. A corrupt/partial
    /// file also yields defaults (so the app still boots) but is preserved
    /// as `config.json.bak-<timestamp>` instead of being silently
    /// overwritten — roots/settings stay recoverable, and the old
    /// machine_id remains inspectable for the future aggregation contract.
    pub fn load(path: &Path) -> Config {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => config,
                Err(_) => {
                    Self::quarantine_broken(path, &content);
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    fn quarantine_broken(path: &Path, content: &str) {
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let bak = path.with_file_name(format!(
            "{}.bak-{stamp}",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "config".into())
        ));
        let _ = std::fs::write(&bak, content);
    }

    /// Atomic save: write to a temp file in the same directory, then rename
    /// over the target. A crash mid-write can never leave a partial config.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_file_name(format!(
            "{}.tmp-{}",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "config".into()),
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_expansion() {
        let home = home();
        assert_eq!(expand_tilde("~"), home.to_string_lossy());
        assert_eq!(
            expand_tilde("~/dev/repos"),
            home.join("dev/repos").to_string_lossy()
        );
        assert_eq!(expand_tilde("/abs/path"), "/abs/path");
        assert_eq!(expand_tilde(" ~/x "), home.join("x").to_string_lossy());
        assert_eq!(expand_tilde("weird~path"), "weird~path");
    }

    #[test]
    fn load_missing_file_gives_defaults() {
        let cfg = Config::load(Path::new("/nonexistent/pushwatch/config.json"));
        assert!(!cfg.machine_id.is_empty());
        assert_eq!(cfg.scan_interval_secs, 300);
        assert!(cfg.exclude_dirs.contains(&"node_modules".to_string()));
    }

    #[test]
    fn save_load_roundtrip_all_fields() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let path = dir.join("config.json");
        let mut cfg = Config::default();
        cfg.roots = vec!["/tmp/dev".into()];
        cfg.exclude_dirs = vec!["vendor".into(), "target".into()];
        cfg.extra_repos = vec!["/opt/proj".into()];
        cfg.hidden_repos = vec!["/tmp/dev/hidden-repo".into()];
        cfg.scan_interval_secs = 123456;
        cfg.max_depth = 9;
        cfg.start_hidden = true;
        cfg.theme = crate::model::Theme::Light;
        cfg.save(&path).unwrap();
        let loaded = Config::load(&path);
        assert_eq!(loaded.roots, vec!["/tmp/dev".to_string()]);
        assert_eq!(loaded.exclude_dirs, vec!["vendor".to_string(), "target".to_string()]);
        assert_eq!(loaded.extra_repos, vec!["/opt/proj".to_string()]);
        assert_eq!(loaded.hidden_repos, vec!["/tmp/dev/hidden-repo".to_string()]);
        assert_eq!(loaded.scan_interval_secs, 123456);
        assert_eq!(loaded.max_depth, 9);
        assert!(loaded.start_hidden);
        assert_eq!(loaded.theme, crate::model::Theme::Light);
        assert_eq!(loaded.machine_id, cfg.machine_id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_config_is_quarantined_not_destroyed() {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, "{ roots: [ broken").unwrap();

        let cfg = Config::load(&path);
        assert!(!cfg.machine_id.is_empty(), "defaults still boot the app");

        // The broken file is preserved, not silently overwritten.
        let baks: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .collect();
        assert_eq!(baks.len(), 1, "one .bak file expected");
        std::fs::remove_dir_all(&dir).ok();
    }
}
