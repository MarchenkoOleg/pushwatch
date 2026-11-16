//! UI language: ru/en strings for the tray menu, notifications, history
//! entries and scan warnings. The frontend has its own dictionary in
//! `src/lib/i18n.svelte.ts` — keep the two in sync when adding strings.

use crate::model::Lang;

/// Static strings used as-is (tray menu, plain messages).
pub struct Strings {
    pub tray_open: &'static str,
    pub tray_scan: &'static str,
    pub tray_autostart: &'static str,
    pub tray_quit: &'static str,
    pub notify_all_pushed: &'static str,
    pub err_spawn_git: &'static str,
    pub err_path_missing: &'static str,
    pub err_push_busy: &'static str,
    pub err_push_all_busy: &'static str,
    pub err_nothing_to_push: &'static str,
    pub err_fetch_busy: &'static str,
}

const RU: Strings = Strings {
    tray_open: "Открыть pushwatch",
    tray_scan: "Сканировать сейчас",
    tray_autostart: "Запускать при входе",
    tray_quit: "Выход",
    notify_all_pushed: "Всё запушено ✓",
    err_spawn_git: "не удалось запустить git",
    err_path_missing: "путь не существует",
    err_push_busy: "push для этого репозитория уже выполняется",
    err_push_all_busy: "push all уже выполняется",
    err_nothing_to_push: "нечего пушить — все ветки отправлены",
    err_fetch_busy: "fetch для этого репозитория уже выполняется",
};

const EN: Strings = Strings {
    tray_open: "Open pushwatch",
    tray_scan: "Scan now",
    tray_autostart: "Launch at login",
    tray_quit: "Quit",
    notify_all_pushed: "All pushed ✓",
    err_spawn_git: "failed to launch git",
    err_path_missing: "path does not exist",
    err_push_busy: "push is already running for this repository",
    err_push_all_busy: "push all is already running",
    err_nothing_to_push: "nothing to push — all branches are pushed",
    err_fetch_busy: "fetch is already running for this repository",
};

/// Resolve the preference into a concrete language. `Auto` follows the OS
/// locale (`ru*` → Ru, everything else → En).
pub fn resolve(lang: Lang) -> Lang {
    match lang {
        Lang::Ru | Lang::En => lang,
        Lang::Auto => {
            let sys = sys_locale::get_locale().unwrap_or_default().to_lowercase();
            if sys.starts_with("ru") {
                Lang::Ru
            } else {
                Lang::En
            }
        }
    }
}

/// String table for the resolved language.
pub fn tr(lang: Lang) -> &'static Strings {
    match resolve(lang) {
        Lang::Ru => &RU,
        _ => &EN,
    }
}

/// Tray tooltip: `pushwatch: N problem repos out of M`.
pub fn tooltip(lang: Lang, problems: usize, total: usize) -> String {
    match resolve(lang) {
        Lang::Ru => format!("pushwatch: {problems} проблемных из {total}"),
        _ => format!("pushwatch: {problems} of {total} repos with problems"),
    }
}

/// History: scan summary line.
pub fn hist_scan(lang: Lang, repos: usize, problems: usize, unpushed: usize, elapsed: u64) -> String {
    match resolve(lang) {
        Lang::Ru => format!(
            "скан: {repos} репо, {problems} проблемных, с незапушенными: {unpushed} ({elapsed} с)"
        ),
        _ => format!(
            "scan: {repos} repos, {problems} with problems, {unpushed} with unpushed commits ({elapsed}s)"
        ),
    }
}

/// History: fetch-all summary line.
pub fn hist_fetch_all(lang: Lang, total: usize, failed: usize) -> String {
    match resolve(lang) {
        Lang::Ru => format!("fetch всё: {total} репо, ошибок: {failed}"),
        _ => format!("fetch all: {total} repos, {failed} failed"),
    }
}

/// History: batch push summary line.
pub fn hist_push_batch(lang: Lang, pushed: usize, total: usize, failed: usize) -> String {
    match resolve(lang) {
        Lang::Ru => format!("batch push: отправлено {pushed} из {total}, ошибок: {failed}"),
        _ => format!("batch push: {pushed} of {total} pushed, {failed} failed"),
    }
}

/// Notification: new unpushed commits appeared.
pub fn notify_unpushed(lang: Lang, count: usize, names: &str) -> String {
    match resolve(lang) {
        Lang::Ru => format!("Незапушенные коммиты: {count} репо ({names})"),
        _ => format!("Unpushed commits: {count} repos ({names})"),
    }
}

/// Scan warning: root folder does not exist.
pub fn warn_missing_root(lang: Lang, root: &str) -> String {
    match resolve(lang) {
        Lang::Ru => format!("Корневая папка не найдена: {root}"),
        _ => format!("Root folder not found: {root}"),
    }
}

/// Scan warning: some paths under a root were unreadable.
pub fn warn_unreadable(lang: Lang, root: &str, errors: usize) -> String {
    match resolve(lang) {
        Lang::Ru => format!("{root}: пропущено недоступных путей: {errors}"),
        _ => format!("{root}: skipped unreadable paths: {errors}"),
    }
}

/// Scan warning: manually added path is not a repository.
pub fn warn_not_repo(lang: Lang, extra: &str) -> String {
    match resolve(lang) {
        Lang::Ru => format!("Не репозиторий (или недоступен): {extra}"),
        _ => format!("Not a repository (or inaccessible): {extra}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_lang_is_respected() {
        assert_eq!(resolve(Lang::Ru), Lang::Ru);
        assert_eq!(resolve(Lang::En), Lang::En);
    }

    #[test]
    fn dictionaries_cover_both_languages() {
        // Every string differs between the two languages and is non-empty.
        let (ru, en) = (tr(Lang::Ru), tr(Lang::En));
        for (a, b) in [
            (ru.tray_open, en.tray_open),
            (ru.tray_scan, en.tray_scan),
            (ru.tray_autostart, en.tray_autostart),
            (ru.tray_quit, en.tray_quit),
            (ru.notify_all_pushed, en.notify_all_pushed),
            (ru.err_spawn_git, en.err_spawn_git),
            (ru.err_path_missing, en.err_path_missing),
            (ru.err_push_busy, en.err_push_busy),
            (ru.err_push_all_busy, en.err_push_all_busy),
            (ru.err_nothing_to_push, en.err_nothing_to_push),
            (ru.err_fetch_busy, en.err_fetch_busy),
        ] {
            assert!(!a.is_empty() && !b.is_empty() && a != b);
        }
    }

    #[test]
    fn formatted_messages_use_placeholders() {
        assert!(hist_scan(Lang::En, 10, 2, 1, 5).contains("10"));
        assert!(hist_scan(Lang::Ru, 10, 2, 1, 5).contains("10"));
        assert!(tooltip(Lang::En, 3, 50).contains("3"));
        assert!(notify_unpushed(Lang::Ru, 4, "a, b").contains("a, b"));
        assert!(warn_missing_root(Lang::En, "/x/y").contains("/x/y"));
        assert!(warn_unreadable(Lang::Ru, "/x/y", 7).contains("7"));
        assert!(warn_not_repo(Lang::En, "/z").contains("/z"));
        assert!(hist_fetch_all(Lang::En, 9, 1).contains("9"));
        assert!(hist_push_batch(Lang::Ru, 5, 8, 3).contains("8"));
    }
}
