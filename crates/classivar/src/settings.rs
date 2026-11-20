//! The preferences worth keeping between runs: the theme, and which picture
//! explains the class (a dial, a column or a track).
//!
//! The file is deliberately tiny (`key=value` lines) and failure never stops the
//! app: if it cannot be read or written, ClassiVAR starts with the defaults.

use std::path::PathBuf;

use gpui_kit::component::ThemeMode;

/// The picture that explains the class. All three draw the same numbers from
/// the engine; they differ only in shape, so the curator can pick the one that
/// reads best for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Picture {
    /// A half circle of class bands with the evidence on an inner ring and a needle at the total.
    #[default]
    Dial,
    /// A tall column of class bands with the evidence stacked inside it, next to earlier reports.
    Column,
    /// A wide band of class zones with the evidence laid end to end.
    Track,
}

impl Picture {
    pub const ALL: [Picture; 3] = [Picture::Dial, Picture::Column, Picture::Track];

    /// The word saved in the settings file and shown on the switch.
    pub fn name(self) -> &'static str {
        match self {
            Picture::Dial => "Dial",
            Picture::Column => "Column",
            Picture::Track => "Track",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|picture| picture.name().eq_ignore_ascii_case(value))
    }
}

/// Where the preferences live: the user's configuration folder, not next to the
/// program, so an install in a read-only folder still remembers the choice.
fn settings_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("ClassiVAR").join("settings.txt"))
}

fn read_file() -> String {
    settings_file().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default()
}

/// The value of `key` in the settings text, if there is one.
fn value_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().filter_map(|line| line.split_once('=')).find(|(k, _)| k.trim() == key).map(|(_, v)| v.trim())
}

/// The settings text with `key` set to `value`. Other lines are kept as they were.
fn with_value(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| line.split_once('=').is_none_or(|(k, _)| k.trim() != key))
        .map(String::from)
        .collect();
    lines.push(format!("{key}={value}"));
    lines.join("\n") + "\n"
}

/// Remember one preference. A failure is ignored on purpose: losing a preference
/// is not worth an error message in the middle of a case.
fn save(key: &str, value: &str) {
    let Some(path) = settings_file() else { return };
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    let _ = std::fs::write(&path, with_value(&read_file(), key, value));
}

/// The theme last chosen, or the light theme if there is no readable choice.
pub fn load_theme() -> ThemeMode {
    parse_theme(&read_file()).unwrap_or(ThemeMode::Light)
}

pub fn save_theme(mode: ThemeMode) {
    save("theme", mode.name());
}

/// The picture last chosen, or the dial if there is no readable choice.
pub fn load_picture() -> Picture {
    value_of(&read_file(), "picture").and_then(Picture::parse).unwrap_or_default()
}

pub fn save_picture(picture: Picture) {
    save("picture", picture.name());
}

/// Read the theme out of the settings text. A file with no usable line means "no choice".
fn parse_theme(text: &str) -> Option<ThemeMode> {
    match value_of(text, "theme")?.to_ascii_lowercase().as_str() {
        "dark" => Some(ThemeMode::Dark),
        "light" => Some(ThemeMode::Light),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Picture, parse_theme, value_of, with_value};
    use gpui_kit::component::ThemeMode;

    #[test]
    fn the_saved_value_is_read_back() {
        assert_eq!(parse_theme("theme=dark\n"), Some(ThemeMode::Dark));
        assert_eq!(parse_theme("theme = light"), Some(ThemeMode::Light));
        assert_eq!(parse_theme("other=1\ntheme=DARK\n"), Some(ThemeMode::Dark));
    }

    #[test]
    fn a_missing_or_broken_file_means_no_choice() {
        assert_eq!(parse_theme(""), None);
        assert_eq!(parse_theme("theme="), None);
        assert_eq!(parse_theme("theme=sepia"), None);
        assert_eq!(parse_theme("nothing here"), None);
    }

    #[test]
    fn saving_one_preference_keeps_the_other() {
        let text = with_value("theme=dark\n", "picture", "Column");
        assert_eq!(value_of(&text, "theme"), Some("dark"));
        assert_eq!(value_of(&text, "picture"), Some("Column"));
        let text = with_value(&text, "theme", "light");
        assert_eq!(value_of(&text, "theme"), Some("light"));
        assert_eq!(value_of(&text, "picture"), Some("Column"), "changing the theme keeps the picture");
        assert_eq!(text.matches("theme=").count(), 1, "a key is written once");
    }

    #[test]
    fn a_picture_is_read_back_by_name_and_an_unknown_one_is_no_choice() {
        for picture in Picture::ALL {
            assert_eq!(Picture::parse(picture.name()), Some(picture));
        }
        assert_eq!(Picture::parse("track"), Some(Picture::Track));
        assert_eq!(Picture::parse("pie"), None);
    }
}
