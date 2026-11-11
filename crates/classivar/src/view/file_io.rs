//! File names and writing a case file safely.

use super::*;

/// A file name for the save dialog, made from the variant name. Anything that is
/// not a letter, digit, dash or dot becomes an underscore, so the name is safe on every system.
pub(super) fn suggested_file_name(variant: &str) -> String {
    let stem: String = variant
        .trim()
        .chars()
        .take(40)
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect();
    let stem = if stem.is_empty() { "case".to_string() } else { stem };
    // Windows refuses CON.classivar.json even with an extension, so a reserved
    // device name gets a prefix rather than a raw OS error at save time.
    let reserved = |name: &str| {
        let base = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && base.as_bytes()[3].is_ascii_digit()
                && base.as_bytes()[3] != b'0')
    };
    let stem = if reserved(&stem) { format!("_{stem}") } else { stem };
    format!("{stem}.classivar.json")
}

pub(super) fn write_case(path: &PathBuf, json: &str) -> Result<(), String> {
    // A temp file next to the target first, then a rename over it: a failed
    // write or a crash leaves the old case intact instead of truncating it.
    let temp = path.with_file_name(match path.file_name() {
        Some(name) => format!("{}.tmp", name.to_string_lossy()),
        None => "case.tmp".to_string(),
    });
    let remove = |p: &PathBuf| {
        let _ = std::fs::remove_file(p);
    };
    if let Err(error) = std::fs::write(&temp, json) {
        remove(&temp);
        return Err(error.to_string());
    }
    if let Err(error) = std::fs::rename(&temp, path) {
        remove(&temp);
        return Err(format!("the file was written, but the old copy could not be replaced: {error}"));
    }
    Ok(())
}

/// Where the save dialog starts: the user's Documents folder. Saving into the
/// program's working directory (often the repository) is how case files end up
/// committed by accident.
pub(super) fn default_save_folder() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from);
    if let Some(home) = home {
        let documents = home.join("Documents");
        return if documents.is_dir() { documents } else { home };
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::suggested_file_name;

    #[test]
    fn file_names_are_safe() {
        assert_eq!(suggested_file_name("NM_007294.4:c.5266dup"), "NM_007294.4_c.5266dup.classivar.json");
        assert_eq!(suggested_file_name("  "), "case.classivar.json");
        assert_eq!(suggested_file_name("a/b\\c:d*e"), "a_b_c_d_e.classivar.json");
        assert!(suggested_file_name(&"x".repeat(200)).len() < 60);
        // Windows device names cannot be used as file names, even with an extension.
        assert_eq!(suggested_file_name("CON"), "_CON.classivar.json");
        assert_eq!(suggested_file_name("com1"), "_com1.classivar.json");
        assert_eq!(suggested_file_name("COM10"), "COM10.classivar.json");
    }
}
