use nom::{
    bytes::complete::take_while1,
    character::complete::{char, space0},
    IResult,
};

/// A PBN tag pair: [Name "Value"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPair {
    pub name: String,
    pub value: String,
}

impl TagPair {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

fn is_tag_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Parse a tag name (alphanumeric + underscore)
fn tag_name(input: &str) -> IResult<&str, &str> {
    take_while1(is_tag_name_char)(input)
}

/// Parse a quoted string value, handling escape sequences
fn quoted_value(input: &str) -> IResult<&str, String> {
    let (input, _) = char('"')(input)?;
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut consumed = 0;

    while let Some(c) = chars.next() {
        consumed += c.len_utf8();
        match c {
            '"' => {
                // End of string
                return Ok((&input[consumed..], result));
            }
            '\\' => {
                // Escape sequence
                if let Some(next) = chars.next() {
                    consumed += next.len_utf8();
                    match next {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        _ => {
                            // Keep backslash for unknown escapes (like \S for suit symbols)
                            result.push('\\');
                            result.push(next);
                        }
                    }
                }
            }
            _ => result.push(c),
        }
    }

    // If we get here, the string was never closed
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Char,
    )))
}

/// Parse a complete tag pair: [Name "Value"]
pub fn parse_tag_pair(input: &str) -> IResult<&str, TagPair> {
    let (input, _) = char('[')(input)?;
    let (input, name) = tag_name(input)?;
    let (input, _) = space0(input)?;
    let (input, value) = quoted_value(input)?;
    let (input, _) = char(']')(input)?;

    Ok((
        input,
        TagPair {
            name: name.to_string(),
            value,
        },
    ))
}

/// Parse multiple tag pairs from a line or block
pub fn parse_tag_pairs(input: &str) -> Vec<TagPair> {
    let mut pairs = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        // Skip whitespace and newlines
        remaining = remaining.trim_start();

        if remaining.starts_with('[') {
            match parse_tag_pair(remaining) {
                Ok((rest, pair)) => {
                    pairs.push(pair);
                    remaining = rest;
                }
                Err(_) => break,
            }
        } else {
            break;
        }
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tag_pair() {
        let input = r#"[Event "Test Event"]"#;
        let (remaining, pair) = parse_tag_pair(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(pair.name, "Event");
        assert_eq!(pair.value, "Test Event");
    }

    #[test]
    fn test_empty_value() {
        let input = r#"[Site ""]"#;
        let (_, pair) = parse_tag_pair(input).unwrap();
        assert_eq!(pair.value, "");
    }

    #[test]
    fn test_escaped_quotes() {
        let input = r#"[Comment "He said \"hello\""]"#;
        let (_, pair) = parse_tag_pair(input).unwrap();
        assert_eq!(pair.value, r#"He said "hello""#);
    }

    #[test]
    fn test_suit_escape_preserved() {
        let input = r#"[Note "Open 1\S"]"#;
        let (_, pair) = parse_tag_pair(input).unwrap();
        assert_eq!(pair.value, r"Open 1\S");
    }

    #[test]
    fn test_multiple_tag_pairs() {
        let input = r#"[Event "Test"][Site "Location"][Date "2024.01.01"]"#;
        let pairs = parse_tag_pairs(input);
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0].name, "Event");
        assert_eq!(pairs[1].name, "Site");
        assert_eq!(pairs[2].name, "Date");
    }
}
