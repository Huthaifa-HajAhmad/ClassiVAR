// A release build is a window app: no console window opens behind it on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod crash_log;
mod design;
#[cfg(debug_assertions)]
mod dev_reload;
mod genes;
mod icons;
mod labs;
mod look;
mod rule_file;
mod settings;
mod status;
mod theme;
mod view;

use std::path::{Path, PathBuf};

use classivar_core::RuleSet;
use gpui_kit::*;

use view::Workbench;

#[cfg(not(debug_assertions))]
const USAGE: &str = "usage: classivar [--rules <rule-file.toml>]";
#[cfg(debug_assertions)]
const USAGE: &str = "usage: classivar [--rules <rule-file.toml>] [--open <case.classivar.json>] [--theme <theme.json>]";

struct Launch {
    rules_path: Option<PathBuf>,
    #[cfg(debug_assertions)]
    open_path: Option<PathBuf>,
    #[cfg(debug_assertions)]
    theme_path: Option<PathBuf>,
}

fn set_flag(slot: &mut Option<PathBuf>, value: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(USAGE.to_string());
    }
    *slot = Some(PathBuf::from(value));
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Launch, String> {
    let mut launch = Launch {
        rules_path: None,
        #[cfg(debug_assertions)]
        open_path: None,
        #[cfg(debug_assertions)]
        theme_path: None,
    };
    let mut iter = args.iter();
    while let Some(flag) = iter.next() {
        let value = iter.next().ok_or_else(|| USAGE.to_string())?;
        match flag.as_str() {
            "--rules" => set_flag(&mut launch.rules_path, value)?,
            #[cfg(debug_assertions)]
            "--open" => set_flag(&mut launch.open_path, value)?,
            #[cfg(debug_assertions)]
            "--theme" => set_flag(&mut launch.theme_path, value)?,
            _ => return Err(USAGE.to_string()),
        }
    }
    Ok(launch)
}

/// The rules to use: the ones built into the program, or a file given with
/// `--rules`, so a reviewer can try a corrected rule file without a rebuild.
/// Either way the rules are checked here, before any window opens.
fn load_rules(path: Option<&Path>) -> Result<&'static RuleSet, String> {
    match path {
        None => RuleSet::builtin().map_err(|error| format!("the built-in rules are not valid: {error}")),
        Some(path) => rule_file::read_rule_file(path),
    }
}

#[cfg(debug_assertions)]
struct DevStartup {
    case: Option<classivar_core::CaseFile>,
    theme: Option<gpui_kit::component::ThemeSet>,
    watch: dev_reload::DevWatch,
}

#[cfg(debug_assertions)]
fn prepare_dev(launch: Launch, rules: &RuleSet) -> Result<DevStartup, String> {
    let case = launch
        .open_path
        .as_deref()
        .map(|path| {
            view::read_case_file(path, rules).map_err(|error| format!("could not open {}: {error}", path.display()))
        })
        .transpose()?;
    let theme = match launch.theme_path.as_deref() {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
            let parsed = theme::parse_themes(&text)?;
            for warning in theme::contrast_warnings(&text) {
                eprintln!("ClassiVAR theme contrast warning: {warning}");
            }
            Some(parsed)
        }
        None => None,
    };
    let theme_path = launch.theme_path.or_else(dev_reload::default_theme_path);
    Ok(DevStartup { case, theme, watch: dev_reload::DevWatch { theme_path, rules_path: launch.rules_path } })
}

/// The size and limits of a ClassiVAR window, for the first window and for one
/// opened with other rules.
pub(crate) fn window_options(cx: &mut App) -> WindowOptions {
    // Opens maximized; un-maximizing returns to 1240 x 900 in the middle of the screen.
    let bounds = Bounds::centered(None, size(px(1240.), px(900.)), cx);
    let mut options = WindowOptions {
        window_bounds: Some(WindowBounds::Maximized(bounds)),
        // Below this width the criteria list and the results no longer fit side by side.
        window_min_size: Some(size(px(960.), px(600.))),
        ..gpui_kit::component::TitleBar::window_options()
    };
    // The app draws its own title bar; this name is what the taskbar and Alt+Tab show.
    if let Some(titlebar) = options.titlebar.as_mut() {
        titlebar.title = Some("ClassiVAR".into());
    }
    options
}

fn die(message: String) -> ! {
    eprintln!("ClassiVAR could not start: {message}");
    std::process::exit(2);
}

fn main() {
    crash_log::install();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let launch = parse_args(&args).unwrap_or_else(|e| die(e));
    let rules = load_rules(launch.rules_path.as_deref()).unwrap_or_else(|e| die(e));
    #[cfg(debug_assertions)]
    let dev = prepare_dev(launch, rules).unwrap_or_else(|e| die(e));
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        look::install_fonts(cx);
        let mode = settings::load_theme();
        theme::install(mode, cx);
        #[cfg(debug_assertions)]
        if let Some(set) = dev.theme {
            theme::apply_theme_set(set, mode, cx);
        }
        let options = window_options(cx);
        open_window(options, cx, move |window, cx| {
            cx.new(|cx| {
                #[allow(unused_mut)]
                let mut workbench = Workbench::new(rules, window, cx);
                #[cfg(debug_assertions)]
                {
                    if let Some(case) = dev.case {
                        workbench.apply_case(case, window, cx);
                    }
                    workbench.start_dev_reload(dev.watch, window, cx);
                }
                workbench
            })
        })
        .expect("could not open the window");
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_args;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn parse_args_accepts_rules_and_debug_flags() {
        let empty = parse_args(&[]).expect("empty args");
        assert_eq!(empty.rules_path, None);

        let rules_only = parse_args(&words(&["--rules", "custom.toml"])).expect("rules flag");
        assert_eq!(rules_only.rules_path, Some(PathBuf::from("custom.toml")));

        #[cfg(debug_assertions)]
        {
            let combined =
                parse_args(&words(&["--theme", "t.json", "--open", "c.json", "--rules", "r.toml"])).expect("combined");
            assert_eq!(combined.rules_path, Some(PathBuf::from("r.toml")));
            assert_eq!(combined.open_path, Some(PathBuf::from("c.json")));
            assert_eq!(combined.theme_path, Some(PathBuf::from("t.json")));
            assert!(parse_args(&words(&["--open", "a", "--open", "b"])).is_err());
        }

        // The debug flags do not exist in a release build (run with `cargo test --release` to see this).
        #[cfg(not(debug_assertions))]
        {
            assert!(parse_args(&words(&["--open", "c.json"])).is_err());
            assert!(parse_args(&words(&["--theme", "t.json"])).is_err());
        }

        assert!(parse_args(&words(&["--rules"])).is_err());
        assert!(parse_args(&words(&["--rules", "a", "--rules", "b"])).is_err());
        assert!(parse_args(&words(&["--unknown", "x"])).is_err());
    }
}
