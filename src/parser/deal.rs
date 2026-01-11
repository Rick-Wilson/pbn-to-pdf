use crate::model::{Deal, Direction, Hand, Holding, Rank};

/// Parse a deal notation string: "N:AKQ.JT9.876.5432 QJ.AK.QT9.87654 ..."
pub fn parse_deal(input: &str) -> Result<Deal, String> {
    let input = input.trim();

    // Parse first direction
    let mut chars = input.chars();
    let first_dir_char = chars.next().ok_or("Empty deal string")?;
    let first_direction = Direction::from_char(first_dir_char)
        .ok_or_else(|| format!("Invalid direction: {}", first_dir_char))?;

    // Expect colon
    if chars.next() != Some(':') {
        return Err("Expected ':' after direction".to_string());
    }

    // Get the rest of the string (hand notations)
    let hands_str: String = chars.collect();
    let hand_notations: Vec<&str> = hands_str.split_whitespace().collect();

    if hand_notations.len() != 4 {
        return Err(format!(
            "Expected 4 hands, got {}",
            hand_notations.len()
        ));
    }

    // Parse each hand
    let mut hands = Vec::with_capacity(4);
    for notation in hand_notations {
        hands.push(parse_hand(notation)?);
    }

    // Assign hands to directions based on first_direction
    // The order is: first_direction, next, next, next
    let mut deal = Deal::new();
    let mut dir = first_direction;
    for hand in hands {
        deal.set_hand(dir, hand);
        dir = dir.next();
    }

    Ok(deal)
}

/// Parse a single hand notation: "AKQ.JT9.876.5432"
fn parse_hand(input: &str) -> Result<Hand, String> {
    let suits: Vec<&str> = input.split('.').collect();

    if suits.len() != 4 {
        return Err(format!(
            "Expected 4 suits separated by '.', got {} in '{}'",
            suits.len(),
            input
        ));
    }

    let spades = parse_holding(suits[0])?;
    let hearts = parse_holding(suits[1])?;
    let diamonds = parse_holding(suits[2])?;
    let clubs = parse_holding(suits[3])?;

    Ok(Hand::from_holdings(spades, hearts, diamonds, clubs))
}

/// Parse a holding notation: "AKQ" or "" (void)
fn parse_holding(input: &str) -> Result<Holding, String> {
    let mut holding = Holding::new();

    for c in input.chars() {
        // Skip dashes used to indicate void
        if c == '-' {
            continue;
        }

        let rank = Rank::from_pbn_char(c)
            .ok_or_else(|| format!("Invalid rank character: {}", c))?;
        holding.add(rank);
    }

    Ok(holding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Rank;

    #[test]
    fn test_parse_holding() {
        let holding = parse_holding("AKQ").unwrap();
        assert_eq!(holding.len(), 3);
        assert!(holding.ranks.contains(&Rank::Ace));
        assert!(holding.ranks.contains(&Rank::King));
        assert!(holding.ranks.contains(&Rank::Queen));
    }

    #[test]
    fn test_parse_void_holding() {
        let holding = parse_holding("").unwrap();
        assert!(holding.is_void());

        let holding = parse_holding("-").unwrap();
        assert!(holding.is_void());
    }

    #[test]
    fn test_parse_hand() {
        let hand = parse_hand("AKQ.JT9.876.5432").unwrap();
        assert_eq!(hand.spades.len(), 3);
        assert_eq!(hand.hearts.len(), 3);
        assert_eq!(hand.diamonds.len(), 3);
        assert_eq!(hand.clubs.len(), 4);
        assert_eq!(hand.card_count(), 13);
    }

    #[test]
    fn test_parse_deal() {
        let input = "N:A65.J4.A764.A983 QJT73.9852.K3.Q7 K82.KQT3.T52.642 94.A76.QJ98.KJT5";
        let deal = parse_deal(input).unwrap();

        // North is first
        assert_eq!(deal.north.spades.len(), 3); // A65
        assert_eq!(deal.north.hearts.len(), 2); // J4
        assert_eq!(deal.north.total_hcp(), 13);

        // East is second
        assert_eq!(deal.east.spades.len(), 5); // QJT73

        // South is third
        assert_eq!(deal.south.spades.len(), 3); // K82

        // West is fourth
        assert_eq!(deal.west.spades.len(), 2); // 94
    }

    #[test]
    fn test_parse_deal_with_void() {
        let input = "S:AKQJT98765432...- -...AKQJT98765432 -.AKQJT98765432..- -..AKQJT98765432.";
        let deal = parse_deal(input).unwrap();

        // South has all spades
        assert_eq!(deal.south.spades.len(), 13);
        assert!(deal.south.hearts.is_void());
        assert!(deal.south.diamonds.is_void());
        assert!(deal.south.clubs.is_void());
    }
}
