//! Three short cards under the picture: what the 2015 table says, what would
//! change the class, and how today compares with the latest earlier report (or,
//! with no reports, where the total sits inside its class).
//!
//! Every number comes from the engine (`classic_class`, `to_upgrade`,
//! `to_downgrade`, the scored records), so a card cannot disagree with the
//! picture. The cards are drawn only while a class is shown.

use classivar_core::Assessment;

use super::point_axis::points_text;
use super::*;
use crate::look::{eyebrow, figure};

/// One card: a small label, a short headline and one quiet line.
fn insight(label: String, headline: String, line: String, cx: &App) -> Div {
    crate::look::card(cx)
        .flex_1()
        .min_w(px(180.))
        .flex()
        .flex_col()
        .gap_1()
        .child(eyebrow(label, cx))
        .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(headline))
        .child(caption(cx.theme().muted_foreground).child(line))
}

/// The headline for the 2015 table card.
fn table_headline(result: &Assessment) -> String {
    match (result.classic_class, result.frameworks_agree()) {
        (Some(class), Some(true)) => format!("Agrees: {}", class.short()),
        (Some(class), _) => format!("Differs: {}", class.short()),
        (None, _) => "Does not cover these strengths".to_string(),
    }
}

/// The moves to the next class up and down, in points, from the engine.
fn moves_headline(result: &Assessment) -> (String, String) {
    let up = result.to_upgrade.map(|next| format!("+{} → {}", next.points_needed, next.class.short()));
    let down = result.to_downgrade.map(|next| format!("−{} → {}", next.points_needed, next.class.short()));
    match (up, down) {
        (Some(up), Some(down)) => (up, down),
        (Some(only), None) | (None, Some(only)) => (only, "Only one move is possible from here.".to_string()),
        (None, None) => ("No move".to_string(), "Stand-alone evidence fixes this class.".to_string()),
    }
}

/// How today compares with an earlier total, for example "4 → 5 points (+1)".
fn change_line(earlier: i32, today: i32) -> String {
    let delta = today - earlier;
    let sign = if delta > 0 {
        "+"
    } else if delta < 0 {
        "−"
    } else {
        "±"
    };
    format!("{} → {} points ({sign}{})", points_text(earlier), points_text(today), delta.unsigned_abs())
}

impl Workbench {
    pub(super) fn insights(&self, result: &Assessment, class: Class, cx: &mut Context<Self>) -> Div {
        let (up, down) = moves_headline(result);
        let third = match self.earlier_totals().into_iter().max_by_key(|report| report.year) {
            Some(latest) if self.editing == Editing::Today => insight(
                format!("Since {} · {}", latest.name, latest.year),
                change_line(latest.total, result.total_points),
                "Its recorded evidence, scored the way it was made.".to_string(),
                cx,
            ),
            _ => self.range_card(result, class, cx),
        };
        div()
            .flex()
            .flex_wrap()
            .gap_3()
            .child(insight("2015 table".into(), table_headline(result), super::hero::classic_comparison(result), cx))
            .child(insight("What would change it".into(), up, down, cx))
            .child(third)
    }

    /// Where the total sits inside the points range of its class.
    fn range_card(&self, result: &Assessment, class: Class, cx: &App) -> Div {
        let scale = scale_for(self.rules);
        let last = scale.zones.len().saturating_sub(1);
        let place = scale.zones.iter().position(|(zone, _, _)| *zone == class);
        let short = class.short();
        let line = match place.map(|i| (i, scale.zones[i])) {
            Some((0, (_, _, next))) => format!("{short} is {} points or fewer.", points_text(next - 1)),
            Some((i, (_, low, _))) if i == last => format!("{short} starts at {} points.", points_text(low)),
            Some((_, (_, low, next))) => {
                format!("{short} covers {} to {} points.", points_text(low), points_text(next - 1))
            }
            None => String::new(),
        };
        let total = figure(13.).child(format!("At {} points", points_text(result.total_points)));
        crate::look::card(cx)
            .flex_1()
            .min_w(px(180.))
            .flex()
            .flex_col()
            .gap_1()
            .child(eyebrow("Inside the class", cx))
            .child(total)
            .child(caption(cx.theme().muted_foreground).child(line))
    }
}

#[cfg(test)]
mod tests {
    use classivar_core::{Applied, RuleSet, Strength};

    use super::{change_line, moves_headline, table_headline};

    #[test]
    fn the_change_since_a_report_is_written_with_its_sign() {
        assert_eq!(change_line(4, 5), "4 → 5 points (+1)");
        assert_eq!(change_line(5, 2), "5 → 2 points (−3)");
        assert_eq!(change_line(3, 3), "3 → 3 points (±0)");
        assert_eq!(change_line(-2, 1), "−2 → 1 points (+3)");
    }

    #[test]
    fn the_cards_read_the_engine_numbers() {
        let rules = RuleSet::builtin().unwrap();
        let result = rules
            .assess(&[Applied::new("PM2", Strength::Supporting), Applied::new("PP3", Strength::Supporting)])
            .unwrap();
        assert!(table_headline(&result).starts_with("Agrees"), "{}", table_headline(&result));
        let (up, down) = moves_headline(&result);
        let next = result.to_upgrade.unwrap();
        assert_eq!(up, format!("+{} → {}", next.points_needed, next.class.short()));
        assert!(down.starts_with('−'), "{down}");
    }
}
