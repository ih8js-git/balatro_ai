use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use balatro_rs::card::{Card as BalatroCard, Suit, Value};
use balatro_rs::hand::SelectHand;
use balatro_rs::rank::HandRank;
use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::sleep;

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u32,
}

#[derive(Deserialize, Debug)]
struct CardValue {
    suit: String,
    rank: String,
}

#[derive(Deserialize, Debug)]
struct Card {
    value: CardValue,
}

#[derive(Deserialize, Debug)]
struct HandInfo {
    cards: Vec<Card>,
}

#[derive(Deserialize, Debug)]
struct HandScore {
    chips: u64,
    mult: u64,
}

#[derive(Deserialize, Debug)]
struct RoundInfo {
    discards_left: u64,
}

#[derive(Deserialize, Debug)]
struct BlindInfo {
    status: String,
    score: u64,
}

#[derive(Deserialize, Debug)]
struct Blinds {
    small: BlindInfo,
    big: BlindInfo,
    boss: BlindInfo,
}

#[derive(Deserialize, Debug)]
struct GameState {
    state: String,
    stake: String,
    hand: HandInfo,
    hands: HashMap<String, HandScore>,
    round: RoundInfo,
    blinds: Blinds,
}

#[derive(Deserialize, Debug)]
struct RpcResponse {
    result: Option<GameState>,
}

const API_URL: &str = "http://127.0.0.1:12346";

fn rank_to_value(r: &str) -> Option<Value> {
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

fn suit_to_suit(s: &str) -> Option<Suit> {
    match s {
        "S" => Some(Suit::Spade),
        "C" => Some(Suit::Club),
        "H" => Some(Suit::Heart),
        "D" => Some(Suit::Diamond),
        _ => None,
    }
}

fn hand_rank_to_api_key(rank: HandRank) -> &'static str {
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

fn card_chips(card: &BalatroCard) -> u64 {
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

fn convert_card(api_card: &Card) -> Option<BalatroCard> {
    let value = rank_to_value(&api_card.value.rank)?;
    let suit = suit_to_suit(&api_card.value.suit)?;
    Some(BalatroCard::new(value, suit))
}

fn evaluate_combo(
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

fn find_best_hand(hand: &[Card], scores: &HashMap<String, HandScore>) -> (Vec<usize>, u64) {
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

fn current_blind_score(blinds: &Blinds) -> u64 {
    if blinds.small.status == "CURRENT" {
        blinds.small.score
    } else if blinds.big.status == "CURRENT" {
        blinds.big.score
    } else {
        blinds.boss.score
    }
}

fn current_blind_name(blinds: &Blinds) -> &str {
    if blinds.small.status == "CURRENT" {
        "small blind"
    } else if blinds.big.status == "CURRENT" {
        "big blind"
    } else {
        "boss blind"
    }
}

fn remaining_deck(hand: &[Card]) -> Vec<BalatroCard> {
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

fn find_best_balatro_hand(
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

fn rollout_once(
    hand: &[BalatroCard],
    deck: &[BalatroCard],
    target: u64,
    scores: &HashMap<String, HandScore>,
    discard_set: &[usize],
    rng: &mut ThreadRng,
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

#[tokio::main]
async fn main() {
    let client = Client::new();
    println!("Waiting for BalatroBot...");

    loop {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "gamestate".to_string(),
            params: json!({}),
            id: 1,
        };

        if let Ok(res) = client.post(API_URL).json(&req).send().await
            && let Ok(rpc_res) = res.json::<RpcResponse>().await
            && let Some(state) = rpc_res.result
            && state.state == "SELECTING_HAND"
            && !state.hand.cards.is_empty()
        {
            if state.stake != "WHITE" {
                println!("Only white stake supported, got {}", state.stake);
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            let target = current_blind_score(&state.blinds);
            let blind = current_blind_name(&state.blinds);
            let (best_indices, best_score) = find_best_hand(&state.hand.cards, &state.hands);

            if best_score >= target {
                println!("Playing hand: {} >= {} ({})", best_score, target, blind);
                let play_req = JsonRpcRequest {
                    jsonrpc: "2.0".to_string(),
                    method: "play".to_string(),
                    params: json!({ "cards": best_indices }),
                    id: 2,
                };
                let _ = client.post(API_URL).json(&play_req).send().await;
                sleep(Duration::from_secs(2)).await;
            } else if state.round.discards_left > 0 {
                let deck = remaining_deck(&state.hand.cards);
                let hand_cards: Vec<BalatroCard> =
                    state.hand.cards.iter().filter_map(convert_card).collect();
                let hand_size = state.hand.cards.len();
                let max_discard = std::cmp::min(5, hand_size);

                println!(
                    "{} < {} ({}), {} discard(s) available",
                    best_score, target, blind, state.round.discards_left
                );

                let mut best_discard: Vec<usize> = Vec::new();
                let mut best_prob = 0.0f64;
                let mut best_hand_probs: HashMap<HandRank, f64> = HashMap::new();
                let eval_start = std::time::Instant::now();

                for discard_count in 1..=max_discard {
                    for discard_set in (0..hand_size).combinations(discard_count) {
                        print!("  evaluating discard {:?}  ", discard_set);
                        let _ = std::io::stdout().flush();
                        let set_start = std::time::Instant::now();

                        let mut rng = thread_rng();
                        let mut wins = 0usize;
                        let mut rank_wins: HashMap<HandRank, usize> = HashMap::new();
                        for _ in 0..100 {
                            if let Some(rank) = rollout_once(
                                &hand_cards,
                                &deck,
                                target,
                                &state.hands,
                                &discard_set,
                                &mut rng,
                            ) {
                                wins += 1;
                                *rank_wins.entry(rank).or_insert(0) += 1;
                            }
                        }

                        let total = wins as f64 / 100.0;
                        let set_elapsed = set_start.elapsed();
                        println!(
                            "{:.1}ms (p={:.1}%)",
                            set_elapsed.as_secs_f64() * 1000.0,
                            total * 100.0
                        );
                        if total > best_prob {
                            best_prob = total;
                            best_discard = discard_set;
                            best_hand_probs = rank_wins
                                .into_iter()
                                .map(|(rank, count)| (rank, count as f64 / 100.0))
                                .collect();
                        }
                    }
                }

                let eval_elapsed = eval_start.elapsed();
                println!("  total evaluation: {:.1}s", eval_elapsed.as_secs_f64());

                if best_prob > 0.0 {
                    println!(
                        "{} < {} ({}), discarding {:?} ({:.1}% win)",
                        best_score,
                        target,
                        blind,
                        best_discard,
                        best_prob * 100.0
                    );
                    let mut sorted: Vec<_> = best_hand_probs.iter().collect();
                    sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
                    for (rank, prob) in sorted {
                        if *prob > 0.001 {
                            println!("  {}: {:.1}%", hand_rank_to_api_key(*rank), prob * 100.0);
                        }
                    }
                    let discard_req = JsonRpcRequest {
                        jsonrpc: "2.0".to_string(),
                        method: "discard".to_string(),
                        params: json!({ "cards": best_discard }),
                        id: 2,
                    };
                    let _ = client.post(API_URL).json(&discard_req).send().await;
                    sleep(Duration::from_secs(1)).await;
                } else {
                    println!(
                        "{} < {} ({}), no winning draw possible, playing anyway",
                        best_score, target, blind
                    );
                    let play_req = JsonRpcRequest {
                        jsonrpc: "2.0".to_string(),
                        method: "play".to_string(),
                        params: json!({ "cards": best_indices }),
                        id: 2,
                    };
                    let _ = client.post(API_URL).json(&play_req).send().await;
                    sleep(Duration::from_secs(2)).await;
                }
            } else {
                println!(
                    "{} < {} ({}), no discards left, we lost",
                    best_score, target, blind
                );
            }
        }
        sleep(Duration::from_millis(500)).await;
    }
}
