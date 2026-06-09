use std::collections::HashMap;

use balatro_rs::card::{Card as BalatroCard, Suit, Value};
use balatro_rs::hand::SelectHand;
pub use balatro_rs::rank::HandRank;
use itertools::Itertools;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CardValue {
    pub suit: String,
    pub rank: String,
}

#[derive(Deserialize, Debug)]
pub struct Card {
    pub value: CardValue,
}

#[derive(Deserialize, Debug)]
pub struct HandInfo {
    pub cards: Vec<Card>,
}

#[derive(Deserialize, Debug)]
pub struct HandScore {
    pub chips: u64,
    pub mult: u64,
}

#[derive(Deserialize, Debug)]
pub struct RoundInfo {
    pub discards_left: u64,
}

#[derive(Deserialize, Debug)]
pub struct BlindInfo {
    pub status: String,
    pub score: u64,
}

#[derive(Deserialize, Debug)]
pub struct Blinds {
    pub small: BlindInfo,
    pub big: BlindInfo,
    pub boss: BlindInfo,
}

#[derive(Deserialize, Debug)]
pub struct GameState {
    pub state: String,
    pub stake: String,
    pub hand: HandInfo,
    pub hands: HashMap<String, HandScore>,
    pub round: RoundInfo,
    pub blinds: Blinds,
}

pub fn rank_to_value(r: &str) -> Option<Value> {
    match r {
        "2" => Some(Value::Two),
        "3" => Some(Value::Three),
        "4" => Some(Value::Four),
        "5" => Some(Value::Five),
        "6" => Some(Value::Six),
        "7" => Some(Value::Seven),
        "8" => Some(Value::Eight),
        "9" => Some(Value::Nine),
        "T" => Some(Value::Ten),
        "J" => Some(Value::Jack),
        "Q" => Some(Value::Queen),
        "K" => Some(Value::King),
        "A" => Some(Value::Ace),
        _ => None,
    }
}

pub fn suit_to_suit(s: &str) -> Option<Suit> {
    match s {
        "S" => Some(Suit::Spade),
        "C" => Some(Suit::Club),
        "H" => Some(Suit::Heart),
        "D" => Some(Suit::Diamond),
        _ => None,
    }
}

pub fn hand_rank_to_api_key(rank: HandRank) -> &'static str {
    match rank {
        HandRank::HighCard => "High Card",
        HandRank::OnePair => "Pair",
        HandRank::TwoPair => "Two Pair",
        HandRank::ThreeOfAKind => "Three of a Kind",
        HandRank::Straight => "Straight",
        HandRank::Flush => "Flush",
        HandRank::FullHouse => "Full House",
        HandRank::FourOfAKind => "Four of a Kind",
        HandRank::StraightFlush | HandRank::RoyalFlush => "Straight Flush",
        HandRank::FiveOfAKind => "Five of a Kind",
        HandRank::FlushHouse => "Flush House",
        HandRank::FlushFive => "Flush Five",
    }
}

pub fn card_chips(card: &BalatroCard) -> u64 {
    match card.value {
        Value::Two => 2,
        Value::Three => 3,
        Value::Four => 4,
        Value::Five => 5,
        Value::Six => 6,
        Value::Seven => 7,
        Value::Eight => 8,
        Value::Nine => 9,
        Value::Ten => 10,
        Value::Jack => 10,
        Value::Queen => 10,
        Value::King => 10,
        Value::Ace => 11,
    }
}

pub fn convert_card(api_card: &Card) -> Option<BalatroCard> {
    let value = rank_to_value(&api_card.value.rank)?;
    let suit = suit_to_suit(&api_card.value.suit)?;
    Some(BalatroCard::new(value, suit))
}

pub fn evaluate_combo(
    cards: &[BalatroCard],
    scores: &HashMap<String, HandScore>,
) -> Option<(u64, HandRank)> {
    let select = SelectHand::new(cards.to_vec());
    let made = select.best_hand().ok()?;
    let key = hand_rank_to_api_key(made.rank);
    let score = scores.get(key)?;
    let total_chips: u64 = cards.iter().map(card_chips).sum();
    let total = (total_chips + score.chips) * score.mult;
    Some((total, made.rank))
}

pub fn find_best_hand(hand: &[Card], scores: &HashMap<String, HandScore>) -> (Vec<usize>, u64) {
    let indices: Vec<usize> = (0..hand.len()).collect();
    let mut best_score = 0u64;
    let mut best_indices: Vec<usize> = (0..std::cmp::min(5, hand.len())).collect();

    for combo in indices.into_iter().combinations(5) {
        let api_cards: Vec<&Card> = combo.iter().map(|&i| &hand[i]).collect();
        let balatro_cards: Vec<BalatroCard> =
            api_cards.iter().filter_map(|c| convert_card(c)).collect();
        if balatro_cards.len() < 5 {
            continue;
        }
        if let Some((total, _rank)) = evaluate_combo(&balatro_cards, scores)
            && total > best_score
        {
            best_score = total;
            best_indices = combo;
        }
    }

    (best_indices, best_score)
}

pub fn current_blind_score(blinds: &Blinds) -> u64 {
    if blinds.small.status == "CURRENT" {
        blinds.small.score
    } else if blinds.big.status == "CURRENT" {
        blinds.big.score
    } else {
        blinds.boss.score
    }
}

pub fn current_blind_name(blinds: &Blinds) -> &str {
    if blinds.small.status == "CURRENT" {
        "small blind"
    } else if blinds.big.status == "CURRENT" {
        "big blind"
    } else {
        "boss blind"
    }
}

pub fn remaining_deck(hand: &[Card]) -> Vec<BalatroCard> {
    let mut deck: Vec<BalatroCard> = Vec::with_capacity(52);
    for suit in Suit::suits() {
        for value in Value::values() {
            deck.push(BalatroCard::new(value, suit));
        }
    }
    for api_card in hand {
        if let Some(hc) = convert_card(api_card)
            && let Some(pos) = deck
                .iter()
                .position(|c| c.value == hc.value && c.suit == hc.suit)
        {
            deck.swap_remove(pos);
        }
    }
    deck
}

pub fn find_best_balatro_hand(
    cards: &[BalatroCard],
    scores: &HashMap<String, HandScore>,
) -> (u64, HandRank) {
    let mut best_score = 0u64;
    let mut best_rank = HandRank::HighCard;

    for combo in (0..cards.len()).combinations(5) {
        let combo_cards: Vec<BalatroCard> = combo.iter().map(|&i| cards[i]).collect();
        if let Some((total, rank)) = evaluate_combo(&combo_cards, scores)
            && total > best_score
        {
            best_score = total;
            best_rank = rank;
        }
    }

    (best_score, best_rank)
}

pub fn rollout_once(
    hand: &[BalatroCard],
    deck: &[BalatroCard],
    target: u64,
    scores: &HashMap<String, HandScore>,
    discard_set: &[usize],
    rng: &mut SmallRng,
) -> Option<HandRank> {
    let mut current = hand.to_vec();
    let mut remains = deck.to_vec();

    let draw_count = discard_set.len();
    if draw_count > remains.len() {
        return None;
    }

    let drawn: Vec<BalatroCard> = remains.choose_multiple(rng, draw_count).cloned().collect();
    for d in &drawn {
        if let Some(pos) = remains.iter().position(|c| c.id == d.id) {
            remains.swap_remove(pos);
        }
    }

    let mut sorted = discard_set.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    for &i in &sorted {
        current.swap_remove(i);
    }

    current.extend(drawn);

    let (best_score, best_rank) = find_best_balatro_hand(&current, scores);
    if best_score >= target {
        Some(best_rank)
    } else {
        None
    }
}
