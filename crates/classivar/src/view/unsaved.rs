//! Whether the case on screen still matches what was last saved or opened.
//!
//! The answer is worked out by comparing, not by setting a flag at every edit:
//! a flag would have to be set in every place that changes the case, and one
//! missed place would leave a "Saved" message that is no longer true.
//!
//! Two places show the answer: a marker beside the Save button, and the file
//! message. The marker is always shown when the case differs. The message only
//! changes when it was good news ("Saved", "Opened"); a message that already
//! asks for attention (a case opened under other rules, a failed save) keeps its
//! own words, so the two can say different things at once, on purpose.

use super::*;

/// How a file message reads once the case has changed since. A "Saved" or
/// "Opened" message would then be untrue, so it keeps its facts, says what
/// happened since, and asks for attention instead of showing good news.
/// A message that already asks for attention is left as it is.
pub(super) fn banner_status(status: &Status, unsaved: bool) -> Status {
    if unsaved && status.tone == Tone::Done {
        Status::attention(format!("{} You have changed the case since. Save to keep the change.", status.text))
    } else {
        status.clone()
    }
}

/// What a screen reader says for the Save button: the state is part of the name,
/// because the visible marker beside it is a separate piece of text.
pub(super) fn save_button_label(unsaved: bool) -> &'static str {
    if unsaved { "Save case… (changed since saved)" } else { "Save case…" }
}

impl Workbench {
    /// The window's close button was pressed. A case that still matches its file
    /// closes at once. Otherwise the close is refused and a question is shown. A
    /// question that was already open is cancelled first, so the one about
    /// closing is the one the curator sees (a second question never replaces the
    /// first elsewhere, but here the curator has just asked to leave).
    pub(super) fn close_requested(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.would_lose_work(cx) {
            return true;
        }
        if self.confirm.is_some() {
            self.cancel_confirm(window, cx);
        }
        self.ask(Confirm::CloseWindow);
        cx.notify();
        false
    }

    /// Would closing the window now throw work away? A case that was saved or
    /// opened is compared with its file. A case never saved holds work as soon
    /// as it holds any data. The example is a demo and is never saved, so it
    /// holds none.
    fn would_lose_work(&self, cx: &App) -> bool {
        if self.example {
            return false;
        }
        match self.saved_case {
            Some(_) => self.has_unsaved_changes(cx),
            None => self.has_case_data(cx),
        }
    }

    /// Remember a case as the saved state: what was written to a file, or what was just opened.
    pub(super) fn remember_saved(&mut self, case: CaseFile) {
        self.saved_case = Some(case);
    }

    /// The save dialog is finished. A finished save makes what was written the
    /// saved state; a failed save or a closed dialog does not move it, because
    /// nothing was written.
    pub(super) fn finish_save(&mut self, message: Option<Status>, written: CaseFile, cx: &mut Context<Self>) {
        if message.as_ref().is_some_and(|status| status.tone == Tone::Done) {
            self.remember_saved(written);
        }
        if message.is_some() {
            self.file_status = message;
        }
        cx.notify();
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
    use super::{banner_status, save_button_label};
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
    fn the_save_button_name_carries_the_changed_state() {
        assert_eq!(save_button_label(false), "Save case…");
        assert_eq!(save_button_label(true), "Save case… (changed since saved)");
    }

    #[test]
    fn an_opened_message_does_not_say_save_again() {
        let opened = Status::done("Opened. Check the gene again to refresh the gene sources.");
        let changed = banner_status(&opened, true);
        assert!(changed.text.starts_with("Opened."), "{}", changed.text);
        assert!(!changed.text.contains("again to keep"), "nothing was saved in this session: {}", changed.text);
        assert!(changed.text.ends_with("Save to keep the change."), "{}", changed.text);
    }

    #[test]
    fn a_message_that_already_asks_for_attention_is_not_changed() {
        let failed = Status::attention("Could not save to a.json: disk full.");
        assert_eq!(banner_status(&failed, true), failed);
    }
}
