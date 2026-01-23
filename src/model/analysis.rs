//! Card pattern analysis for declarer play
//!
//! Provides functions to identify card patterns useful for declarer play planning,
//! such as sure winners (cards that can win tricks without losing the lead).

use super::card::{Card, Rank, Suit};
use super::hand::Hand;

/// Find all sure winners in a NT contract by combining dummy and declarer hands.
///
/// Sure winners are cards that can win tricks without giving up the lead.
/// This includes sequences starting from the Ace: A, AK, AKQ, AKQJ, etc.
///
/// The analysis combines both hands to find the continuous sequence from Ace,
/// but respects trick rules: the number of sure winner tricks in a suit is
/// limited by the longer holding between the two hands. When you play a card
/// from one hand, you must play a card from the other hand on the same trick.
///
/// For example, if dummy has AKQ and declarer has JT9:
/// - The combined sequence is AKQJT9 (6 cards continuous from Ace)
/// - But max(3, 3) = 3 tricks can be taken
/// - The sure winners are AKQ (the top 3 cards that actually win tricks)
/// - The JT9 are played on those same tricks but don't win
///
/// # Arguments
/// * `dummy` - The dummy's hand (typically North)
/// * `declarer` - The declarer's hand (typically South)
///
/// # Returns
/// A vector of Cards that are sure winners (cards that actually win tricks).
///
/// # Example
/// ```
/// use pbn_to_pdf::model::{Hand, Holding, Rank, Suit};
/// use pbn_to_pdf::model::analysis::find_sure_winners;
///
/// let mut dummy = Hand::new();
/// dummy.spades = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);
///
/// let mut declarer = Hand::new();
/// declarer.spades = Holding::from_ranks([Rank::Jack, Rank::Ten, Rank::Nine]);
///
/// let winners = find_sure_winners(&dummy, &declarer);
/// // Only 3 sure winners (AKQ), not 6, because you can only take 3 tricks
/// assert_eq!(winners.len(), 3);
/// ```
pub fn find_sure_winners(dummy: &Hand, declarer: &Hand) -> Vec<Card> {
    let mut winners = Vec::new();

    for suit in Suit::all() {
        let suit_winners = find_sure_winners_in_suit(dummy, declarer, suit);
        winners.extend(suit_winners);
    }

    winners
}

/// Find sure winners in a single suit by combining both hands.
///
/// Returns the top cards from the continuous sequence starting from Ace,
/// limited by the number of tricks that can be taken (max of the two hand lengths).
fn find_sure_winners_in_suit(dummy: &Hand, declarer: &Hand, suit: Suit) -> Vec<Card> {
    let dummy_holding = dummy.holding(suit);
    let declarer_holding = declarer.holding(suit);

    // The maximum number of tricks we can take in this suit is limited by
    // the longer holding - you must play from both hands on each trick
    let max_tricks = dummy_holding.len().max(declarer_holding.len());

    if max_tricks == 0 {
        return Vec::new();
    }

    // Combine ranks from both hands
    let mut combined_ranks: Vec<Rank> = dummy_holding
        .ranks
        .iter()
        .chain(declarer_holding.ranks.iter())
        .copied()
        .collect();

    // Sort by rank (Ace highest, so it comes first in BTreeSet order which is already correct)
    // Rank ordering: Ace > King > Queen > Jack > Ten > ... > Two
    combined_ranks.sort();

    if combined_ranks.is_empty() {
        return Vec::new();
    }

    // Must start with Ace to have any sure winners
    if combined_ranks.first() != Some(&Rank::Ace) {
        return Vec::new();
    }

    // Find the continuous sequence starting from Ace
    let mut sequence = Vec::new();
    let expected_sequence = Rank::all();
    let mut combined_iter = combined_ranks.iter().peekable();

    for expected_rank in expected_sequence {
        if combined_iter.peek() == Some(&&expected_rank) {
            let rank = *combined_iter.next().unwrap();
            sequence.push(rank);
        } else {
            // Gap in sequence - stop here
            break;
        }
    }

    // The number of sure winner tricks is the minimum of:
    // 1. The length of the continuous sequence from Ace
    // 2. The maximum tricks we can take (longer hand length)
    let num_winners = sequence.len().min(max_tricks);

    // Return the top N cards as sure winners
    sequence
        .into_iter()
        .take(num_winners)
        .map(|rank| Card::new(suit, rank))
        .collect()
}

/// Result of promotion analysis for a suit or hand.
#[derive(Debug, Clone, Default)]
pub struct PromotionResult {
    /// Cards that are spent (used to drive out higher honors)
    pub spent: Vec<Card>,
    /// Cards that become winners after promotion
    pub winners: Vec<Card>,
}

impl PromotionResult {
    /// Create a new empty promotion result
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another promotion result into this one
    pub fn merge(&mut self, other: PromotionResult) {
        self.spent.extend(other.spent);
        self.winners.extend(other.winners);
    }

    /// Returns true if there are any promotable cards
    pub fn has_promotion(&self) -> bool {
        !self.winners.is_empty()
    }

    /// Total number of cards involved in promotion (spent + winners)
    pub fn total_cards(&self) -> usize {
        self.spent.len() + self.winners.len()
    }
}

/// Result of length winner analysis for a suit or hand.
#[derive(Debug, Clone, Default)]
pub struct LengthResult {
    /// Cards used to duck (give up tricks to exhaust defenders)
    pub ducks: Vec<Card>,
    /// Cards that become winners through length (after defenders are exhausted)
    pub winners: Vec<Card>,
}

impl LengthResult {
    /// Create a new empty length result
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another length result into this one
    pub fn merge(&mut self, other: LengthResult) {
        self.ducks.extend(other.ducks);
        self.winners.extend(other.winners);
    }

    /// Returns true if there are any length winners
    pub fn has_length_winners(&self) -> bool {
        !self.winners.is_empty()
    }

    /// Total number of cards involved (ducks + winners)
    pub fn total_cards(&self) -> usize {
        self.ducks.len() + self.winners.len()
    }
}

/// Find all potential length winners by combining dummy and declarer hands.
///
/// Length winners are cards that can become winners after exhausting the defenders'
/// cards in a suit. This requires a combined holding of 7+ cards, giving a chance
/// that defenders will run out of that suit.
///
/// The analysis assumes optimal defense distribution (cards divide as evenly as possible):
/// - 7 cards (6 out): defenders have 3-3 split = 1 length winner
/// - 8 cards (5 out): defenders have 3-2 split = 2 length winners
/// - 9 cards (4 out): defenders have 2-2 split = 3 length winners
///
/// The function identifies:
/// - Duck cards: low cards played to exhaust defenders (after cashing winners)
/// - Length winners: cards that win after defenders are exhausted
///
/// **Important**: This function excludes cards that are sure winners or promotable
/// winners (including spent cards). Those are handled by `find_sure_winners` and
/// `find_promotable_winners`. It also excludes cards from the shorter hand that
/// would be played alongside sure winners or promotions (since you must play from
/// both hands on each trick).
///
/// # Arguments
/// * `dummy` - The dummy's hand (typically North)
/// * `declarer` - The declarer's hand (typically South)
///
/// # Returns
/// A `LengthResult` containing:
/// - `ducks`: Cards played to duck and exhaust defenders
/// - `winners`: Cards that will win through length
pub fn find_length_winners(dummy: &Hand, declarer: &Hand) -> LengthResult {
    // Get cards already accounted for by other functions
    let sure_winners = find_sure_winners(dummy, declarer);
    let promotion_result = find_promotable_winners(dummy, declarer);

    let mut result = LengthResult::new();

    for suit in Suit::all() {
        let suit_result =
            find_length_winners_in_suit(dummy, declarer, suit, &sure_winners, &promotion_result);
        result.merge(suit_result);
    }

    result
}

/// Find length winners in a single suit by combining both hands.
///
/// Returns a LengthResult with duck cards and potential length winner cards.
/// Excludes cards that are sure winners, promotable winners, or played alongside them.
fn find_length_winners_in_suit(
    dummy: &Hand,
    declarer: &Hand,
    suit: Suit,
    sure_winners: &[Card],
    promotion_result: &PromotionResult,
) -> LengthResult {
    let dummy_holding = dummy.holding(suit);
    let declarer_holding = declarer.holding(suit);

    let dummy_len = dummy_holding.len();
    let declarer_len = declarer_holding.len();
    let combined_len = dummy_len + declarer_len;

    // Need 7+ combined cards for length winners to be possible
    if combined_len < 7 {
        return LengthResult::new();
    }

    // Filter to only cards in this suit
    let suit_sure_winners: Vec<Card> = sure_winners
        .iter()
        .filter(|c| c.suit == suit)
        .copied()
        .collect();
    let suit_promotion_spent: Vec<Card> = promotion_result
        .spent
        .iter()
        .filter(|c| c.suit == suit)
        .copied()
        .collect();
    let suit_promotion_winners: Vec<Card> = promotion_result
        .winners
        .iter()
        .filter(|c| c.suit == suit)
        .copied()
        .collect();

    // Cards held by defenders
    let defenders_cards = 13 - combined_len;

    // Assume optimal split for defenders (as even as possible)
    let defenders_longer = defenders_cards.div_ceil(2);

    // After defenders' longer holding is exhausted, remaining cards are length winners
    let max_tricks = dummy_len.max(declarer_len);

    // Length winners = max tricks - rounds to exhaust defenders
    // The sure winner tricks ARE part of those rounds (they exhaust defenders too)
    // So we don't subtract accounted_tricks from length_winners_count
    let length_winners_count = max_tricks.saturating_sub(defenders_longer);

    if length_winners_count == 0 {
        return LengthResult::new();
    }

    // Count tricks already accounted for by sure winners and promotions
    // These affect which cards are available, not how many length winners
    let sure_winner_tricks = suit_sure_winners.len();
    let promotion_tricks = suit_promotion_winners.len() + suit_promotion_spent.len();
    let accounted_tricks = sure_winner_tricks + promotion_tricks;

    // Get all ranks sorted by rank (highest first) for both hands
    // Exclude cards already accounted for
    let is_accounted = |card: &Card| {
        suit_sure_winners.contains(card)
            || suit_promotion_spent.contains(card)
            || suit_promotion_winners.contains(card)
    };

    let dummy_ranks: Vec<Rank> = dummy_holding
        .ranks
        .iter()
        .copied()
        .filter(|&r| !is_accounted(&Card::new(suit, r)))
        .collect();
    let declarer_ranks: Vec<Rank> = declarer_holding
        .ranks
        .iter()
        .copied()
        .filter(|&r| !is_accounted(&Card::new(suit, r)))
        .collect();

    // Calculate how many cards from the shorter hand are used as companions
    // to sure winners and promotions (played on the same trick)
    let shorter_hand_cards = if dummy_len >= declarer_len {
        declarer_len
    } else {
        dummy_len
    };

    // Sure winners and promotions are in the longer hand (typically)
    // The shorter hand plays a companion card on each of those tricks
    // Companions are the LOWEST cards from the shorter hand
    let companion_count = accounted_tricks.min(shorter_hand_cards);

    // Remove companion cards from the shorter hand's remaining cards
    let (dummy_available, declarer_available) = if dummy_len >= declarer_len {
        // Declarer is shorter, remove lowest `companion_count` cards
        let mut declarer_sorted: Vec<Rank> = declarer_ranks.clone();
        declarer_sorted.sort(); // highest first
        declarer_sorted.reverse(); // now lowest first
        let declarer_after_companions: Vec<Rank> =
            declarer_sorted.into_iter().skip(companion_count).collect();
        (dummy_ranks.clone(), declarer_after_companions)
    } else {
        // Dummy is shorter, remove lowest `companion_count` cards
        let mut dummy_sorted: Vec<Rank> = dummy_ranks.clone();
        dummy_sorted.sort(); // highest first
        dummy_sorted.reverse(); // now lowest first
        let dummy_after_companions: Vec<Rank> =
            dummy_sorted.into_iter().skip(companion_count).collect();
        (dummy_after_companions, declarer_ranks.clone())
    };

    // Now determine length winners and ducks from remaining cards
    let dummy_avail_len = dummy_available.len();
    let declarer_avail_len = declarer_available.len();

    if dummy_avail_len == 0 && declarer_avail_len == 0 {
        return LengthResult::new();
    }

    // Determine which hand has the length winners (the one with more remaining cards)
    let (winner_hand_ranks, other_hand_ranks) = if dummy_avail_len > declarer_avail_len {
        (dummy_available.clone(), declarer_available.clone())
    } else if declarer_avail_len > dummy_avail_len {
        (declarer_available.clone(), dummy_available.clone())
    } else {
        // Equal length - assign to hand with stronger remaining cards
        let mut dummy_sorted = dummy_available.clone();
        let mut declarer_sorted = declarer_available.clone();
        dummy_sorted.sort();
        declarer_sorted.sort();
        let dummy_best = dummy_sorted.first();
        let declarer_best = declarer_sorted.first();
        match (dummy_best, declarer_best) {
            (Some(d), Some(c)) if d < c => (dummy_available.clone(), declarer_available.clone()),
            _ => (declarer_available.clone(), dummy_available.clone()),
        }
    };

    // Sort winner hand ranks to identify length winners
    let mut sorted_winner_ranks = winner_hand_ranks.clone();
    sorted_winner_ranks.sort(); // Derived Ord puts Ace first (lowest enum value = highest bridge rank)

    // Length winners are the HIGHEST cards in the winner hand
    // After sort(), Ace is first, then King, etc. (because Rank enum ordering)
    // So we take the first N elements to get highest ranks
    let winners: Vec<Card> = sorted_winner_ranks
        .iter()
        .take(length_winners_count)
        .map(|&rank| Card::new(suit, rank))
        .collect();

    // Duck cards are all remaining cards that aren't length winners
    let mut ducks = Vec::new();

    // From winner hand, cards that aren't length winners are ducks
    for &rank in &winner_hand_ranks {
        let card = Card::new(suit, rank);
        if !winners.contains(&card) {
            ducks.push(card);
        }
    }

    // All cards from the other hand are ducks
    for &rank in &other_hand_ranks {
        ducks.push(Card::new(suit, rank));
    }

    // Sort ducks by rank (highest first) for consistent output
    ducks.sort_by(|a, b| a.rank.cmp(&b.rank));

    // Sort winners by rank (highest first)
    let mut winners = winners;
    winners.sort_by(|a, b| a.rank.cmp(&b.rank));

    LengthResult { ducks, winners }
}

/// Find all promotable winners in a NT contract by combining dummy and declarer hands.
///
/// Promotable winners are cards in honor sequences that can become winners after
/// driving out higher honors. The sequence must have more touching honors than
/// there are missing higher honors.
///
/// For example:
/// - KQJ: 3 touching honors, missing 1 higher (Ace) = K spent, QJ promoted
/// - QJT98: 5 touching honors, missing 2 higher (A, K) = QJ spent, T98 promoted
/// - QJT: 3 touching honors, missing 2 higher (A, K) = QJ spent, T promoted
/// - QJ: 2 touching honors, missing 2 higher (A, K) = no promotion possible
///
/// Note: Cards that are already sure winners (part of a sequence from Ace) are
/// not included in promotable winners.
///
/// # Arguments
/// * `dummy` - The dummy's hand (typically North)
/// * `declarer` - The declarer's hand (typically South)
///
/// # Returns
/// A `PromotionResult` containing:
/// - `spent`: Cards used to drive out higher honors
/// - `winners`: Cards that will win after promotion
///
/// # Example
/// ```
/// use pbn_to_pdf::model::{Hand, Holding, Rank, Suit, Card};
/// use pbn_to_pdf::model::analysis::find_promotable_winners;
///
/// let mut dummy = Hand::new();
/// dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack]);
///
/// let declarer = Hand::new();
///
/// let result = find_promotable_winners(&dummy, &declarer);
/// // KQJ missing only A: K is spent, QJ become winners
/// assert_eq!(result.spent.len(), 1);
/// assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::King)));
/// assert_eq!(result.winners.len(), 2);
/// assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
/// assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
/// ```
pub fn find_promotable_winners(dummy: &Hand, declarer: &Hand) -> PromotionResult {
    let mut result = PromotionResult::new();

    for suit in Suit::all() {
        let suit_result = find_promotable_winners_in_suit(dummy, declarer, suit);
        result.merge(suit_result);
    }

    result
}

/// Find promotable winners in a single suit by combining both hands.
///
/// Returns a PromotionResult with spent cards and winner cards.
/// When honors are split between hands with different lengths, prefers to spend
/// honors from the shorter hand first to preserve entries and avoid blocking the suit.
fn find_promotable_winners_in_suit(dummy: &Hand, declarer: &Hand, suit: Suit) -> PromotionResult {
    let dummy_holding = dummy.holding(suit);
    let declarer_holding = declarer.holding(suit);

    // The maximum number of tricks we can take in this suit is limited by
    // the longer holding - you must play from both hands on each trick
    let max_tricks = dummy_holding.len().max(declarer_holding.len());

    if max_tricks == 0 {
        return PromotionResult::new();
    }

    // Combine ranks from both hands
    let mut combined_ranks: Vec<Rank> = dummy_holding
        .ranks
        .iter()
        .chain(declarer_holding.ranks.iter())
        .copied()
        .collect();

    // Sort by rank (Ace highest first)
    combined_ranks.sort();

    if combined_ranks.is_empty() {
        return PromotionResult::new();
    }

    // Find the first card we have (highest rank in combined holding)
    let highest_rank = combined_ranks[0];

    // If we have the Ace, there are no promotable winners (they'd be sure winners)
    if highest_rank == Rank::Ace {
        return PromotionResult::new();
    }

    // Count how many higher honors we're missing
    let all_ranks = Rank::all();
    let highest_idx = all_ranks.iter().position(|&r| r == highest_rank).unwrap();
    let missing_higher = highest_idx; // Number of ranks above our highest

    // Find the continuous sequence starting from our highest card
    let mut sequence = Vec::new();
    let mut combined_iter = combined_ranks.iter().peekable();

    for expected_rank in all_ranks.iter().skip(highest_idx) {
        if combined_iter.peek() == Some(&expected_rank) {
            let rank = *combined_iter.next().unwrap();
            sequence.push(rank);
        } else {
            // Gap in sequence - stop here
            break;
        }
    }

    // Number of promotable winner tricks = sequence_length - missing_higher
    // (we need to give up `missing_higher` tricks to drive out the higher honors)
    if sequence.len() <= missing_higher {
        return PromotionResult::new();
    }

    let promotable_tricks = sequence.len() - missing_higher;

    // Limited by max tricks we can take
    let num_winners = promotable_tricks.min(max_tricks);

    // Determine which hand is shorter (prefer to spend from shorter hand)
    // When equal length, prefer declarer's cards to preserve dummy entries
    let dummy_is_shorter = dummy_holding.len() < declarer_holding.len();
    let declarer_is_shorter = declarer_holding.len() < dummy_holding.len();

    // Select which cards to spend: prefer cards from the shorter hand
    // This preserves entries and avoids blocking the suit
    let mut spent = Vec::new();
    let mut remaining_to_spend = missing_higher;

    // First pass: spend cards from the shorter hand that are in the sequence
    // (skip this if hands are equal length - we'll just take from top)
    if dummy_is_shorter || declarer_is_shorter {
        for &rank in &sequence {
            if remaining_to_spend == 0 {
                break;
            }
            let in_shorter = if dummy_is_shorter {
                dummy_holding.ranks.contains(&rank)
            } else {
                declarer_holding.ranks.contains(&rank)
            };
            if in_shorter {
                spent.push(Card::new(suit, rank));
                remaining_to_spend -= 1;
            }
        }
    }

    // Second pass: if we still need to spend more (or hands are equal length),
    // take highest cards from the sequence that aren't already spent
    for &rank in &sequence {
        if remaining_to_spend == 0 {
            break;
        }
        let card = Card::new(suit, rank);
        if !spent.contains(&card) {
            spent.push(card);
            remaining_to_spend -= 1;
        }
    }

    // Winners are the remaining sequence cards not spent, limited by num_winners
    let winners: Vec<Card> = sequence
        .iter()
        .filter_map(|&rank| {
            let card = Card::new(suit, rank);
            if spent.contains(&card) {
                None
            } else {
                Some(card)
            }
        })
        .take(num_winners)
        .collect();

    // Sort spent cards by rank (highest first) for consistent ordering
    spent.sort_by(|a, b| a.rank.cmp(&b.rank));

    PromotionResult { spent, winners }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Holding;

    #[test]
    fn test_ace_only() {
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace]);

        let declarer = Hand::new();

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 1);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
    }

    #[test]
    fn test_ak_same_hand() {
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::King]);

        let declarer = Hand::new();

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 2);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::King)));
    }

    #[test]
    fn test_ak_split_hands_singleton_each() {
        // Dummy has singleton A, declarer has singleton K
        // On trick 1: Play A from dummy, K must follow from declarer
        // Result: only 1 trick can be taken, the Ace wins
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([Rank::King]);

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 1);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        // King is played on the same trick as Ace, doesn't win separately
        assert!(!winners.contains(&Card::new(Suit::Spades, Rank::King)));
    }

    #[test]
    fn test_ak_split_hands_with_length() {
        // Dummy has A and a small card, declarer has K and a small card
        // Now we can take 2 tricks: A wins trick 1, K wins trick 2
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::Three]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([Rank::King, Rank::Two]);

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 2);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::King)));
    }

    #[test]
    fn test_akqjt_combined_limited_by_hand_length() {
        // Dummy has 3 spades (AQT), declarer has 2 spades (KJ)
        // Combined sequence: AKQJT (5 cards continuous from Ace)
        // But max tricks = max(3, 2) = 3
        // So only top 3 cards (AKQ) are sure winners
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::Queen, Rank::Ten]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([Rank::King, Rank::Jack]);

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 3);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::King)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        // Jack and Ten are NOT sure winners - they get played when AKQ win
        assert!(!winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
        assert!(!winners.contains(&Card::new(Suit::Spades, Rank::Ten)));
    }

    #[test]
    fn test_long_sequence_same_hand() {
        // All 5 cards in one hand - can take 5 tricks
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([
            Rank::Ace,
            Rank::King,
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
        ]);

        let declarer = Hand::new();

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 5);
    }

    #[test]
    fn test_trick_limit_with_equal_lengths() {
        // Dummy has AKQ (3 cards), declarer has JT9 (3 cards)
        // Combined sequence: AKQJT9 (6 cards continuous)
        // Max tricks = max(3, 3) = 3
        // Sure winners: AKQ (the top 3 that win)
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([Rank::Jack, Rank::Ten, Rank::Nine]);

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 3);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::King)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        // JT9 are played but don't win
        assert!(!winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
    }

    #[test]
    fn test_gap_in_sequence() {
        // A-K with gap (missing Q), so only A-K are sure winners
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Jack]);

        let declarer = Hand::new();

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 2);
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::King)));
        // Jack is NOT a sure winner because Queen is missing
        assert!(!winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
    }

    #[test]
    fn test_no_ace_no_winners() {
        // K-Q-J but no Ace - no sure winners
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack]);

        let declarer = Hand::new();

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 0);
    }

    #[test]
    fn test_multiple_suits() {
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::King]);
        dummy.hearts = Holding::from_ranks([Rank::Queen]); // No Ace, no winners

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);
        declarer.clubs = Holding::from_ranks([Rank::Ace]);

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 6); // 2 spades + 0 hearts + 3 diamonds + 1 club

        // Verify specific cards
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Spades, Rank::King)));
        assert!(winners.contains(&Card::new(Suit::Diamonds, Rank::Ace)));
        assert!(winners.contains(&Card::new(Suit::Diamonds, Rank::King)));
        assert!(winners.contains(&Card::new(Suit::Diamonds, Rank::Queen)));
        assert!(winners.contains(&Card::new(Suit::Clubs, Rank::Ace)));

        // Hearts should not be included
        assert!(!winners.contains(&Card::new(Suit::Hearts, Rank::Queen)));
    }

    #[test]
    fn test_empty_hands() {
        let dummy = Hand::new();
        let declarer = Hand::new();

        let winners = find_sure_winners(&dummy, &declarer);
        assert_eq!(winners.len(), 0);
    }

    // Tests for find_promotable_winners

    #[test]
    fn test_promotable_kqj() {
        // KQJ missing only Ace = 2 promotable winners (Q and J)
        // K is used to drive out Ace
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        // K is spent to drive out Ace
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::King)));
        // QJ become winners
        assert_eq!(result.winners.len(), 2);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
    }

    #[test]
    fn test_promotable_qjt98() {
        // QJT98: 5 touching honors, missing A and K = 3 promotable winners
        // Q drives out A, J drives out K, then T98 are winners
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
            Rank::Nine,
            Rank::Eight,
        ]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        // Q and J are spent to drive out A and K
        assert_eq!(result.spent.len(), 2);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::Queen)));
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::Jack)));
        // T98 become winners
        assert_eq!(result.winners.len(), 3);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Ten)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Nine)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Eight)));
    }

    #[test]
    fn test_promotable_qjt() {
        // QJT: 3 touching honors, missing A and K = 1 promotable winner
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Queen, Rank::Jack, Rank::Ten]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        // Q and J are spent to drive out A and K
        assert_eq!(result.spent.len(), 2);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::Queen)));
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::Jack)));
        // T becomes winner
        assert_eq!(result.winners.len(), 1);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Ten)));
    }

    #[test]
    fn test_promotable_qj_not_enough() {
        // QJ: 2 touching honors, missing A and K = 0 promotable winners
        // (sequence length equals missing higher, so no net gain)
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Queen, Rank::Jack]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        // No promotion possible - would need all cards just to drive out higher honors
        assert_eq!(result.spent.len(), 0);
        assert_eq!(result.winners.len(), 0);
        assert!(!result.has_promotion());
    }

    #[test]
    fn test_promotable_with_ace_no_promotable() {
        // AKQ - these are sure winners, not promotable
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        assert_eq!(result.spent.len(), 0);
        assert_eq!(result.winners.len(), 0);
        assert!(!result.has_promotion());
    }

    #[test]
    fn test_promotable_split_hands_equal_length() {
        // Dummy has KQ (2 cards), declarer has JT (2 cards) - equal length
        // Combined: KQJT, missing A = 3 promotable winners possible (QJT)
        // But max tricks = max(2, 2) = 2
        // With equal length, K is spent (highest), only 2 promotable winners (QJ)
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([Rank::Jack, Rank::Ten]);

        let result = find_promotable_winners(&dummy, &declarer);
        // K is spent to drive out Ace (equal length, so take highest)
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::King)));
        // QJ become winners (limited to 2 by max tricks)
        assert_eq!(result.winners.len(), 2);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
        // Ten doesn't win because we can only take 2 tricks total
        assert!(!result.winners.contains(&Card::new(Suit::Spades, Rank::Ten)));
    }

    #[test]
    fn test_promotable_limited_by_hand_length() {
        // Dummy has KQJ (3 cards), declarer has T9 (2 cards)
        // Combined: KQJT9, missing A = 4 promotable winners possible
        // But max tricks = max(3, 2) = 3
        // Declarer is shorter but has no honors in sequence (T9 are in sequence)
        // So T is spent from shorter hand, then K from longer if needed
        // Actually T is in sequence, so spend T first, gives KQJT9 - T = KQJ9
        // Wait, T is part of the sequence KQJT9, so we need 1 card to spend
        // Shorter hand (declarer) has T and 9 in the sequence
        // Spend T from declarer, winners are KQJ9 but limited to 3
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([Rank::Ten, Rank::Nine]);

        let result = find_promotable_winners(&dummy, &declarer);
        // T is spent (from shorter hand - declarer)
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::Ten)));
        // KQJ become winners (9 is after the spent T, limited to 3 by max tricks)
        assert_eq!(result.winners.len(), 3);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::King)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
        // Nine is in sequence but we already have 3 winners
        assert!(!result.winners.contains(&Card::new(Suit::Spades, Rank::Nine)));
    }

    #[test]
    fn test_promotable_gap_in_sequence() {
        // KQT - gap at J, so only KQ sequence (2 cards), missing A = 1 promotable
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Ten]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        // K is spent to drive out Ace
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::King)));
        // Q becomes winner (only card in sequence after K)
        assert_eq!(result.winners.len(), 1);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        // Ten is not part of the continuous sequence
        assert!(!result.winners.contains(&Card::new(Suit::Spades, Rank::Ten)));
    }

    #[test]
    fn test_promotable_multiple_suits() {
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack]); // K spent, QJ winners
        dummy.hearts = Holding::from_ranks([Rank::Queen, Rank::Jack, Rank::Ten]); // QJ spent, T winner

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([Rank::Ace, Rank::King]); // 0 promotable (sure winners)
        declarer.clubs = Holding::from_ranks([Rank::Jack, Rank::Ten]); // 0 promotable (missing AKQ)

        let result = find_promotable_winners(&dummy, &declarer);
        // Spades: K spent, Hearts: QJ spent
        assert_eq!(result.spent.len(), 3);
        // Spades: QJ winners, Hearts: T winner
        assert_eq!(result.winners.len(), 3);
    }

    #[test]
    fn test_promotable_singleton_sequence() {
        // Just K - missing A, sequence of 1 - missing 1 = 0 promotable
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King]);

        let declarer = Hand::new();

        let result = find_promotable_winners(&dummy, &declarer);
        assert_eq!(result.spent.len(), 0);
        assert_eq!(result.winners.len(), 0);
        assert!(!result.has_promotion());
    }

    #[test]
    fn test_promotable_spend_from_shorter_hand() {
        // Dummy has KQT3 (4 cards), declarer has J4 (2 cards)
        // Combined sequence: KQJT (4 cards), missing A = 3 promotable winners
        // Should spend J from shorter hand (declarer), not K from longer hand
        let mut dummy = Hand::new();
        dummy.hearts = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Ten, Rank::Three]);

        let mut declarer = Hand::new();
        declarer.hearts = Holding::from_ranks([Rank::Jack, Rank::Four]);

        let result = find_promotable_winners(&dummy, &declarer);
        // J should be spent (from shorter hand), not K
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Hearts, Rank::Jack)));
        assert!(!result.spent.contains(&Card::new(Suit::Hearts, Rank::King)));
        // KQT become winners (max tricks = 4, but only 3 in sequence after spending J)
        assert_eq!(result.winners.len(), 3);
        assert!(result.winners.contains(&Card::new(Suit::Hearts, Rank::King)));
        assert!(result.winners.contains(&Card::new(Suit::Hearts, Rank::Queen)));
        assert!(result.winners.contains(&Card::new(Suit::Hearts, Rank::Ten)));
    }

    #[test]
    fn test_promotable_equal_length_take_highest() {
        // Dummy has KT3 (3 cards), declarer has QJ4 (3 cards) - equal length
        // Combined sequence: KQJT (4 cards), missing A = 3 promotable winners
        // With equal lengths, should just take highest cards to spend
        let mut dummy = Hand::new();
        dummy.hearts = Holding::from_ranks([Rank::King, Rank::Ten, Rank::Three]);

        let mut declarer = Hand::new();
        declarer.hearts = Holding::from_ranks([Rank::Queen, Rank::Jack, Rank::Four]);

        let result = find_promotable_winners(&dummy, &declarer);
        // K should be spent (highest in sequence, equal length so no preference)
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Hearts, Rank::King)));
        // QJT become winners
        assert_eq!(result.winners.len(), 3);
        assert!(result.winners.contains(&Card::new(Suit::Hearts, Rank::Queen)));
        assert!(result.winners.contains(&Card::new(Suit::Hearts, Rank::Jack)));
        assert!(result.winners.contains(&Card::new(Suit::Hearts, Rank::Ten)));
    }

    #[test]
    fn test_promotable_spend_multiple_from_shorter() {
        // Dummy has K32 (3 cards), declarer has QJT98 (5 cards)
        // Combined sequence: KQJT98 (6 cards), missing A = 5 promotable winners
        // Dummy is shorter, should spend K first, then from declarer
        // Max tricks = 5, so up to 5 winners possible
        let mut dummy = Hand::new();
        dummy.spades = Holding::from_ranks([Rank::King, Rank::Three, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.spades = Holding::from_ranks([
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
            Rank::Nine,
            Rank::Eight,
        ]);

        let result = find_promotable_winners(&dummy, &declarer);
        // K should be spent (from shorter hand)
        assert_eq!(result.spent.len(), 1);
        assert!(result.spent.contains(&Card::new(Suit::Spades, Rank::King)));
        // QJJT98 become winners, but limited to 5 by max tricks
        assert_eq!(result.winners.len(), 5);
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Queen)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Jack)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Ten)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Nine)));
        assert!(result.winners.contains(&Card::new(Suit::Spades, Rank::Eight)));
    }

    // Tests for find_length_winners
    //
    // These tests verify that length winners excludes:
    // 1. Sure winners (like Aces)
    // 2. Companion cards (played from shorter hand alongside sure winners)

    #[test]
    fn test_length_winners_4_3_fit_with_ace() {
        // Dummy has T52 (3 cards), declarer has A764 (4 cards)
        // Combined: 7 cards, defenders have 6 cards (3-3 optimal split)
        // A is a sure winner (excluded from length analysis)
        // 2 is companion (played with A from shorter hand, excluded)
        // Remaining: T5 + 764 = 5 cards (2 dummy, 3 declarer)
        // Max tricks = 4, defenders need 3 rounds to exhaust
        // Length winners = 4 - 3 = 1
        // Length winner = highest in longer remaining hand (declarer: 7)
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Ten, Rank::Five, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds =
            Holding::from_ranks([Rank::Ace, Rank::Seven, Rank::Six, Rank::Four]);

        let result = find_length_winners(&dummy, &declarer);

        // 1 length winner - the 7 (highest in remaining longer hand after excluding A and companion)
        assert_eq!(result.winners.len(), 1);
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Seven)));

        // Ducks: T5 from dummy + 64 from declarer (A excluded as sure winner, 2 as companion)
        assert_eq!(result.ducks.len(), 4);
    }

    #[test]
    fn test_length_winners_4_3_fit_no_sure_winners() {
        // Dummy has T52 (3 cards), declarer has 9764 (4 cards) - no Ace
        // Combined: 7 cards, defenders have 6 cards (3-3 optimal split)
        // No sure winners, so all cards participate in length play
        // Max tricks = 4, tricks to exhaust = 3
        // Length winners = 4 - 3 = 1
        // Length winner = highest in longer hand (declarer: 9)
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Ten, Rank::Five, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds =
            Holding::from_ranks([Rank::Nine, Rank::Seven, Rank::Six, Rank::Four]);

        let result = find_length_winners(&dummy, &declarer);

        // 1 length winner - the 9 (highest in longer hand)
        assert_eq!(result.winners.len(), 1);
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Nine)));

        // Ducks: all other cards (T52 from dummy, 764 from declarer)
        assert_eq!(result.ducks.len(), 6);
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Ten)));
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Five)));
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Two)));
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Seven)));
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Six)));
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Four)));
    }

    #[test]
    fn test_length_winners_board1_clubs() {
        // Board 1 clubs: dummy (South after rotation) has 642, declarer (North) has A983
        // This matches the user's example
        // A is a sure winner (excluded)
        // 2 is played with A (companion, excluded)
        // Remaining: 64 + 983 = 5 cards
        // Duck tricks: 3+4, 8+6
        // Length winner: 9
        let mut dummy = Hand::new();
        dummy.clubs = Holding::from_ranks([Rank::Six, Rank::Four, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.clubs =
            Holding::from_ranks([Rank::Ace, Rank::Nine, Rank::Eight, Rank::Three]);

        let result = find_length_winners(&dummy, &declarer);

        // 1 length winner - the 9 (lowest remaining in longer hand after excluding Ace)
        assert_eq!(result.winners.len(), 1);
        assert!(result.winners.contains(&Card::new(Suit::Clubs, Rank::Nine)));

        // Ducks: 3,8 from declarer + 6,4 from dummy (2 was companion to Ace)
        assert_eq!(result.ducks.len(), 4);
        assert!(result.ducks.contains(&Card::new(Suit::Clubs, Rank::Eight)));
        assert!(result.ducks.contains(&Card::new(Suit::Clubs, Rank::Three)));
        assert!(result.ducks.contains(&Card::new(Suit::Clubs, Rank::Six)));
        assert!(result.ducks.contains(&Card::new(Suit::Clubs, Rank::Four)));
    }

    #[test]
    fn test_length_winners_5_2_fit_with_ak() {
        // Dummy has 52 (2 cards), declarer has AK764 (5 cards)
        // Combined: 7 cards, defenders have 6 cards (3-3 optimal split)
        // AK are sure winners (2 tricks), uses companions 5 and 2 from dummy
        // Remaining: none in dummy + 764 in declarer = 3 cards
        // Max tricks = 5, defenders need 3 rounds to exhaust
        // Length winners = 5 - 3 = 2
        // Length winners = highest 2 in declarer after excluding AK = 7, 6
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Five, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([
            Rank::Ace,
            Rank::King,
            Rank::Seven,
            Rank::Six,
            Rank::Four,
        ]);

        let result = find_length_winners(&dummy, &declarer);

        // 2 length winners - the 7 and 6 (highest remaining after AK excluded)
        assert_eq!(result.winners.len(), 2);
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Seven)));
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Six)));

        // Ducks: 4 from declarer (52 from dummy are companions to AK)
        assert_eq!(result.ducks.len(), 1);
        assert!(result.ducks.contains(&Card::new(Suit::Diamonds, Rank::Four)));
    }

    #[test]
    fn test_length_winners_5_3_fit_with_ak() {
        // Dummy has 532 (3 cards), declarer has AK764 (5 cards)
        // Combined: 8 cards, defenders have 5 cards (3-2 optimal split)
        // AK are sure winners (2 tricks), uses 2 companions from dummy (2, 3)
        // Remaining: 5 from dummy + 764 from declarer = 4 cards
        // Max tricks = 5, defenders need 3 rounds to exhaust
        // Length winners = 5 - 3 = 2
        // Length winners = highest 2 in declarer after excluding AK = 7, 6
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Five, Rank::Three, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([
            Rank::Ace,
            Rank::King,
            Rank::Seven,
            Rank::Six,
            Rank::Four,
        ]);

        let result = find_length_winners(&dummy, &declarer);

        // 2 length winners - the 7 and 6 (highest remaining)
        assert_eq!(result.winners.len(), 2);
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Seven)));
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Six)));
    }

    #[test]
    fn test_length_winners_5_4_fit_with_ak() {
        // Dummy has 5432 (4 cards), declarer has AK876 (5 cards)
        // Combined: 9 cards, defenders have 4 cards (2-2 optimal split)
        // AK are sure winners (2 tricks), uses 2 companions from dummy (2, 3)
        // Remaining: 54 from dummy + 876 from declarer = 5 cards
        // Max tricks = 5, defenders need 2 rounds to exhaust
        // Length winners = 5 - 2 = 3
        // Length winners = highest 3 in declarer after excluding AK = 8, 7, 6
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Five, Rank::Four, Rank::Three, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([
            Rank::Ace,
            Rank::King,
            Rank::Eight,
            Rank::Seven,
            Rank::Six,
        ]);

        let result = find_length_winners(&dummy, &declarer);

        // 3 length winners - the 8, 7, 6 (highest remaining)
        assert_eq!(result.winners.len(), 3);
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Eight)));
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Seven)));
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Six)));
    }

    #[test]
    fn test_length_winners_not_enough_cards() {
        // Dummy has 52 (2 cards), declarer has 976 (3 cards) - no sure winners
        // Combined: 5 cards - not enough for length winners (need 7+)
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Five, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([Rank::Nine, Rank::Seven, Rank::Six]);

        let result = find_length_winners(&dummy, &declarer);

        assert_eq!(result.winners.len(), 0);
        assert_eq!(result.ducks.len(), 0);
        assert!(!result.has_length_winners());
    }

    #[test]
    fn test_length_winners_6_cards_not_enough() {
        // Dummy has 52 (2 cards), declarer has 9764 (4 cards) - no sure winners
        // Combined: 6 cards - not enough for length winners (need 7+)
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Five, Rank::Two]);

        let mut declarer = Hand::new();
        declarer.diamonds =
            Holding::from_ranks([Rank::Nine, Rank::Seven, Rank::Six, Rank::Four]);

        let result = find_length_winners(&dummy, &declarer);

        assert_eq!(result.winners.len(), 0);
        assert!(!result.has_length_winners());
    }

    #[test]
    fn test_length_winners_equal_length_no_sure_winners() {
        // Dummy has T954 (4 cards), declarer has 763 (3 cards) - no sure winners
        // Combined: 7 cards, defenders have 6 (3-3 split)
        // Max tricks = 4, tricks to exhaust = 3
        // Length winners = 4 - 3 = 1
        // Dummy is longer, so length winner is in dummy (the highest = T)
        let mut dummy = Hand::new();
        dummy.diamonds =
            Holding::from_ranks([Rank::Ten, Rank::Nine, Rank::Five, Rank::Four]);

        let mut declarer = Hand::new();
        declarer.diamonds = Holding::from_ranks([Rank::Seven, Rank::Six, Rank::Three]);

        let result = find_length_winners(&dummy, &declarer);

        // 1 length winner in dummy (longer hand) - the T (highest)
        assert_eq!(result.winners.len(), 1);
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Ten)));
    }

    #[test]
    fn test_length_winners_multiple_suits_with_promotable() {
        // Test with multiple suits - diamonds has length winners, clubs has promotable
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Ten, Rank::Five, Rank::Two]);
        dummy.clubs = Holding::from_ranks([Rank::Nine, Rank::Eight]);

        let mut declarer = Hand::new();
        declarer.diamonds =
            Holding::from_ranks([Rank::Nine, Rank::Seven, Rank::Six, Rank::Four]);
        declarer.clubs = Holding::from_ranks([
            Rank::Ten,
            Rank::Seven,
            Rank::Six,
            Rank::Four,
            Rank::Three,
        ]);

        let result = find_length_winners(&dummy, &declarer);

        // Diamonds: 7 cards (4-3 fit), no promotable = 1 length winner (highest = 9)
        // Clubs: promotable winners consume most cards, leaving only 43 for length play
        // After T9876 are accounted for as promotable, only 43 remain
        // With 2 length winners expected (5-3=2), both 4 and 3 become length winners
        assert_eq!(result.winners.len(), 3);
        assert!(result.has_length_winners());
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Nine)));
        // Clubs 4 and 3 are the remaining length winners after promotable cards excluded
        assert!(result.winners.contains(&Card::new(Suit::Clubs, Rank::Four)));
        assert!(result.winners.contains(&Card::new(Suit::Clubs, Rank::Three)));
    }

    #[test]
    fn test_length_winners_multiple_suits_no_promotable() {
        // Test with multiple suits having length winners, no promotable sequences
        let mut dummy = Hand::new();
        dummy.diamonds = Holding::from_ranks([Rank::Ten, Rank::Five, Rank::Two]);
        dummy.clubs = Holding::from_ranks([Rank::Nine, Rank::Five]); // Non-touching, no promotion

        let mut declarer = Hand::new();
        declarer.diamonds =
            Holding::from_ranks([Rank::Nine, Rank::Seven, Rank::Six, Rank::Four]);
        declarer.clubs = Holding::from_ranks([
            Rank::Eight,
            Rank::Seven,
            Rank::Four,
            Rank::Three,
            Rank::Two,
        ]); // 87 touches but missing too many higher cards

        let result = find_length_winners(&dummy, &declarer);

        // Diamonds: 7 cards (4-3 fit) = 1 length winner (highest = 9)
        // Clubs: 7 cards (5-2 fit), no promotable = 2 length winners (highest = 8, 7)
        assert_eq!(result.winners.len(), 3);
        assert!(result.has_length_winners());
        assert!(result.winners.contains(&Card::new(Suit::Diamonds, Rank::Nine)));
        assert!(result.winners.contains(&Card::new(Suit::Clubs, Rank::Eight)));
        assert!(result.winners.contains(&Card::new(Suit::Clubs, Rank::Seven)));
    }
}
