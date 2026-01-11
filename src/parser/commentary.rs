use crate::model::{CommentaryBlock, FormattedText, Rank, Suit, TextSpan};

/// Parse commentary text from PBN, handling formatting codes
/// Commentary is enclosed in braces: { ... }
/// Supports: <b>bold</b>, <i>italic</i>, \S \H \D \C for suits
pub fn parse_commentary(input: &str) -> Result<CommentaryBlock, String> {
    let content = parse_formatted_text(input)?;
    Ok(CommentaryBlock::new(content))
}

/// Parse formatted text with HTML-like tags and suit symbols
pub fn parse_formatted_text(input: &str) -> Result<FormattedText, String> {
    let mut text = FormattedText::new();
    let mut remaining = input;
    let mut plain_buffer = String::new();

    while !remaining.is_empty() {
        if remaining.starts_with("<b>") {
            // Flush plain buffer
            if !plain_buffer.is_empty() {
                text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
            }

            // Find closing tag
            let end = remaining.find("</b>").ok_or("Unclosed <b> tag")?;
            let bold_content = &remaining[3..end];
            text.push(TextSpan::bold(bold_content));
            remaining = &remaining[end + 4..];
        } else if remaining.starts_with("<i>") {
            // Flush plain buffer
            if !plain_buffer.is_empty() {
                text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
            }

            // Find closing tag
            let end = remaining.find("</i>").ok_or("Unclosed <i> tag")?;
            let italic_content = &remaining[3..end];
            text.push(TextSpan::italic(italic_content));
            remaining = &remaining[end + 4..];
        } else if remaining.starts_with('\\') && remaining.len() >= 2 {
            // Check for suit symbol escape
            let next_char = remaining.chars().nth(1).unwrap();

            match next_char {
                'S' | 's' => {
                    // Flush plain buffer
                    if !plain_buffer.is_empty() {
                        text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
                    }

                    // Check if followed by a rank (card reference)
                    if remaining.len() >= 3 {
                        let rank_char = remaining.chars().nth(2).unwrap();
                        if let Some(rank) = Rank::from_pbn_char(rank_char) {
                            text.push(TextSpan::CardRef {
                                suit: Suit::Spades,
                                rank,
                            });
                            remaining = &remaining[3..];
                            continue;
                        }
                    }
                    text.push(TextSpan::SuitSymbol(Suit::Spades));
                    remaining = &remaining[2..];
                }
                'H' | 'h' => {
                    if !plain_buffer.is_empty() {
                        text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
                    }

                    if remaining.len() >= 3 {
                        let rank_char = remaining.chars().nth(2).unwrap();
                        if let Some(rank) = Rank::from_pbn_char(rank_char) {
                            text.push(TextSpan::CardRef {
                                suit: Suit::Hearts,
                                rank,
                            });
                            remaining = &remaining[3..];
                            continue;
                        }
                    }
                    text.push(TextSpan::SuitSymbol(Suit::Hearts));
                    remaining = &remaining[2..];
                }
                'D' | 'd' => {
                    if !plain_buffer.is_empty() {
                        text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
                    }

                    if remaining.len() >= 3 {
                        let rank_char = remaining.chars().nth(2).unwrap();
                        if let Some(rank) = Rank::from_pbn_char(rank_char) {
                            text.push(TextSpan::CardRef {
                                suit: Suit::Diamonds,
                                rank,
                            });
                            remaining = &remaining[3..];
                            continue;
                        }
                    }
                    text.push(TextSpan::SuitSymbol(Suit::Diamonds));
                    remaining = &remaining[2..];
                }
                'C' | 'c' => {
                    if !plain_buffer.is_empty() {
                        text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
                    }

                    if remaining.len() >= 3 {
                        let rank_char = remaining.chars().nth(2).unwrap();
                        if let Some(rank) = Rank::from_pbn_char(rank_char) {
                            text.push(TextSpan::CardRef {
                                suit: Suit::Clubs,
                                rank,
                            });
                            remaining = &remaining[3..];
                            continue;
                        }
                    }
                    text.push(TextSpan::SuitSymbol(Suit::Clubs));
                    remaining = &remaining[2..];
                }
                'n' => {
                    // Newline
                    if !plain_buffer.is_empty() {
                        text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
                    }
                    text.push(TextSpan::LineBreak);
                    remaining = &remaining[2..];
                }
                _ => {
                    // Unknown escape, keep as-is
                    plain_buffer.push('\\');
                    plain_buffer.push(next_char);
                    remaining = &remaining[2..];
                }
            }
        } else if remaining.starts_with("\n\n") || remaining.starts_with("\r\n\r\n") {
            // Blank line = paragraph break (double line break for spacing)
            if !plain_buffer.is_empty() {
                text.push(TextSpan::plain(std::mem::take(&mut plain_buffer)));
            }
            text.push(TextSpan::LineBreak);
            text.push(TextSpan::LineBreak);
            if remaining.starts_with("\r\n\r\n") {
                remaining = &remaining[4..];
            } else {
                remaining = &remaining[2..];
            }
        } else if remaining.starts_with('\n') || remaining.starts_with("\r\n") {
            // Single newline = soft line break (treat as space for word wrapping)
            if !plain_buffer.is_empty() && !plain_buffer.ends_with(' ') {
                plain_buffer.push(' ');
            }
            if remaining.starts_with("\r\n") {
                remaining = &remaining[2..];
            } else {
                remaining = &remaining[1..];
            }
        } else {
            // Regular character
            let c = remaining.chars().next().unwrap();
            plain_buffer.push(c);
            remaining = &remaining[c.len_utf8()..];
        }
    }

    // Flush remaining plain buffer
    if !plain_buffer.is_empty() {
        text.push(TextSpan::plain(plain_buffer));
    }

    Ok(text)
}

/// Extract commentary block from braces
pub fn extract_commentary(input: &str) -> Option<(&str, &str)> {
    let start = input.find('{')?;
    let end = input.rfind('}')?;

    if end > start {
        Some((&input[start + 1..end], &input[end + 1..]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let text = parse_formatted_text("Hello world").unwrap();
        assert_eq!(text.spans.len(), 1);
        assert_eq!(text.spans[0], TextSpan::Plain("Hello world".to_string()));
    }

    #[test]
    fn test_bold_text() {
        let text = parse_formatted_text("<b>Bold</b> text").unwrap();
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0], TextSpan::Bold("Bold".to_string()));
        assert_eq!(text.spans[1], TextSpan::Plain(" text".to_string()));
    }

    #[test]
    fn test_suit_symbols() {
        let text = parse_formatted_text(r"Open 1\S").unwrap();
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0], TextSpan::Plain("Open 1".to_string()));
        assert_eq!(text.spans[1], TextSpan::SuitSymbol(Suit::Spades));
    }

    #[test]
    fn test_card_reference() {
        let text = parse_formatted_text(r"Lead the \SQ").unwrap();
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0], TextSpan::Plain("Lead the ".to_string()));
        assert_eq!(
            text.spans[1],
            TextSpan::CardRef {
                suit: Suit::Spades,
                rank: Rank::Queen
            }
        );
    }

    #[test]
    fn test_mixed_formatting() {
        let text = parse_formatted_text(r"<b>Bidding.</b> Open 1\D and rebid 2\H").unwrap();
        assert_eq!(text.to_plain_text(), "Bidding. Open 1♦ and rebid 2♥");
    }

    #[test]
    fn test_extract_commentary() {
        let input = "{This is commentary} rest of line";
        let (commentary, rest) = extract_commentary(input).unwrap();
        assert_eq!(commentary, "This is commentary");
        assert_eq!(rest, " rest of line");
    }

    #[test]
    fn test_bold_with_spaces() {
        // Test "Declarer play." in bold followed by plain text
        let text = parse_formatted_text("<b>Declarer play.</b> Declarer (North)").unwrap();
        println!("Spans:");
        for (i, span) in text.spans.iter().enumerate() {
            println!("  {}: {:?}", i, span);
        }
        assert_eq!(text.spans.len(), 2);
        assert_eq!(text.spans[0], TextSpan::Bold("Declarer play.".to_string()));
        assert_eq!(text.spans[1], TextSpan::Plain(" Declarer (North)".to_string()));
    }
}
