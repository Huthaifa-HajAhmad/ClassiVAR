//! Debug-only live reload for theme JSON and `--rules` files.
//! Compiled out of release builds completely so shipped binaries stay sealed.

use std::path::{Path, PathBuf};
use std::time::Duration;

use classivar_core::RuleSet;
use gpui_kit::component::{ActiveTheme, ThemeSet};
use gpui_kit::{AppContext, Context, Task, Window};

use crate::status::Status;
use crate::view::Workbench;

const POLL_INTERVAL: Duration = Duration::from_millis(350);

/// Paths to watch for live reload while developing.
#[derive(Clone, Default)]
pub struct DevWatch {
    pub theme_path: Option<PathBuf>,
    pub rules_path: Option<PathBuf>,
}

/// The default theme file in the checkout, when running from the source tree.
pub fn default_theme_path() -> Option<PathBuf> {
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/classivar-theme.json"));
    path.is_file().then_some(path)
}

/// A file watched by comparing its text with the text seen last. The files are
/// small, so reading one on every poll costs little, and an edit that keeps the
/// size and lands in the same clock tick as the last save (a colour change such
/// as `#eeede9` to `#f4f1ea`) is not missed.
#[derive(Default)]
pub struct WatchedFile {
    seen: String,
    /// True once an unreadable file was reported, so each failure is reported once.
    reported_unreadable: bool,
}

impl WatchedFile {
    /// Start from the text the file holds now. A file that cannot be read yet counts as unseen.
    pub fn read(path: &Path) -> Self {
        Self { seen: std::fs::read_to_string(path).unwrap_or_default(), reported_unreadable: false }
    }

    /// The new text when the file changed, or an error the first time it cannot be read.
    /// A missing or empty file is an editor in the middle of a save, so it is skipped.
    pub fn poll(&mut self, path: &Path) -> Option<Result<String, String>> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                self.reported_unreadable = false;
                if text.is_empty() || text == self.seen {
                    return None;
                }
                self.seen.clone_from(&text);
                Some(Ok(text))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) if self.reported_unreadable => None,
            Err(error) => {
                self.reported_unreadable = true;
                Some(Err(format!("could not read {}: {error}", path.display())))
            }
        }
    }
}

type ThemeUpdate = (PathBuf, Result<(ThemeSet, Vec<String>), String>);

struct PollState {
    theme_path: Option<PathBuf>,
    theme_file: WatchedFile,
    rules_path: Option<PathBuf>,
    rules_file: WatchedFile,
}

impl PollState {
    fn from_watch(watch: DevWatch) -> Self {
        let theme_file = watch.theme_path.as_deref().map(WatchedFile::read).unwrap_or_default();
        let rules_file = watch.rules_path.as_deref().map(WatchedFile::read).unwrap_or_default();
        Self { theme_path: watch.theme_path, theme_file, rules_path: watch.rules_path, rules_file }
    }

    fn poll(mut self) -> (Self, Option<ThemeUpdate>, Option<Result<&'static RuleSet, String>>) {
        let theme = self.theme_path.as_deref().and_then(|path| {
            let parsed = self.theme_file.poll(path)?.and_then(|text| {
                crate::theme::parse_themes(&text).map(|set| (set, crate::theme::contrast_warnings(&text)))
            });
            Some((path.to_path_buf(), parsed))
        });
        let rules = self.rules_path.as_deref().and_then(|path| {
            let read = self.rules_file.poll(path)?;
            Some(read.and_then(|_| crate::rule_file::read_rule_file(path)))
        });
        (self, theme, rules)
    }
}

fn theme_status(path: &Path, warnings: &[String]) -> Status {
    if warnings.is_empty() {
        Status::done(format!("Reloaded theme from {}.", path.display()))
    } else {
        Status::attention(format!("Reloaded theme from {}, contrast warning: {}", path.display(), warnings.join("; ")))
    }
}

/// Watch `watch` paths off the UI thread and apply valid edits live.
pub fn start_watcher(watch: DevWatch, window: &mut Window, cx: &mut Context<Workbench>) -> Option<Task<()>> {
    if watch.theme_path.is_none() && watch.rules_path.is_none() {
        return None;
    }
    let mut state = Some(PollState::from_watch(watch));
    Some(cx.spawn_in(window, async move |this, cx| {
        loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            let current = state.take().expect("poll state is present");
            let (next, theme, rules) = cx.background_spawn(async move { current.poll() }).await;
            state = Some(next);
            if let Some((path, parsed)) = theme {
                let alive = this.update(cx, |this, cx| apply_theme_poll(this, &path, parsed, cx)).is_ok();
                if !alive {
                    break;
                }
            }
            if let Some(loaded) = rules {
                let alive = this.update_in(cx, |this, window, cx| apply_rules_poll(this, loaded, window, cx)).is_ok();
                if !alive {
                    break;
                }
            }
        }
    }))
}

pub(crate) fn apply_theme_poll(
    this: &mut Workbench,
    path: &Path,
    parsed: Result<(ThemeSet, Vec<String>), String>,
    cx: &mut Context<Workbench>,
) {
    match parsed {
        Ok((set, warnings)) => {
            let mode = cx.theme().mode;
            crate::theme::apply_theme_set(set, mode, cx);
            this.set_dev_status(theme_status(path, &warnings), cx);
        }
        Err(error) => this.set_dev_status(Status::attention(format!("Theme reload skipped: {error}")), cx),
    }
}

pub(crate) fn apply_rules_poll(
    this: &mut Workbench,
    loaded: Result<&'static RuleSet, String>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) {
    match loaded {
        Ok(rules) => this.reload_rules(rules, window, cx),
        Err(error) => this.set_dev_status(Status::attention(format!("Rules reload skipped: {error}")), cx),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{DevWatch, PollState, WatchedFile};

    const THEME_JSON: &str = include_str!("../assets/classivar-theme.json");
    const RULES_TOML: &str = include_str!("../../../rules/acmg-amp-points.toml");

    /// A scratch file in the temp folder. It is removed when the test ends, also when it fails.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let scratch =
                Self(std::env::temp_dir().join(format!("classivar-dev-reload-{}-{name}", std::process::id())));
            scratch.write(bytes);
            scratch
        }

        fn write(&self, bytes: &[u8]) {
            std::fs::write(&self.0, bytes).expect("the temp folder is writable");
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_file(&self.0).ok();
        }
    }

    fn edited(text: &str, from: &str, to: &str) -> String {
        let out = text.replace(from, to);
        assert_ne!(out, text, "the fixture holds {from:?}");
        out
    }

    #[test]
    fn a_watched_file_reports_real_edits_only() {
        let file = Scratch::new("edits.txt", b"alpha");
        let mut watched = WatchedFile::read(&file.0);
        assert_eq!(watched.poll(&file.0), None, "an unchanged file yields nothing");

        file.write(b"alpha");
        assert_eq!(watched.poll(&file.0), None, "a rewrite with the same text yields nothing");

        file.write(b"");
        assert_eq!(watched.poll(&file.0), None, "an empty file is a save in progress");

        // Same length as the old text: a size-and-time check could miss this.
        file.write(b"omega");
        assert_eq!(watched.poll(&file.0), Some(Ok("omega".to_string())));
        assert_eq!(watched.poll(&file.0), None, "the edit is reported once");
    }

    #[test]
    fn an_unreadable_file_is_reported_once_and_reloads_after_it_is_fixed() {
        let file = Scratch::new("unreadable.txt", b"alpha");
        let mut watched = WatchedFile::read(&file.0);

        file.write(&[0xff, 0xfe, 0x00]);
        let report = watched.poll(&file.0).expect("an unreadable file is reported").unwrap_err();
        assert!(report.contains("could not read") && report.contains("unreadable.txt"), "{report}");
        assert_eq!(watched.poll(&file.0), None, "the same failure is not reported again");

        file.write(b"fixed");
        assert_eq!(watched.poll(&file.0), Some(Ok("fixed".to_string())), "a fixed file reloads");

        std::fs::remove_file(&file.0).expect("the scratch file exists");
        assert_eq!(watched.poll(&file.0), None, "a missing file is a save in progress");
    }

    #[test]
    fn a_poll_returns_a_good_theme_edit_with_its_warnings_and_reports_a_bad_one() {
        let file = Scratch::new("theme.json", THEME_JSON.as_bytes());
        let watch = DevWatch { theme_path: Some(file.0.clone()), rules_path: None };
        let (state, theme, rules) = PollState::from_watch(watch).poll();
        assert!(theme.is_none() && rules.is_none(), "nothing changed yet");

        file.write(edited(THEME_JSON, "#eeede9", "#f4f1ea").as_bytes());
        let (state, theme, _) = state.poll();
        let Some((path, Ok((_set, warnings)))) = theme else { panic!("a valid edit is returned") };
        assert_eq!(path, file.0);
        assert!(warnings.is_empty(), "{warnings:?}");

        file.write(edited(THEME_JSON, "\"foreground\": \"#141414\"", "\"foreground\": \"#dcdad3\"").as_bytes());
        let (state, theme, _) = state.poll();
        let Some((_, Ok((_set, warnings)))) = theme else { panic!("a low contrast edit still parses") };
        assert!(warnings.iter().any(|w| w.contains("foreground on background")), "{warnings:?}");

        file.write(b"{ not json");
        let (_, theme, _) = state.poll();
        let Some((_, Err(error))) = theme else { panic!("invalid JSON is an error") };
        assert!(error.contains("invalid theme JSON"), "{error}");
    }

    #[test]
    fn a_poll_returns_good_rules_and_reports_a_broken_rule_file() {
        let file = Scratch::new("rules.toml", RULES_TOML.as_bytes());
        let watch = DevWatch { theme_path: None, rules_path: Some(file.0.clone()) };
        let (state, theme, rules) = PollState::from_watch(watch).poll();
        assert!(theme.is_none() && rules.is_none(), "nothing changed yet");

        file.write(edited(RULES_TOML, "version = \"0.14.0-draft\"", "version = \"0.14.0-poll\"").as_bytes());
        let (state, _, rules) = state.poll();
        let Some(Ok(rules)) = rules else { panic!("valid rules are returned") };
        assert_eq!(rules.meta.version, "0.14.0-poll");

        file.write(b"this is not = [valid toml");
        let (_, _, rules) = state.poll();
        let Some(Err(error)) = rules else { panic!("a broken rule file is an error") };
        assert!(error.contains("not a valid rule file"), "{error}");
    }
}
