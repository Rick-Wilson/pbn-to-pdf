use super::card::{Rank, Suit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSpan {
    Plain(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    SuitSymbol(Suit),
    CardRef { suit: Suit, rank: Rank },
    LineBreak,
}

impl TextSpan {
    pub fn plain(s: impl Into<String>) -> Self {
        TextSpan::Plain(s.into())
    }

    pub fn bold(s: impl Into<String>) -> Self {
        TextSpan::Bold(s.into())
    }

    pub fn italic(s: impl Into<String>) -> Self {
        TextSpan::Italic(s.into())
    }

    pub fn bold_italic(s: impl Into<String>) -> Self {
        TextSpan::BoldItalic(s.into())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FormattedText {
    pub spans: Vec<TextSpan>,
}

impl FormattedText {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn push(&mut self, span: TextSpan) {
        self.spans.push(span);
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn to_plain_text(&self) -> String {
        let mut result = String::new();
        for span in &self.spans {
            match span {
                TextSpan::Plain(s)
                | TextSpan::Bold(s)
                | TextSpan::Italic(s)
                | TextSpan::BoldItalic(s) => {
                    result.push_str(s);
                }
                TextSpan::SuitSymbol(suit) => {
                    result.push(suit.symbol());
                }
                TextSpan::CardRef { suit, rank } => {
                    result.push(suit.symbol());
                    result.push(rank.to_char());
                }
                TextSpan::LineBreak => {
                    result.push('\n');
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct CommentaryBlock {
    pub content: FormattedText,
}

impl CommentaryBlock {
    pub fn new(content: FormattedText) -> Self {
        Self { content }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatted_text_to_plain() {
        let mut text = FormattedText::new();
        text.push(TextSpan::Bold("Bidding.".to_string()));
        text.push(TextSpan::Plain(" Open 1".to_string()));
        text.push(TextSpan::SuitSymbol(Suit::Spades));

        assert_eq!(text.to_plain_text(), "Bidding. Open 1♠");
    }
}
