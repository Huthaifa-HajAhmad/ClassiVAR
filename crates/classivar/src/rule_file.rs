//! Reading a rule file from disk, for `--rules` at start-up and for the Rules
//! button. Both paths check the file the same way before any window uses it.

use std::path::Path;

use classivar_core::RuleSet;

/// The largest rule file that can still be one: the built-in file is about 70 kB.
const MAX_RULE_BYTES: u64 = 1024 * 1024;

/// Read and check a rule file. The rules live until the program ends, so they
/// are leaked once: a window holds them by reference for its whole life.
pub fn read_rule_file(path: &Path) -> Result<&'static RuleSet, String> {
    let shown = path.display();
    // A rule file is small. Refuse a huge one before reading it into memory.
    let size =
        std::fs::metadata(path).map(|meta| meta.len()).map_err(|error| format!("could not read {shown}: {error}"))?;
    if size > MAX_RULE_BYTES {
        return Err(format!("{shown} is larger than {} kB and cannot be a rule file", MAX_RULE_BYTES / 1024));
    }
    let text = std::fs::read_to_string(path).map_err(|error| format!("could not read {shown}: {error}"))?;
    let rules = RuleSet::parse(&text).map_err(|error| format!("{shown} is not a valid rule file: {error}"))?;
    Ok(Box::leak(Box::new(rules)))
}

#[cfg(test)]
mod tests {
    use super::read_rule_file;

    fn scratch(name: &str, text: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("classivar-rule-file-test-{}-{name}", std::process::id()));
        std::fs::write(&path, text).expect("the temp folder is writable");
        path
    }

    #[test]
    fn a_valid_rule_file_is_read_and_a_broken_one_is_refused_by_name() {
        let builtin = include_str!("../../../rules/acmg-amp-points.toml");
        let good = scratch("good.toml", &builtin.replace("version = \"0.14.0-draft\"", "version = \"0.14.0-panel\""));
        assert_eq!(read_rule_file(&good).unwrap().meta.version, "0.14.0-panel");
        let bad = scratch("bad.toml", &builtin.replace("ba1_frequency = 0.05", "ba1_frequency = 7.0"));
        let error = read_rule_file(&bad).unwrap_err();
        assert!(error.contains("bad.toml") && error.contains("not a valid rule file"), "{error}");
        let missing = std::env::temp_dir().join("classivar-no-such-rule-file.toml");
        assert!(read_rule_file(&missing).unwrap_err().contains("could not read"));
        for path in [good, bad] {
            std::fs::remove_file(path).ok();
        }
    }
}
