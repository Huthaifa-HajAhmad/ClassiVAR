mod view;

use gpui_kit::*;

use view::Workbench;

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);
        let bounds = Bounds::centered(None, size(px(1240.), px(900.)), cx);
        let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() };
        open_window(options, cx, |window, cx| cx.new(|cx| Workbench::new(window, cx)))
            .expect("could not open the window");
    });
}
