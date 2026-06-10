use balatro_ai::GameState;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Debug)]
struct RpcResponse {
    result: Option<GameState>,
}

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u32,
}

const API_URL: &str = "http://127.0.0.1:12346";

#[tokio::main]
async fn main() {
    let verbose = std::env::args().any(|a| a == "-v" || a == "--verbose");

    let client = reqwest::Client::new();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "gamestate".to_string(),
        params: json!({}),
        id: 1,
    };

    let res = client
        .post(API_URL)
        .json(&req)
        .send()
        .await
        .expect("failed to connect to BalatroBot");

    let rpc_res: RpcResponse = res.json().await.expect("failed to parse response");
    let state = rpc_res.result.expect("no game state");

    println!("═══ Game State ═══");
    println!("State:     {}", state.state);
    println!("Round:     {}", state.round_num);
    println!("Ante:      {}", state.ante_num);
    println!("Money:     ${}", state.money);
    if let Some(ref s) = state.stake {
        println!("Stake:     {}", s);
    }
    if let Some(ref d) = state.deck {
        println!("Deck:      {}", d);
    }
    if let Some(ref s) = state.seed {
        println!("Seed:      {}", s);
    }
    if let Some(won) = state.won {
        println!("Won:       {}", won);
    }
    println!();

    println!("─── Round ───");
    if let Some(v) = state.round.hands_left {
        println!("  Hands left:   {}", v);
    }
    if let Some(v) = state.round.hands_played {
        println!("  Hands played: {}", v);
    }
    if let Some(v) = state.round.discards_left {
        println!("  Discards left:  {}", v);
    }
    if let Some(v) = state.round.discards_used {
        println!("  Discards used:  {}", v);
    }
    if let Some(v) = state.round.reroll_cost {
        println!("  Reroll cost:    ${}", v);
    }
    if let Some(v) = state.round.chips {
        println!("  Chips scored:   {}", v);
    }
    println!();

    // ── Blinds ──────────────────────────────────────────────────────────────
    println!("─── Blinds ───");
    for (name, blind) in [
        ("small", &state.blinds.small),
        ("big", &state.blinds.big),
        ("boss", &state.blinds.boss),
    ] {
        println!("  {}:", name);
        println!("    type:   {}", blind.blind_type);
        println!("    status: {}", blind.status);
        println!("    name:   {}", blind.name);
        if !blind.effect.is_empty() && verbose {
            println!("    effect: {}", blind.effect);
        }
        println!("    score:  {}", blind.score);
        if !blind.tag_name.is_empty() {
            if verbose {
                println!("    tag:    {} — {}", blind.tag_name, blind.tag_effect);
            } else {
                println!("    tag:    {}", blind.tag_name);
            }
        }
    }
    println!();

    // ── Hands (poker hand levels) ────────────────────────────────────────────
    if !state.hands.is_empty() {
        println!("─── Poker Hand Levels ───");
        let mut sorted: Vec<_> = state.hands.iter().collect();
        sorted.sort_by(|a, b| a.1.order.cmp(&b.1.order));
        for (name, hand) in &sorted {
            if verbose {
                println!(
                    "  {:20} Lv{:<2}  {} chips × {} mult  ({}× this round)",
                    name, hand.level, hand.chips, hand.mult, hand.played_this_round
                );
            } else {
                println!("  {:20} Lv{:<2}", name, hand.level);
            }
        }
        println!();
    }

    // ── Hand cards ──────────────────────────────────────────────────────────
    if let Some(ref area) = state.hand {
        println!("─── Hand ({}/{}) ───", area.cards.len(), area.limit);
        for (i, card) in area.cards.iter().enumerate() {
            print_card(i, card, verbose);
        }
        println!();
    }

    // ── Jokers ──────────────────────────────────────────────────────────────
    if let Some(ref area) = state.jokers {
        if !area.cards.is_empty() {
            println!("─── Jokers ({}/{}) ───", area.cards.len(), area.limit);
            for (i, card) in area.cards.iter().enumerate() {
                if verbose {
                    print_card(i, card, true);
                } else {
                    println!("  [{}] {}", i, card.label);
                    println!("       key:   {}", card.key);
                    if let Some(ref e) = card.modifier.edition {
                        println!("       ed:    {}", e);
                    }
                    if let Some(ref s) = card.modifier.seal {
                        println!("       seal:  {}", s);
                    }
                    if card.modifier.eternal.unwrap_or(false) {
                        println!("       eternal");
                    }
                    if let Some(p) = card.modifier.perishable {
                        println!("       perishable: {} rounds", p);
                    }
                    if card.modifier.rental.unwrap_or(false) {
                        println!("       rental");
                    }
                    if card.state.debuff.unwrap_or(false) {
                        println!("       debuffed");
                    }
                    if card.state.highlight.unwrap_or(false) {
                        println!("       highlighted");
                    }
                }
            }
            println!();
        }
    }

    // ── Consumables ──────────────────────────────────────────────────────────
    if let Some(ref area) = state.consumables {
        if !area.cards.is_empty() {
            println!("─── Consumables ({}/{}) ───", area.cards.len(), area.limit);
            for (i, card) in area.cards.iter().enumerate() {
                print_card(i, card, verbose);
            }
            println!();
        }
    }

    // ── Remaining deck ──────────────────────────────────────────────────────
    if let Some(ref area) = state.cards {
        if verbose {
            println!("─── Deck ({}/{}) ───", area.cards.len(), area.limit);
            for (i, card) in area.cards.iter().enumerate() {
                print_card(i, card, true);
            }
        } else {
            println!("─── Deck ({}/{}) ───", area.cards.len(), area.limit);
        }
        println!();
    }

    // ── Shop ────────────────────────────────────────────────────────────────
    if let Some(ref area) = state.shop {
        if !area.cards.is_empty() {
            println!("─── Shop ({}/{}) ───", area.cards.len(), area.limit);
            for (i, card) in area.cards.iter().enumerate() {
                if verbose {
                    print_card(i, card, true);
                } else {
                    println!("  [{}] {}", i, card.key);
                }
            }
            println!();
        }
    }

    // ── Booster packs ──────────────────────────────────────────────────────
    if let Some(ref area) = state.packs {
        if !area.cards.is_empty() {
            println!("─── Booster Packs ({}) ───", area.cards.len());
            for (i, card) in area.cards.iter().enumerate() {
                if verbose {
                    print_card(i, card, true);
                } else {
                    println!("  [{}] {}", i, card.key);
                }
            }
            println!();
        }
    }
}

fn print_card(i: usize, card: &balatro_ai::ApiCard, verbose: bool) {
    println!("  [{}] {}", i, card.label);
    if verbose {
        println!("       key:   {}", card.key);
    }
    println!("       set:   {}", card.set);
    println!("       id:    {}", card.id);
    if let Some(ref s) = card.value.suit {
        print!("       suit:  {}", s);
        if let Some(ref r) = card.value.rank {
            print!("  rank: {}", r);
        }
        println!();
    }
    if !card.value.effect.is_empty() {
        println!("       desc:  {}", card.value.effect);
    }
    if let Some(ref e) = card.modifier.edition {
        println!("       ed:    {}", e);
    }
    if let Some(ref e) = card.modifier.enhancement {
        println!("       enh:   {}", e);
    }
    if let Some(ref s) = card.modifier.seal {
        println!("       seal:  {}", s);
    }
    if card.modifier.eternal.unwrap_or(false) {
        println!("       eternal");
    }
    if let Some(p) = card.modifier.perishable {
        println!("       perishable: {} rounds", p);
    }
    if card.modifier.rental.unwrap_or(false) {
        println!("       rental");
    }
    if card.state.debuff.unwrap_or(false) {
        println!("       debuffed");
    }
    if card.state.highlight.unwrap_or(false) {
        println!("       highlighted");
    }
    if verbose && (card.cost.sell > 0 || card.cost.buy > 0) {
        println!("       sell: ${}  buy: ${}", card.cost.sell, card.cost.buy);
    }
}
