//! The one preference worth keeping between runs: the theme.
//!
//! The file is deliberately tiny and failure never stops the app: if it cannot
//! be read or written, ClassiVAR simply starts in the light theme.

use std::path::PathBuf;

use gpui_kit::component::ThemeMode;

/// Where the preference lives: the user's configuration folder, not next to the
/// program, so an install in a read-only folder still remembers the choice.
fn settings_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("ClassiVAR").join("settings.txt"))
}

/// The theme last chosen, or the light theme if there is no readable choice.
pub fn load_theme() -> ThemeMode {
    let Some(path) = settings_file() else { return ThemeMode::Light };
    let Ok(text) = std::fs::read_to_string(path) else { return ThemeMode::Light };
    parse_theme(&text).unwrap_or(ThemeMode::Light)
}

/// Remember the theme. A failure is ignored on purpose: losing a preference is
/// not worth an error message in the middle of a case.
pub fn save_theme(mode: ThemeMode) {
    let Some(path) = settings_file() else { return };
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    let _ = std::fs::write(path, format!("theme={}\n", mode.name()));
}

/// Read the theme out of the settings file. Unknown lines are ignored; a file
/// with no usable line means "no choice".
fn parse_theme(text: &str) -> Option<ThemeMode> {
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        if key.trim() != "theme" {
            continue;
        }
        return match value.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(ThemeMode::Dark),
            "light" => Some(ThemeMode::Light),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_theme;
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
}
