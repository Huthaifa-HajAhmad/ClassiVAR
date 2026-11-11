//! What the window watches: typing in the boxes, and the keys that work anywhere.

use super::*;

/// Repaint the screen whenever the text in a box changes. The panels read what a
/// box holds while they draw (a result title, a helper's reading), so they must
/// draw again after each keystroke.
pub(super) fn repaint_on_change(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) -> Subscription {
    cx.subscribe_in(input, window, |_this, _, event: &InputEvent, _window, cx| {
        if matches!(event, InputEvent::Change) {
            cx.notify();
        }
    })
}

/// Enter in the gene box checks the gene; typing refreshes the fill-in suggestions under it.
pub(super) fn watch_gene_box(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) -> Subscription {
    cx.subscribe_in(input, window, |this, _, event: &InputEvent, _window, cx| match event {
        InputEvent::PressEnter { .. } => this.check_gene(cx),
        InputEvent::Change => cx.notify(),
        InputEvent::Focus | InputEvent::Blur => {}
    })
}

/// Typing in the name box renames the record that is open. A programmatic
/// change gives the same text, so it is harmless.
pub(super) fn watch_record_name(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<Workbench>,
) -> Subscription {
    cx.subscribe_in(input, window, |this, input, event: &InputEvent, _window, cx| {
        if !matches!(event, InputEvent::Change) {
            return;
        }
        let Editing::Record(i) = this.editing else { return };
        let typed = input.read(cx).value();
        let name: String = typed.trim().chars().take(MAX_LAB_CHARS).collect();
        if let Some(record) = this.records.get_mut(i) {
            record.label = if name.is_empty() { UNNAMED_RECORD.to_string() } else { name };
            cx.notify();
        }
    })
}

/// Escape is watched for the whole window: no element has to hold keyboard
/// focus (after a click on a plain button none may), and a focused text box
/// must not hide it. Page and Home/End keys scroll the pane under the pointer.
pub(super) fn watch_keys(cx: &mut Context<Workbench>) -> Subscription {
    cx.observe_keystrokes(|this, event, window, cx| match event.keystroke.key.as_str() {
        "escape" => this.escape(window, cx),
        key @ ("pageup" | "pagedown" | "home" | "end") => this.scroll_with_key(key, window, cx),
        _ => {}
    })
}
