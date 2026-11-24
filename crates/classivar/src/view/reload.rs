//! Debug-only live reload hooks on `Workbench`.

use super::*;

impl Workbench {
    /// Start watching the theme and `--rules` files for live reload in debug builds.
    pub(crate) fn start_dev_reload(
        &mut self,
        watch: crate::dev_reload::DevWatch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._dev_watch = crate::dev_reload::start_watcher(watch, window, cx);
    }

    pub(crate) fn set_dev_status(&mut self, status: Status, cx: &mut Context<Self>) {
        self.set_file_status(status, cx);
    }

    /// Replace the active rule set live, carrying today's evidence forward and dropping stale focus state.
    /// Earlier reports keep the evidence they recorded; one that uses a criterion these rules lack is named.
    pub(crate) fn reload_rules(&mut self, rules: &'static RuleSet, window: &mut Window, cx: &mut Context<Self>) {
        self.rules = rules;
        let (today, changes) = carry_forward(rules, &to_applied(&self.today));
        self.today = today.into_iter().collect();
        // Another rule version is not something the curator changed.
        if let Some(saved) = &mut self.saved_case {
            saved.rule_set = rules.reference();
        }
        self.sanitize_selection(rules);
        self.sanitize_helpers(rules, window, cx);
        let mut adjusted = open_notes(&changes);
        for record in &self.records {
            for code in record.evidence.keys().filter(|code| rules.criterion(code).is_none()) {
                adjusted.push(format!(
                    "{} {} uses {code}, which these rules lack, so it cannot be scored.",
                    record.label, record.year
                ));
            }
        }
        let tag = rules_tag(rules);
        let status = if adjusted.is_empty() {
            Status::done(format!("Reloaded rules {} ({tag}).", rules.meta.id))
        } else {
            Status::attention(format!("Reloaded rules {} ({tag}). Adjusted: {}", rules.meta.id, adjusted.join(" ")))
        };
        self.set_file_status(status, cx);
    }

    fn sanitize_selection(&mut self, rules: &RuleSet) {
        self.raising = None;
        if self.about.as_deref().is_some_and(|code| rules.criterion(code).is_none()) {
            self.about = None;
        }
        // A guide answer of the old rules may not exist in the new ones, so the guide closes.
        self.close_guide();
        if self.inspected.as_deref().is_some_and(|code| !self.current().contains_key(code)) {
            self.inspected = None;
            self.preview = false;
        }
        if self.hovered.as_deref().is_some_and(|code| !self.current().contains_key(code)) {
            self.hovered = None;
        }
    }

    fn sanitize_helpers(&mut self, rules: &RuleSet, window: &mut Window, cx: &mut Context<Self>) {
        if self.helpers.predictor.tool >= rules.predictors.len() {
            self.helpers.predictor.tool = 0;
        }
        if rules.locus_evidence.as_ref().is_none_or(|l| self.helpers.locus.kind >= l.segregation.len()) {
            self.helpers.locus.kind = 0;
        }
        if rules.non_segregation.as_ref().is_none_or(|n| self.helpers.segregation.situation >= n.situations.len()) {
            self.helpers.segregation.situation = 0;
        }
        let pm3_rows = rules.pm3.as_ref().map_or(0, |t| t.rows.len());
        let de_novo_rows = rules.de_novo.as_ref().map_or(0, |t| t.phenotypes.len());
        if self.helpers.pm3.inputs.len() != pm3_rows || self.helpers.de_novo.inputs.len() != de_novo_rows {
            self.helpers = Helpers::new(rules, &mut self._subscriptions, window, cx);
        }
    }
}
