//! "What it is made of": the evidence grouped by kind (population, function,
//! family and so on). Each kind that has evidence is one row: its criteria as
//! chips, a small two-sided bar (points for disease to the right, against to
//! the left, on one shared length so rows compare), and its net points. The
//! kinds with no evidence yet are named on one quiet line, because that is
//! where evidence that could move the class would come from.
//!
//! The grouping and the sums are a plain function (`makeup`) with tests; the
//! drawing below only lays it out.

use classivar_core::Item;

use super::point_axis::signed;
use super::*;
use crate::look::{eyebrow, figure};

/// One kind of evidence that has at least one criterion.
#[derive(Debug)]
struct Lane<'a> {
    category: Category,
    items: Vec<&'a Item>,
    /// Counted points for disease, and against it (as a positive number).
    for_points: i32,
    against_points: i32,
}

/// The rows to draw, the kinds with nothing yet, and the longest side of any
/// row, so every two-sided bar uses the same length per point.
#[derive(Debug)]
struct Makeup<'a> {
    lanes: Vec<Lane<'a>>,
    empty: Vec<Category>,
    longest: i32,
    /// Whether any row has points against the disease, and whether any has points for it.
    /// A side no row uses gets no room in the bars.
    sides: (bool, bool),
}

/// Group `items` by the kind `category_of` gives each code, in the order of
/// `Category::ALL`. A criterion that is listed but not counted is kept in its
/// row (it is shown) and adds no points.
fn makeup(items: &[Item], category_of: impl Fn(&str) -> Option<Category>) -> Makeup<'_> {
    let mut lanes = Vec::new();
    let mut empty = Vec::new();
    for category in Category::ALL {
        let mine: Vec<&Item> = items.iter().filter(|item| category_of(&item.code) == Some(category)).collect();
        if mine.is_empty() {
            empty.push(category);
            continue;
        }
        let side = |sign: i32| {
            mine.iter().filter(|item| item.counted && item.points * sign > 0).map(|item| item.points.abs()).sum()
        };
        lanes.push(Lane { category, for_points: side(1), against_points: side(-1), items: mine });
    }
    let longest = lanes.iter().map(|lane| lane.for_points.max(lane.against_points)).max().unwrap_or(0);
    let sides = (lanes.iter().any(|lane| lane.against_points > 0), lanes.iter().any(|lane| lane.for_points > 0));
    Makeup { lanes, empty, longest, sides }
}

/// One line over the rows, in the place where the 2015 table has its sentence,
/// so the two sections side by side start their cards at the same height.
fn summary(drawn: &Makeup) -> String {
    let criteria: usize = drawn.lanes.iter().map(|lane| lane.items.len()).sum();
    let kinds = drawn.lanes.len();
    let for_points: i32 = drawn.lanes.iter().map(|lane| lane.for_points).sum();
    let against: i32 = drawn.lanes.iter().map(|lane| lane.against_points).sum();
    format!(
        "{criteria} {} in {kinds} {}: {} for disease, {} against.",
        if criteria == 1 { "criterion" } else { "criteria" },
        if kinds == 1 { "kind" } else { "kinds" },
        signed(for_points),
        signed(-against),
    )
}

/// Width of the kind's name, of the two-sided bar, and of the net points.
const NAME_WIDTH: f32 = 104.;
const BAR_WIDTH: f32 = 84.;
const NET_WIDTH: f32 = 44.;

impl Workbench {
    /// The rows of evidence by kind, in one card, and the kinds with nothing yet.
    pub(super) fn makeup(&self, result: &Assessment, cx: &mut Context<Self>) -> AnyElement {
        let drawn = makeup(&result.items, |code| self.rules.criterion(code).map(|criterion| criterion.category));
        let border = cx.theme().border;
        let mut rows = crate::look::card(cx).p_0().px_4().py_1().flex().flex_col();
        for (i, lane) in drawn.lanes.iter().enumerate() {
            rows = rows.child(lane_row(lane, &drawn, cx).when(i > 0, |row| row.border_t_1().border_color(border)));
        }
        if !drawn.empty.is_empty() {
            rows = rows.child(
                empty_row(&drawn.empty, cx).when(!drawn.lanes.is_empty(), |row| row.border_t_1().border_color(border)),
            );
        }
        div().flex().flex_col().gap_3().child(div().text_sm().child(summary(&drawn))).child(rows).into_any_element()
    }
}

/// One kind: its name, its criteria as chips, the two-sided bar and the net points.
fn lane_row(lane: &Lane, drawn: &Makeup, cx: &App) -> Div {
    // Clipped, so a chip in a narrow column never runs into the bar beside it.
    let mut chips = div().flex_1().min_w_0().overflow_hidden().flex().flex_wrap().items_center().gap_2();
    for item in &lane.items {
        chips = chips.child(chip(item, cx));
    }
    let net = lane.for_points - lane.against_points;
    div()
        .flex()
        .items_center()
        .gap_3()
        .py(px(10.))
        .child(div().w(px(NAME_WIDTH)).flex_none().child(eyebrow(lane.category.label(), cx)))
        .child(chips)
        .child(two_sided(lane, drawn, cx))
        .child(figure(13.).w(px(NET_WIDTH)).flex_none().flex().justify_end().child(signed(net)))
}

/// A criterion: its code on its direction colour, its points and its strength.
/// One that is listed but not counted has a dashed outline and says so.
fn chip(item: &Item, cx: &App) -> Div {
    let theme = cx.theme();
    let tag = figure(11.).px_2().h(px(20.)).flex().items_center().rounded_full().child(item.code.clone());
    let (tag, words) = if item.counted {
        let tag = tag.bg(direction_colour(item.direction)).text_color(direction_ink());
        (tag, format!("{} {}", signed(item.points), item.strength.label()))
    } else {
        let tag =
            tag.border_1().border_dashed().border_color(theme.muted_foreground).text_color(theme.muted_foreground);
        (tag, format!("{}: listed, not counted", item.strength.label()))
    };
    div()
        .max_w_full()
        .flex()
        .items_center()
        .gap_2()
        .pl(px(3.))
        .pr_3()
        .py(px(3.))
        .rounded_full()
        .bg(theme.popover)
        .border_1()
        .border_color(theme.border)
        .child(tag)
        .child(div().min_w_0().truncate().text_xs().text_color(theme.muted_foreground).child(words))
}

/// Points against the disease grow left from the zero line, points for it grow
/// right, on the same length per point in every row. A side no row uses gets no room.
fn two_sided(lane: &Lane, drawn: &Makeup, cx: &App) -> Div {
    let room = BAR_WIDTH - 4.;
    let (against_room, for_room) = match drawn.sides {
        (true, true) => (room / 2., room / 2.),
        (true, false) => (room, 0.),
        _ => (0., room),
    };
    let longest = drawn.longest.max(1) as f32;
    let side = |points: i32, direction: Direction, width: f32| {
        div().h(px(8.)).w(px(points as f32 / longest * width)).rounded_full().bg(direction_colour(direction))
    };
    div()
        .w(px(BAR_WIDTH))
        .flex_none()
        .h(px(14.))
        .flex()
        .items_center()
        .child(div().w(px(against_room)).flex().justify_end().child(side(
            lane.against_points,
            Direction::Benign,
            against_room,
        )))
        .child(div().w(px(1.)).h_full().mx(px(1.5)).bg(cx.theme().muted_foreground))
        .child(div().w(px(for_room)).flex().child(side(lane.for_points, Direction::Pathogenic, for_room)))
}

/// The kinds with no evidence yet, as quiet outlined names.
fn empty_row(empty: &[Category], cx: &App) -> Div {
    let theme = cx.theme();
    let names = empty.iter().map(|category| {
        div()
            .px_2()
            .py(px(2.))
            .rounded_full()
            .border_1()
            .border_dashed()
            .border_color(theme.border)
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(category.label())
    });
    div()
        .flex()
        .items_center()
        .gap_3()
        .py(px(10.))
        .child(div().w(px(NAME_WIDTH)).flex_none().child(eyebrow("Nothing yet", cx)))
        .child(div().flex_1().flex().flex_wrap().gap_2().children(names))
}

#[cfg(test)]
mod tests {
    // Explicit imports on purpose; see the note in `view.rs` tests.
    use classivar_core::{Applied, Category, RuleSet, Strength};

    use super::{makeup, summary};

    fn rules() -> &'static RuleSet {
        RuleSet::builtin().expect("the built-in rules are valid")
    }

    fn category_of(code: &str) -> Option<Category> {
        rules().criterion(code).map(|criterion| criterion.category)
    }

    #[test]
    fn rows_follow_the_category_order_and_sum_each_side() {
        let result = rules()
            .assess(&[
                Applied::new("PP3", Strength::Supporting),
                Applied::new("PM2", Strength::Supporting),
                Applied::new("BS1", Strength::Strong),
                Applied::new("PS3", Strength::Strong),
            ])
            .unwrap();
        let drawn = makeup(&result.items, category_of);
        let order: Vec<Category> = drawn.lanes.iter().map(|lane| lane.category).collect();
        let expected: Vec<Category> = Category::ALL.into_iter().filter(|c| order.contains(c)).collect();
        assert_eq!(order, expected, "rows keep the order of Category::ALL");
        let population = drawn.lanes.iter().find(|lane| lane.category == Category::Population).unwrap();
        assert_eq!((population.for_points, population.against_points), (1, 4), "PM2 +1 and BS1 −4");
        assert_eq!(drawn.longest, 4);
        assert_eq!(drawn.sides, (true, true));
        assert!(drawn.empty.contains(&Category::FamilyCases));
        assert!(!drawn.empty.contains(&Category::Population));
        assert_eq!(drawn.lanes.len() + drawn.empty.len(), Category::ALL.len());
    }

    #[test]
    fn the_summary_counts_criteria_kinds_and_both_sides() {
        let result = rules()
            .assess(&[
                Applied::new("PP3", Strength::Supporting),
                Applied::new("PM2", Strength::Supporting),
                Applied::new("BS1", Strength::Strong),
                Applied::new("PS3", Strength::Strong),
            ])
            .unwrap();
        let text = summary(&makeup(&result.items, category_of));
        assert_eq!(text, "4 criteria in 3 kinds: +6 for disease, −4 against.");
        let one = rules().assess(&[Applied::new("PM2", Strength::Supporting)]).unwrap();
        assert_eq!(summary(&makeup(&one.items, category_of)), "1 criterion in 1 kind: +1 for disease, 0 against.");
    }

    #[test]
    fn a_criterion_that_is_listed_but_not_counted_is_shown_and_adds_nothing() {
        let result = rules().assess(&[Applied::new("PP5", Strength::Supporting)]).unwrap();
        assert!(!result.items[0].counted, "PP5 is not counted in a new assessment");
        let drawn = makeup(&result.items, category_of);
        assert_eq!(drawn.lanes.len(), 1);
        assert_eq!(drawn.lanes[0].items.len(), 1);
        assert_eq!((drawn.lanes[0].for_points, drawn.lanes[0].against_points), (0, 0));
        assert_eq!(drawn.longest, 0);
        assert_eq!(drawn.sides, (false, false), "nothing counted, so neither side needs room");
    }

    #[test]
    fn evidence_on_one_side_only_gives_the_bars_one_side() {
        let result = rules()
            .assess(&[Applied::new("PM2", Strength::Supporting), Applied::new("PS3", Strength::Strong)])
            .unwrap();
        assert_eq!(makeup(&result.items, category_of).sides, (false, true));
    }

    #[test]
    fn no_evidence_leaves_every_kind_empty() {
        let drawn = makeup(&[], category_of);
        assert!(drawn.lanes.is_empty());
        assert_eq!(drawn.empty, Category::ALL);
    }
}
