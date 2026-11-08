//! A message about a file or a case action, with what kind it is, so good news
//! and problems do not share one colour (see `design.rs` for the colours).

/// How the message should look: finished as expected, or something to read and act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Done,
    Attention,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub tone: Tone,
    pub text: String,
}

impl Status {
    pub fn done(text: impl Into<String>) -> Self {
        Self { tone: Tone::Done, text: text.into() }
    }

    pub fn attention(text: impl Into<String>) -> Self {
        Self { tone: Tone::Attention, text: text.into() }
    }
}
