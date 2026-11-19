//! Whether the case on screen still matches what was last saved or opened.
//!
//! The answer is worked out by comparing, not by setting a flag at every edit:
//! a flag would have to be set in every place that changes the case, and one
//! missed place would leave a "Saved" message that is no longer true.

use super::*;

/// How a file message reads once the case has changed since. A "Saved" or
/// "Opened" message would then be untrue, so it keeps its facts, says what
/// happened since, and asks for attention instead of showing good news.
/// A message that already asks for attention is left as it is.
pub(super) fn banner_status(status: &Status, unsaved: bool) -> Status {
    if unsaved && status.tone == Tone::Done {
        Status::attention(format!("{} You have changed the case since. Save again to keep the change.", status.text))
    } else {
        status.clone()
    }
}

impl Workbench {
    /// Remember a case as the saved state: what was written to a file, or what was just opened.
    pub(super) fn remember_saved(&mut self, case: CaseFile) {
        self.saved_case = Some(case);
    }

    /// Forget the saved state (the case was cleared or replaced by the example).
    pub(super) fn forget_saved(&mut self) {
        self.saved_case = None;
    }

    /// Would saving now write something different from the last save or open?
    /// False when there is no saved state to compare with.
    pub(super) fn has_unsaved_changes(&self, cx: &App) -> bool {
        let Some(saved) = &self.saved_case else { return false };
        let mut now = self.case_file(cx);
        // The date of saving is not something the curator changed.
        now.saved_on = saved.saved_on.clone();
        now != *saved
    }
}

#[cfg(test)]
mod tests {
    use super::banner_status;
    use crate::status::{Status, Tone};

    #[test]
    fn a_saved_message_stops_being_good_news_once_the_case_has_changed() {
        let saved = Status::done("Saved to C:\\cases\\a.classivar.json.");
        assert_eq!(banner_status(&saved, false), saved);
        let changed = banner_status(&saved, true);
        assert_eq!(changed.tone, Tone::Attention);
        assert!(changed.text.starts_with("Saved to C:\\cases\\a.classivar.json."), "{}", changed.text);
        assert!(changed.text.contains("changed the case since"), "{}", changed.text);
    }

    #[test]
    fn a_message_that_already_asks_for_attention_is_not_changed() {
        let failed = Status::attention("Could not save to a.json: disk full.");
        assert_eq!(banner_status(&failed, true), failed);
    }
}
