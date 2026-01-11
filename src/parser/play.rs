use crate::model::{Card, Direction, PlaySequence, Rank, Suit, Trick};

/// Parse a play section from PBN
/// The play starts after [Play "X"] where X is the opening leader
/// Cards are in format SR where S is suit (S/H/D/C) and R is rank
pub fn parse_play(opening_leader: Direction, input: &str) -> Result<PlaySequence, String> {
    let mut play = PlaySequence::new(opening_leader);
    let input = input.trim();

    if input.is_empty() {
        return Ok(play);
    }

    // Split on whitespace
    let tokens: Vec<&str> = input.split_whitespace().collect();

    let mut current_trick = Trick::new(opening_leader);
    let mut card_index = 0;

    for token in tokens {
        // Skip empty tokens
        if token.is_empty() {
            continue;
        }

        // Handle end markers
        if token == "*" {
            break;
        }

        // Handle dashes (card not played / unknown)
        if token == "-" {
            card_index += 1;
            if card_index >= 4 {
                play.add_trick(current_trick);
                current_trick = Trick::new(opening_leader); // TODO: determine next leader
                card_index = 0;
            }
            continue;
        }

        // Parse card
        if let Some(card) = parse_card(token) {
            current_trick.set_card(card_index, card);
            card_index += 1;

            if card_index >= 4 {
                play.add_trick(current_trick.clone());
                current_trick = Trick::new(opening_leader); // TODO: determine next leader based on winner
                card_index = 0;
            }
        } else {
            log::debug!("Skipping unrecognized play token: {}", token);
        }
    }

    // Add incomplete trick if any cards were played
    if card_index > 0 {
        play.add_trick(current_trick);
    }

    Ok(play)
}

/// Parse a card notation: "SQ" = Queen of Spades, "HA" = Ace of Hearts
fn parse_card(input: &str) -> Option<Card> {
    let mut chars = input.chars();

    let suit_char = chars.next()?;
    let rank_char = chars.next()?;

    let suit = Suit::from_char(suit_char)?;
    let rank = Rank::from_pbn_char(rank_char)?;

    Some(Card::new(suit, rank))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_card() {
        let card = parse_card("SQ").unwrap();
        assert_eq!(card.suit, Suit::Spades);
        assert_eq!(card.rank, Rank::Queen);

        let card = parse_card("HA").unwrap();
        assert_eq!(card.suit, Suit::Hearts);
        assert_eq!(card.rank, Rank::Ace);

        let card = parse_card("DT").unwrap();
        assert_eq!(card.suit, Suit::Diamonds);
        assert_eq!(card.rank, Rank::Ten);
    }

    #[test]
    fn test_parse_play_single_trick() {
        let play = parse_play(Direction::West, "SQ SK S8 S4").unwrap();

        assert_eq!(play.tricks.len(), 1);
        assert!(play.tricks[0].is_complete());

        let trick = &play.tricks[0];
        assert_eq!(trick.cards[0].unwrap().rank, Rank::Queen);
        assert_eq!(trick.cards[1].unwrap().rank, Rank::King);
        assert_eq!(trick.cards[2].unwrap().rank, Rank::Eight);
        assert_eq!(trick.cards[3].unwrap().rank, Rank::Four);
    }

    #[test]
    fn test_parse_incomplete_play() {
        let play = parse_play(Direction::East, "SQ *").unwrap();

        assert_eq!(play.tricks.len(), 1);
        assert!(!play.tricks[0].is_complete());
    }

    #[test]
    fn test_parse_empty_play() {
        let play = parse_play(Direction::North, "").unwrap();
        assert_eq!(play.tricks.len(), 0);
    }
}
