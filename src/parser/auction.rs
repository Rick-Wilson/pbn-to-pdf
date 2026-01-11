use crate::model::{Auction, Call, Direction};

/// Parse an auction section from PBN
/// The auction starts after [Auction "X"] where X is the dealer
/// Calls are separated by whitespace
pub fn parse_auction(dealer: Direction, input: &str) -> Result<Auction, String> {
    let mut auction = Auction::new(dealer);
    let input = input.trim();

    if input.is_empty() {
        return Ok(auction);
    }

    // Split on whitespace
    let tokens: Vec<&str> = input.split_whitespace().collect();

    for token in tokens {
        // Skip empty tokens
        if token.is_empty() {
            continue;
        }

        // Handle special tokens
        match token.to_uppercase().as_str() {
            "AP" => {
                // All Pass - add 3 passes to end the auction
                auction.add_call(Call::Pass);
                auction.add_call(Call::Pass);
                auction.add_call(Call::Pass);
                auction.is_passed_out = false;
                break;
            }
            "*" => {
                // End of auction marker (incomplete auction)
                break;
            }
            _ => {
                // Try to parse as a call
                // Handle annotations like "1C!" or "1C$1"
                let clean_token = strip_annotations(token);

                if let Some(call) = Call::from_pbn(&clean_token) {
                    auction.add_call(call);
                } else {
                    // Skip unrecognized tokens (might be annotations)
                    log::debug!("Skipping unrecognized auction token: {}", token);
                }
            }
        }
    }

    // Check if auction passed out (4 passes at start)
    if auction.calls.len() >= 4
        && auction.calls[0].call == Call::Pass
        && auction.calls[1].call == Call::Pass
        && auction.calls[2].call == Call::Pass
        && auction.calls[3].call == Call::Pass
    {
        auction.is_passed_out = true;
    }

    Ok(auction)
}

/// Strip annotations from a call token
/// e.g., "1C!" -> "1C", "2H$1" -> "2H"
fn strip_annotations(token: &str) -> String {
    let mut result = String::new();
    for c in token.chars() {
        match c {
            '!' | '?' | '$' => break,       // NAG markers
            '{' | '}' | '(' | ')' => break, // Comment markers
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BidSuit;

    #[test]
    fn test_simple_auction() {
        let auction = parse_auction(Direction::North, "1D 1S X Pass 1NT AP").unwrap();

        assert_eq!(auction.dealer, Direction::North);
        assert_eq!(auction.calls.len(), 8); // 5 calls + 3 passes from AP

        // Check first few calls
        assert_eq!(
            auction.calls[0].call,
            Call::Bid {
                level: 1,
                suit: BidSuit::Diamonds
            }
        );
        assert_eq!(
            auction.calls[1].call,
            Call::Bid {
                level: 1,
                suit: BidSuit::Spades
            }
        );
        assert_eq!(auction.calls[2].call, Call::Double);
        assert_eq!(auction.calls[3].call, Call::Pass);
    }

    #[test]
    fn test_passed_out() {
        let auction = parse_auction(Direction::South, "Pass Pass Pass Pass").unwrap();
        assert!(auction.is_passed_out);
        assert_eq!(auction.calls.len(), 4);
    }

    #[test]
    fn test_auction_with_annotations() {
        let auction = parse_auction(Direction::West, "1C! 1H 2C$1 Pass").unwrap();
        assert_eq!(auction.calls.len(), 4);
        assert_eq!(
            auction.calls[0].call,
            Call::Bid {
                level: 1,
                suit: BidSuit::Clubs
            }
        );
    }

    #[test]
    fn test_empty_auction() {
        let auction = parse_auction(Direction::East, "").unwrap();
        assert_eq!(auction.calls.len(), 0);
    }

    #[test]
    fn test_final_contract() {
        // N bids 1NT, E passes, S bids 3NT, so S is declarer
        let auction = parse_auction(Direction::North, "1NT Pass 3NT AP").unwrap();
        let contract = auction.final_contract().unwrap();

        assert_eq!(contract.level, 3);
        assert_eq!(contract.suit, BidSuit::NoTrump);
        assert!(!contract.doubled);
        assert!(!contract.redoubled);
        assert_eq!(contract.declarer, Direction::South);
    }

    #[test]
    fn test_doubled_contract() {
        let auction = parse_auction(Direction::South, "1S X XX Pass Pass Pass").unwrap();
        let contract = auction.final_contract().unwrap();

        assert_eq!(contract.level, 1);
        assert_eq!(contract.suit, BidSuit::Spades);
        assert!(!contract.doubled);
        assert!(contract.redoubled);
    }
}
