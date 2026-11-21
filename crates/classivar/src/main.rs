mod design;
mod genes;
mod labs;
mod look;
mod rule_file;
mod settings;
mod status;
mod theme;
mod view;

use classivar_core::RuleSet;
use gpui_kit::*;

use view::Workbench;

const USAGE: &str = "usage: classivar [--rules <rule-file.toml>]";

/// The rules to use: the ones built into the program, or a file given with
/// `--rules`, so a reviewer can try a corrected rule file without a rebuild.
/// Either way the rules are checked here, before any window opens.
fn load_rules() -> Result<&'static RuleSet, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => RuleSet::builtin().map_err(|error| format!("the built-in rules are not valid: {error}")),
        [flag, path] if flag == "--rules" => rule_file::read_rule_file(std::path::Path::new(path)),
        _ => Err(USAGE.to_string()),
    }
}

/// The size and limits of a ClassiVAR window, for the first window and for one
/// opened with other rules.
pub(crate) fn window_options(cx: &mut App) -> WindowOptions {
    let bounds = Bounds::centered(None, size(px(1240.), px(900.)), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        // Below this width the criteria list and the results no longer fit side by side.
        window_min_size: Some(size(px(960.), px(600.))),
        ..gpui_kit::component::TitleBar::window_options()
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
        let options = window_options(cx);
        open_window(options, cx, |window, cx| cx.new(|cx| Workbench::new(rules, window, cx)))
            .expect("could not open the window");
    });
}
