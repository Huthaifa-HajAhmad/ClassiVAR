mod design;
mod genes;
mod labs;
mod look;
mod settings;
mod status;
mod theme;
mod view;

use classivar_core::RuleSet;
use gpui_kit::*;

use view::Workbench;

const USAGE: &str = "usage: classivar [--rules <rule-file.toml>]";

/// The largest rule file that can still be one: the built-in file is about 54 kB.
const MAX_RULE_BYTES: u64 = 1024 * 1024;

/// The rules to use: the ones built into the program, or a file given with
/// `--rules`, so a reviewer can try a corrected rule file without a rebuild.
/// Either way the rules are checked here, before any window opens.
fn load_rules() -> Result<&'static RuleSet, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => RuleSet::builtin().map_err(|error| format!("the built-in rules are not valid: {error}")),
        [flag, path] if flag == "--rules" => {
            // A rule file is a few kilobytes. Refuse a huge one before reading it
            // into memory, the same discipline the network layer uses.
            let size = std::fs::metadata(path)
                .map(|meta| meta.len())
                .map_err(|error| format!("could not read {path}: {error}"))?;
            if size > MAX_RULE_BYTES {
                return Err(format!("{path} is larger than {} kB and cannot be a rule file", MAX_RULE_BYTES / 1024));
            }
            let text = std::fs::read_to_string(path).map_err(|error| format!("could not read {path}: {error}"))?;
            let rules = RuleSet::parse(&text).map_err(|error| format!("{path} is not a valid rule file: {error}"))?;
            // The rules live until the program ends, so leaking them once is simple and safe.
            Ok(Box::leak(Box::new(rules)))
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() {
    let rules = match load_rules() {
        Ok(rules) => rules,
        Err(message) => {
            eprintln!("ClassiVAR could not start: {message}");
            std::process::exit(2);
        }
    };
    application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        look::install_fonts(cx);
        // Start in the theme last chosen on this machine (light if there is no choice).
        theme::install(settings::load_theme(), cx);
        let bounds = Bounds::centered(None, size(px(1240.), px(900.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // Below this width the criteria list and the results no longer fit side by side.
            window_min_size: Some(size(px(960.), px(600.))),
            ..gpui_kit::component::TitleBar::window_options()
        };
        open_window(options, cx, |window, cx| cx.new(|cx| Workbench::new(rules, window, cx)))
            .expect("could not open the window");
    });
}
