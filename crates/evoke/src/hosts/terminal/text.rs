//! Text for the terminal: the plain line, and the roles its spans carry. `report` says what a span is; the
//! terminal says how it looks — colour and weight where it shows them, the plain text everywhere else, so a pipe,
//! a log and the transcripts read the same words. In: pieces of text, roled or not. Out: the plain text; the
//! styled text.

use std::fmt;

use evoke_core::manifest::Effect;

/// What a span of text is, so the terminal can weight it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A reflex's name where a call is shown.
    Call,
    /// The most probable answer of a question, where every answer is shown.
    Top,
    /// The effect a call runs under.
    Effect(Effect),
    /// The weakest judgment, or a confidence.
    Weak,
    /// The arrow before a fix.
    Fix,
    /// The `+` of a write, or of a line that is yours.
    Added,
    /// The `-` of a removal.
    Removed,
    /// A failed count, a regression.
    Failed,
    /// `inactive`, `lint`: a label that asks for attention.
    Warning,
}

impl Role {
    /// The Select Graphic Rendition the role takes: weight for the call, a light for each effect, grey for what
    /// is secondary, the accent on the arrow, green and red on the signs.
    fn sgr(self) -> &'static str {
        match self {
            Self::Call | Self::Top => "1",
            Self::Weak => "2",
            Self::Effect(Effect::Read) | Self::Added => "32",
            Self::Effect(Effect::Write) | Self::Warning => "33",
            Self::Effect(Effect::Destructive) | Self::Removed | Self::Failed => "31",
            Self::Fix => "36",
        }
    }
}

/// A line or block for the terminal, built piece by piece; lines within it are separated by `\n`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    pieces: Vec<Piece>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Piece {
    role: Option<Role>,
    text: String,
}

impl Text {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Plain text on the end.
    pub fn push(&mut self, text: &str) -> &mut Self {
        self.piece(None, text)
    }

    /// Text with a role on the end.
    pub fn roled(&mut self, role: Role, text: &str) -> &mut Self {
        self.piece(Some(role), text)
    }

    /// A whole text on the end.
    pub fn append(&mut self, text: Text) -> &mut Self {
        self.pieces.extend(text.pieces);
        self
    }

    fn piece(&mut self, role: Option<Role>, text: &str) -> &mut Self {
        if !text.is_empty() {
            self.pieces.push(Piece {
                role,
                text: text.to_owned(),
            });
        }
        self
    }

    /// Texts one under the other.
    #[must_use]
    pub fn lines(lines: impl IntoIterator<Item = Text>) -> Self {
        let mut text = Self::new();
        for (i, line) in lines.into_iter().enumerate() {
            if i > 0 {
                text.push("\n");
            }
            text.append(line);
        }
        text
    }

    /// Without the spaces a padded column leaves at the end of a line.
    #[must_use]
    pub fn trim_end(mut self) -> Self {
        while let Some(last) = self.pieces.last_mut() {
            let kept = last.text.trim_end_matches(' ').len();
            if kept == last.text.len() {
                break;
            }
            last.text.truncate(kept);
            if last.text.is_empty() {
                self.pieces.pop();
            }
        }
        self
    }

    /// The text with each role's colour and weight around its span, reset after it.
    #[must_use]
    pub fn styled(&self) -> String {
        let mut styled = String::new();
        for piece in &self.pieces {
            match piece.role {
                Some(role) => {
                    styled.push_str("\x1b[");
                    styled.push_str(role.sgr());
                    styled.push('m');
                    styled.push_str(&piece.text);
                    styled.push_str("\x1b[0m");
                }
                None => styled.push_str(&piece.text),
            }
        }
        styled
    }

    /// Each roled span with its role, for a test to read.
    #[cfg(test)]
    pub fn roles(&self) -> Vec<(Role, &str)> {
        self.pieces
            .iter()
            .filter_map(|piece| piece.role.map(|role| (role, piece.text.as_str())))
            .collect()
    }
}

/// The plain text: what a pipe, a log and the transcripts see.
impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for piece in &self.pieces {
            f.write_str(&piece.text)?;
        }
        Ok(())
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        let mut plain = Self::new();
        plain.push(&text);
        plain
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        let mut plain = Self::new();
        plain.push(text);
        plain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plain_text_is_the_words_alone_and_the_styled_one_wraps_each_role() {
        let mut text = Text::from("  ");
        text.roled(Role::Call, "lights")
            .push(" room=\"den\"  ")
            .roled(Role::Weak, "0.85");
        assert_eq!(text.to_string(), "  lights room=\"den\"  0.85");
        assert_eq!(
            text.styled(),
            "  \x1b[1mlights\x1b[0m room=\"den\"  \x1b[2m0.85\x1b[0m"
        );
        assert_eq!(
            text.roles(),
            vec![(Role::Call, "lights"), (Role::Weak, "0.85")]
        );
    }

    #[test]
    fn each_effect_has_its_light_and_each_sign_its_colour() {
        let mut text = Text::new();
        text.roled(Role::Effect(Effect::Read), "read")
            .roled(Role::Effect(Effect::Write), "write")
            .roled(Role::Effect(Effect::Destructive), "destructive")
            .roled(Role::Added, "+")
            .roled(Role::Removed, "-")
            .roled(Role::Fix, "→");
        assert_eq!(
            text.styled(),
            "\x1b[32mread\x1b[0m\x1b[33mwrite\x1b[0m\x1b[31mdestructive\x1b[0m\x1b[32m+\x1b[0m\x1b[31m-\x1b[0m\x1b[36m→\x1b[0m"
        );
    }

    #[test]
    fn lines_stack_and_a_padded_end_is_trimmed() {
        let mut first = Text::from("a");
        first.push("   ");
        let text = Text::lines([first.trim_end(), Text::from("b  ").trim_end()]);
        assert_eq!(text.to_string(), "a\nb");
        assert_eq!(Text::from("   ").trim_end(), Text::new());
        assert_eq!(Text::from(""), Text::new());
    }
}
