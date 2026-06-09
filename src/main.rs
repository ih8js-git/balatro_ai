use std::collections::HashMap;
use std::time::Duration;

use balatro_ai::{
    GameState, HandRank, convert_card, current_blind_name, current_blind_score, find_best_hand,
    hand_rank_to_api_key, remaining_deck, rollout_once,
};
use balatro_rs::card::Card as BalatroCard;
use itertools::Itertools;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rayon::prelude::*;
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
struct RpcResponse {
    result: Option<GameState>,
}

const API_URL: &str = "http://127.0.0.1:12346";

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
                    let example: Vec<usize> = (0..discard_count).collect();
                    println!("  evaluating discard {:?}", example);
                    let results: Vec<(Vec<usize>, f64, HashMap<HandRank, usize>)> = (0..hand_size)
                        .combinations(discard_count)
                        .par_bridge()
                        .map(|discard_set| {
                            let mut rng = SmallRng::from_entropy();
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
                            (discard_set, total, rank_wins)
                        })
                        .collect();

                    for (discard_set, total, rank_wins) in results {
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
                    sleep(Duration::from_secs(1)).await;
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
