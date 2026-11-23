//! Headless window tests for the layout of the criteria tabs and the opening
//! screen's preview.
//!
//! Explicit imports on purpose (see `headless.rs`).

use classivar_core::{Category, Class};
use gpui::{Modifiers, TestAppContext, px, size};
use gpui_kit::component::ThemeMode;

use super::Mode;
use super::headless::{draw, open};
use crate::settings::Picture;

/// A selector built at run time. The test context only looks up `'static` names, and a test may leak a few bytes.
fn selector(name: String) -> &'static str {
    Box::leak(name.into_boxed_str())
}

/// The id of a criteria tab, as the sheet draws it.
fn tab(category: Category) -> &'static str {
    selector(format!("criteria-tab-{}", category.label().to_lowercase().replace(' ', "-")))
}

#[gpui::test]
fn the_criteria_show_one_kind_of_evidence_at_a_time(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(size(px(1920.), px(1080.)));
    workbench.update_in(cx, |this, _, _| this.set_mode(Mode::NewVariant));
    draw(cx);
    assert!(cx.debug_bounds("criterion-PVS1").is_some(), "the first kind of evidence is open at the start");
    assert!(cx.debug_bounds("criterion-PM2").is_none(), "other kinds wait behind their tab");

    let population = cx.debug_bounds(tab(Category::Population)).expect("the Population tab is drawn");
    cx.simulate_click(population.center(), Modifiers::default());
    draw(cx);
    workbench.read_with(cx, |this, _| assert_eq!(this.category, Some(Category::Population)));
    assert!(cx.debug_bounds("criterion-PM2").is_some(), "the Population tab shows its criteria");
    assert!(cx.debug_bounds("criterion-PVS1").is_none(), "and hides the variant-type criteria");
}

#[gpui::test]
fn the_tabs_sit_in_a_row_above_the_criteria_at_every_width(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    workbench.update_in(cx, |this, _, _| this.set_mode(Mode::NewVariant));
    for width in [1920., 960.] {
        cx.simulate_resize(size(px(width), px(1000.)));
        draw(cx);
        let a = cx.debug_bounds(tab(Category::VariantType)).expect("drawn");
        let b = cx.debug_bounds(tab(Category::Population)).expect("drawn");
        let row = cx.debug_bounds("criterion-PVS1").expect("drawn");
        assert_eq!(a.origin.y, b.origin.y, "the tabs sit side by side at {width} px");
        assert!(row.origin.y > a.origin.y, "and above the criteria");
    }
}

#[gpui::test]
fn the_opening_preview_draws_the_chosen_picture(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(size(px(1920.), px(1080.)));
    for picture in Picture::ALL {
        workbench.update_in(cx, |this, _, cx| {
            this.picture = picture;
            cx.notify();
        });
        draw(cx);
        let name = picture.name().to_lowercase();
        assert!(cx.debug_bounds(selector(format!("welcome-preview-{name}"))).is_some(), "the preview draws the {name}");
    }
}

#[gpui::test]
fn pointing_at_a_class_on_the_opening_preview_marks_it(cx: &mut TestAppContext) {
    let (workbench, cx) = open(cx, ThemeMode::Light);
    cx.simulate_resize(size(px(1920.), px(1080.)));
    for picture in Picture::ALL {
        workbench.update_in(cx, |this, _, cx| {
            this.picture = picture;
            this.pointed_zone = None;
            cx.notify();
        });
        draw(cx);
        let likely = cx.debug_bounds("welcome-zone-lp").expect("the class key is drawn");
        cx.simulate_mouse_move(likely.center(), None, Modifiers::default());
        draw(cx);
        workbench.read_with(cx, |this, _| assert_eq!(this.pointed_zone, Some(Class::LikelyPathogenic)));
        let benign = cx.debug_bounds("welcome-zone-b").expect("drawn");
        cx.simulate_click(benign.center(), Modifiers::default());
        draw(cx);
        workbench.read_with(cx, |this, _| assert_eq!(this.pointed_zone, Some(Class::Benign)));
    }
}
