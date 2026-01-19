use crate::error::PbnError;
use crate::model::{Board, Direction, PbnMetadata, Vulnerability};

use super::auction::parse_auction;
use super::commentary::{extract_commentary, parse_commentary};
use super::deal::parse_deal;
use super::header::parse_headers;
use super::play::parse_play;
use super::tags::{parse_tag_pair, TagPair};

/// Parse a note value in format "N:text" where N is the note number
/// Returns (note_number, note_text) if successful
fn parse_note_value(value: &str) -> Option<(u8, String)> {
    let colon_pos = value.find(':')?;
    let num_str = &value[..colon_pos];
    let text = &value[colon_pos + 1..];
    let num = num_str.parse::<u8>().ok()?;
    Some((num, text.to_string()))
}

/// Result of parsing a PBN file
#[derive(Debug)]
pub struct PbnFile {
    pub metadata: PbnMetadata,
    pub boards: Vec<Board>,
}

/// Parse a complete PBN file
pub fn parse_pbn(content: &str) -> Result<PbnFile, PbnError> {
    let lines: Vec<&str> = content.lines().collect();

    // Extract header lines (starting with %)
    let header_lines: Vec<&str> = lines
        .iter()
        .filter(|line| line.trim().starts_with('%'))
        .copied()
        .collect();

    let metadata = parse_headers(&header_lines);

    // Parse boards
    let boards = parse_boards(&lines)?;

    Ok(PbnFile { metadata, boards })
}

/// Parse all board records from the file
fn parse_boards(lines: &[&str]) -> Result<Vec<Board>, PbnError> {
    let mut boards = Vec::new();
    let mut current_board: Option<Board> = None;
    let mut in_auction = false;
    let mut auction_dealer: Option<Direction> = None;
    let mut auction_lines = Vec::new();
    let mut in_play = false;
    let mut play_leader: Option<Direction> = None;
    let mut play_lines = Vec::new();
    let mut in_commentary = false;
    let mut commentary_lines: Vec<&str> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments (but not if we're in commentary)
        if !in_commentary
            && (trimmed.is_empty() || trimmed.starts_with('%') || trimmed.starts_with(';'))
        {
            continue;
        }

        // Handle multi-line commentary
        if in_commentary {
            commentary_lines.push(*line);
            if line.contains('}') {
                // End of commentary block
                in_commentary = false;
                if let Some(ref mut board) = current_board {
                    let full_text = commentary_lines.join("\n");
                    if let Some((commentary_text, _)) = extract_commentary(&full_text) {
                        if let Ok(block) = parse_commentary(commentary_text) {
                            board.commentary.push(block);
                        }
                    }
                }
                commentary_lines.clear();
            }
            continue;
        }

        // Check for tag pairs
        if trimmed.starts_with('[') {
            // Finish any ongoing auction section
            if in_auction && !auction_lines.is_empty() {
                if let (Some(ref mut board), Some(dealer)) = (&mut current_board, auction_dealer) {
                    let auction_text = auction_lines.join(" ");
                    if let Ok(auction) = parse_auction(dealer, &auction_text) {
                        board.auction = Some(auction);
                    }
                }
                auction_lines.clear();
                in_auction = false;
            }

            // Finish any ongoing play section
            if in_play && !play_lines.is_empty() {
                if let (Some(ref mut board), Some(leader)) = (&mut current_board, play_leader) {
                    let play_text = play_lines.join(" ");
                    if let Ok(play) = parse_play(leader, &play_text) {
                        board.play = Some(play);
                    }
                }
                play_lines.clear();
                in_play = false;
            }

            // Parse the tag pair
            if let Ok((_, tag)) = parse_tag_pair(trimmed) {
                process_tag(
                    &mut current_board,
                    &mut boards,
                    tag,
                    &mut in_auction,
                    &mut auction_dealer,
                    &mut in_play,
                    &mut play_leader,
                )?;
            }
        } else if trimmed.starts_with('{') {
            // Start of commentary block
            commentary_lines.push(*line);
            if line.contains('}') {
                // Single-line commentary
                if let Some(ref mut board) = current_board {
                    if let Some((commentary_text, _)) = extract_commentary(line) {
                        if let Ok(block) = parse_commentary(commentary_text) {
                            board.commentary.push(block);
                        }
                    }
                }
                commentary_lines.clear();
            } else {
                // Multi-line commentary
                in_commentary = true;
            }
        } else if in_auction {
            // Continuation of auction section
            auction_lines.push(trimmed);
        } else if in_play {
            // Continuation of play section
            play_lines.push(trimmed);
        }
    }

    // Finish any final auction section
    if in_auction && !auction_lines.is_empty() {
        if let (Some(ref mut board), Some(dealer)) = (&mut current_board, auction_dealer) {
            let auction_text = auction_lines.join(" ");
            if let Ok(auction) = parse_auction(dealer, &auction_text) {
                board.auction = Some(auction);
            }
        }
    }

    // Finish any final play section
    if in_play && !play_lines.is_empty() {
        if let (Some(ref mut board), Some(leader)) = (&mut current_board, play_leader) {
            let play_text = play_lines.join(" ");
            if let Ok(play) = parse_play(leader, &play_text) {
                board.play = Some(play);
            }
        }
    }

    // Save the last board
    if let Some(board) = current_board {
        boards.push(board);
    }

    Ok(boards)
}

/// Process a single tag pair
fn process_tag(
    current_board: &mut Option<Board>,
    boards: &mut Vec<Board>,
    tag: TagPair,
    in_auction: &mut bool,
    auction_dealer: &mut Option<Direction>,
    in_play: &mut bool,
    play_leader: &mut Option<Direction>,
) -> Result<(), PbnError> {
    match tag.name.as_str() {
        "Event" => {
            // Start of a new board
            if let Some(board) = current_board.take() {
                boards.push(board);
            }
            let mut board = Board::new();
            if !tag.value.is_empty() {
                board.event = Some(tag.value);
            }
            *current_board = Some(board);
        }
        "Site" => {
            if let Some(ref mut board) = current_board {
                if !tag.value.is_empty() {
                    board.site = Some(tag.value);
                }
            }
        }
        "Date" => {
            if let Some(ref mut board) = current_board {
                if !tag.value.is_empty() {
                    board.date = Some(tag.value);
                }
            }
        }
        "Board" => {
            if let Some(ref mut board) = current_board {
                if let Ok(num) = tag.value.parse::<u32>() {
                    board.number = Some(num);
                }
            }
        }
        "Dealer" => {
            if let Some(ref mut board) = current_board {
                if let Some(dir) = tag.value.chars().next().and_then(Direction::from_char) {
                    board.dealer = Some(dir);
                }
            }
        }
        "Vulnerable" => {
            if let Some(ref mut board) = current_board {
                if let Some(vuln) = Vulnerability::from_pbn(&tag.value) {
                    board.vulnerable = vuln;
                }
            }
        }
        "Deal" => {
            if let Some(ref mut board) = current_board {
                match parse_deal(&tag.value) {
                    Ok(deal) => board.deal = deal,
                    Err(e) => {
                        log::warn!("Failed to parse deal: {}", e);
                    }
                }
            }
        }
        "Declarer" => {
            if let Some(ref mut board) = current_board {
                if let Some(dir) = tag.value.chars().next().and_then(Direction::from_char) {
                    board.declarer = Some(dir);
                }
            }
        }
        "Contract" => {
            // Contract is often derived from auction, but can be explicit
            // We'll handle this in a simplified way for now
        }
        "Result" => {
            if let Some(ref mut board) = current_board {
                if let Ok(result) = tag.value.parse::<i8>() {
                    board.result = Some(result);
                }
            }
        }
        "Auction" => {
            *in_auction = true;
            *in_play = false;
            if let Some(dir) = tag.value.chars().next().and_then(Direction::from_char) {
                *auction_dealer = Some(dir);
            }
        }
        "Play" => {
            *in_play = true;
            *in_auction = false;
            if let Some(dir) = tag.value.chars().next().and_then(Direction::from_char) {
                *play_leader = Some(dir);
            }
        }
        "Note" => {
            // Parse note in format "N:text" where N is the note number
            if let Some(ref mut board) = current_board {
                if let Some((num, text)) = parse_note_value(&tag.value) {
                    if let Some(ref mut auction) = board.auction {
                        auction.add_note(num, text);
                    }
                }
            }
        }
        _ => {
            // Unknown tag, skip
            log::debug!("Skipping unknown tag: {}", tag.name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pbn() {
        let content = r#"% PBN 2.1
[Event "Test"]
[Site ""]
[Date "2024.01.01"]
[Board "1"]
[Dealer "N"]
[Vulnerable "None"]
[Deal "N:AKQ.JT9.876.5432 JT9.AKQ.543.8765 876.543.AKQ.JT98 543.876.JT9.AKQ6"]
[Auction "N"]
1NT Pass 3NT AP
"#;

        let result = parse_pbn(content).unwrap();
        assert_eq!(result.boards.len(), 1);

        let board = &result.boards[0];
        assert_eq!(board.number, Some(1));
        assert_eq!(board.dealer, Some(Direction::North));
        assert_eq!(board.vulnerable, Vulnerability::None);
        assert_eq!(board.deal.north.spades.len(), 3); // AKQ
    }

    #[test]
    fn test_parse_multiple_boards() {
        let content = r#"[Event "Test1"]
[Board "1"]
[Dealer "N"]
[Vulnerable "None"]
[Deal "N:AKQ.JT9.876.5432 JT9.AKQ.543.8765 876.543.AKQ.JT98 543.876.JT9.AKQ6"]

[Event "Test2"]
[Board "2"]
[Dealer "E"]
[Vulnerable "NS"]
[Deal "N:AKQ.JT9.876.5432 JT9.AKQ.543.8765 876.543.AKQ.JT98 543.876.JT9.AKQ6"]
"#;

        let result = parse_pbn(content).unwrap();
        assert_eq!(result.boards.len(), 2);
        assert_eq!(result.boards[0].number, Some(1));
        assert_eq!(result.boards[1].number, Some(2));
        assert_eq!(result.boards[1].vulnerable, Vulnerability::NorthSouth);
    }
}
