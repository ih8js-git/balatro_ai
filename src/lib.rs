use std::collections::HashMap;

use balatro_rs::card::{Card as BalatroCard, Suit, Value};
use balatro_rs::hand::SelectHand;
pub use balatro_rs::rank::HandRank;
use itertools::Itertools;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use serde::Deserialize;

fn deser_mod<'de, D: serde::Deserializer<'de>>(d: D) -> Result<CardModifier, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum H {
        Obj(CardModifier),
        #[allow(dead_code)]
        Arr(Vec<()>),
    }
    match H::deserialize(d)? {
        H::Obj(v) => Ok(v),
        H::Arr(_) => Ok(CardModifier::default()),
    }
}

fn deser_map_or_empty<'de, D>(d: D) -> Result<HashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum H {
        Map(HashMap<String, String>),
        #[allow(dead_code)]
        Arr(Vec<()>),
    }
    match H::deserialize(d)? {
        H::Map(v) => Ok(v),
        H::Arr(_) => Ok(HashMap::new()),
    }
}

fn deser_state<'de, D: serde::Deserializer<'de>>(d: D) -> Result<CardState, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum H {
        Obj(CardState),
        #[allow(dead_code)]
        Arr(Vec<()>),
    }
    match H::deserialize(d)? {
        H::Obj(v) => Ok(v),
        H::Arr(_) => Ok(CardState::default()),
    }
}

// ── API Card Types ──────────────────────────────────────────────────────────

#[derive(Default, Deserialize, Debug)]
pub struct CardModifier {
    #[serde(default)]
    pub seal: Option<String>,
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub enhancement: Option<String>,
    #[serde(default)]
    pub eternal: Option<bool>,
    #[serde(default)]
    pub perishable: Option<u64>,
    #[serde(default)]
    pub rental: Option<bool>,
}

#[derive(Default, Deserialize, Debug)]
pub struct CardState {
    #[serde(default)]
    pub debuff: Option<bool>,
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub highlight: Option<bool>,
}

#[derive(Default, Deserialize, Debug)]
pub struct CardCost {
    pub sell: u64,
    pub buy: u64,
}

#[derive(Default, Deserialize, Debug)]
pub struct CardValue {
    #[serde(default)]
    pub suit: Option<String>,
    #[serde(default)]
    pub rank: Option<String>,
    #[serde(default)]
    pub effect: String,
}

#[derive(Deserialize, Debug)]
pub struct ApiCard {
    pub id: u64,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub set: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub value: CardValue,
    #[serde(default, deserialize_with = "deser_mod")]
    pub modifier: CardModifier,
    #[serde(default, deserialize_with = "deser_state")]
    pub state: CardState,
    #[serde(default)]
    pub cost: CardCost,
}

// ── Area Types ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct Area {
    pub count: u64,
    pub limit: u64,
    #[serde(default)]
    pub highlighted_limit: Option<u64>,
    #[serde(default)]
    pub cards: Vec<ApiCard>,
}

// ── Hand Data ───────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct HandData {
    pub order: u64,
    pub level: u64,
    pub chips: u64,
    pub mult: u64,
    pub played: u64,
    pub played_this_round: u64,
}

// ── Round Info ──────────────────────────────────────────────────────────────

#[derive(Default, Deserialize, Debug)]
pub struct RoundInfo {
    #[serde(default)]
    pub hands_left: Option<u64>,
    #[serde(default)]
    pub hands_played: Option<u64>,
    #[serde(default)]
    pub discards_left: Option<u64>,
    #[serde(default)]
    pub discards_used: Option<u64>,
    #[serde(default)]
    pub reroll_cost: Option<u64>,
    #[serde(default)]
    pub chips: Option<u64>,
}

// ── Blind Info ──────────────────────────────────────────────────────────────

#[derive(Default, Deserialize, Debug)]
pub struct BlindInfo {
    #[serde(rename = "type")]
    #[serde(default)]
    pub blind_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub effect: String,
    pub score: u64,
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub tag_effect: String,
}

#[derive(Default, Deserialize, Debug)]
pub struct Blinds {
    pub small: BlindInfo,
    pub big: BlindInfo,
    pub boss: BlindInfo,
}

// ── Game State ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct GameState {
    pub state: String,
    #[serde(default)]
    pub round_num: u64,
    #[serde(default)]
    pub ante_num: u64,
    #[serde(default)]
    pub money: u64,
    #[serde(default)]
    pub won: Option<bool>,
    #[serde(default)]
    pub deck: Option<String>,
    #[serde(default)]
    pub stake: Option<String>,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default, deserialize_with = "deser_map_or_empty")]
    pub used_vouchers: HashMap<String, String>,
    #[serde(default)]
    pub hands: HashMap<String, HandData>,
    #[serde(default)]
    pub round: RoundInfo,
    #[serde(default)]
    pub blinds: Blinds,
    #[serde(default)]
    pub jokers: Option<Area>,
    #[serde(default)]
    pub consumables: Option<Area>,
    #[serde(default)]
    pub hand: Option<Area>,
    #[serde(default)]
    pub cards: Option<Area>,
    #[serde(default)]
    pub shop: Option<Area>,
    #[serde(default)]
    pub vouchers: Option<Area>,
    #[serde(default)]
    pub packs: Option<Area>,
    #[serde(default)]
    pub pack: Option<Area>,
}

// ── Conversion Functions ────────────────────────────────────────────────────

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

pub fn convert_card(api_card: &ApiCard) -> Option<BalatroCard> {
    let value = rank_to_value(api_card.value.rank.as_ref()?)?;
    let suit = suit_to_suit(api_card.value.suit.as_ref()?)?;
    Some(BalatroCard::new(value, suit))
}

/// Convert only the hand area cards, checking both the area and player state guard.
pub fn hand_cards(state: &GameState) -> &[ApiCard] {
    state.hand.as_ref().map(|h| h.cards.as_slice()).unwrap_or(&[])
}

// ── Scoring Functions ───────────────────────────────────────────────────────

pub fn evaluate_combo(
    cards: &[BalatroCard],
    scores: &HashMap<String, HandData>,
) -> Option<(u64, HandRank)> {
    let select = SelectHand::new(cards.to_vec());
    let made = select.best_hand().ok()?;
    let key = hand_rank_to_api_key(made.rank);
    let score = scores.get(key)?;
    let total_chips: u64 = cards.iter().map(card_chips).sum();
    let total = (total_chips + score.chips) * score.mult;
    Some((total, made.rank))
}

pub fn find_best_hand(hand: &[ApiCard], scores: &HashMap<String, HandData>) -> (Vec<usize>, u64) {
    let indices: Vec<usize> = (0..hand.len()).collect();
    let mut best_score = 0u64;
    let mut best_indices: Vec<usize> = (0..std::cmp::min(5, hand.len())).collect();

    for combo in indices.into_iter().combinations(5) {
        let api_cards: Vec<&ApiCard> = combo.iter().map(|&i| &hand[i]).collect();
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

pub fn remaining_deck(hand: &[ApiCard]) -> Vec<BalatroCard> {
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
    scores: &HashMap<String, HandData>,
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
    scores: &HashMap<String, HandData>,
    discard_set: &[usize],
    rng: &mut SmallRng,
) -> Option<HandRank> {
    let mut current = hand.to_vec();
    let remains = deck.to_vec();

    let draw_count = discard_set.len();
    if draw_count > remains.len() {
        return None;
    }

    let drawn: Vec<BalatroCard> = remains.choose_multiple(rng, draw_count).cloned().collect();

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
