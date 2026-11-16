//! Activity history: append-only JSONL log of scans and pushes with size
//! rotation. The UI reads the tail via `get_history`.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use chrono::Utc;

use crate::model::HistoryEntry;

/// Rotate when the file exceeds this size.
const ROTATE_BYTES: u64 = 512 * 1024;
/// Lines kept after rotation.
const KEEP_LINES: usize = 1000;

/// Append one entry; rotates the file when it grows past ROTATE_BYTES.
/// Never panics — history failures must not affect scanning/pushing.
pub fn append(path: &Path, kind: &str, message: &str, ok: Option<bool>) {
    let entry = HistoryEntry {
        ts: Utc::now(),
        kind: kind.to_string(),
        message: message.to_string(),
        ok,
    };
    let Ok(json) = serde_json::to_string(&entry) else {
        return;
    };
    if path.exists() {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() > ROTATE_BYTES {
                rotate(path);
            }
        }
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{json}");
    }
}

/// Keep only the last KEEP_LINES lines.
fn rotate(path: &Path) {
    let Ok(file) = std::fs::File::open(path) else {
        return;
    };
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .collect();
    let tail_start = lines.len().saturating_sub(KEEP_LINES);
    let tmp = path.with_extension("jsonl.tmp");
    if let Ok(mut out) = std::fs::File::create(&tmp) {
        for line in &lines[tail_start..] {
            let _ = writeln!(out, "{line}");
        }
        let _ = std::fs::rename(&tmp, path);
    }
}

/// Read the newest `limit` entries (file order is oldest-first).
pub fn read(path: &Path, limit: usize) -> Vec<HistoryEntry> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut entries: Vec<HistoryEntry> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect();
    entries.reverse();
    entries.truncate(limit);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pushwatch-hist-{tag}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn append_read_roundtrip_newest_first() {
        let path = tmp_path("rt");
        append(&path, "push", "repo-a: main", Some(true));
        append(&path, "scan", "12 repos, 1 problem", None);
        append(&path, "push", "repo-b: feature", Some(false));
        let entries = read(&path, 10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "repo-b: feature", "newest first");
        assert_eq!(entries[0].kind, "push");
        assert_eq!(entries[0].ok, Some(false));
        assert_eq!(entries[2].message, "repo-a: main");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn read_missing_file_is_empty() {
        let path = tmp_path("missing");
        assert!(read(&path, 10).is_empty());
    }

    #[test]
    fn corrupt_lines_are_skipped() {
        let path = tmp_path("corrupt");
        std::fs::write(&path, "garbage\n{\"ts\":\"2026-01-01T00:00:00Z\",\"kind\":\"scan\",\"message\":\"m\",\"ok\":null}\n").unwrap();
        let entries = read(&path, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "m");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn limit_truncates_to_newest() {
        let path = tmp_path("limit");
        for i in 0..10 {
            append(&path, "scan", &format!("entry-{i}"), None);
        }
        let entries = read(&path, 3);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "entry-9");
        assert_eq!(entries[2].message, "entry-7");
        std::fs::remove_file(&path).ok();
    }
}
