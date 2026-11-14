//! The deterministic search must produce the same game twice.
//!
//! The bot layer tests the search's determinism directly; this test goes
//! through the arena (`run_round` -> `BotAgent`) and compares every non-header
//! replay record. The header differs only by `started_at`.

use std::time::Duration;

use intetrigence_arena::agent::Agent;
use intetrigence_arena::bot_agent::{BotAgent, BotAgentConfig};
use intetrigence_arena::engine::Sheet;
use intetrigence_arena::referee::{RoundConfig, run_round};
use intetrigence_arena::replay::ReplayRecord;
use intetrigence_bot::SearchConfig;

fn run(seed: u64, threads: usize) -> Vec<ReplayRecord> {
    let search = SearchConfig {
        depth: 3,
        width: 16,
        max_width: 16,
        threads,
        budget: Duration::from_millis(20),
        deterministic: true,
        ..SearchConfig::default()
    };
    let mut bot = BotAgent::new(BotAgentConfig::new(search.clone()));
    let mut opponent = BotAgent::new(BotAgentConfig::new(search));
    let mut config = RoundConfig::new(seed, Sheet::A);
    config.turn_cap = 30;
    config.deterministic = true;
    let mut agents: [&mut dyn Agent; 2] = [&mut bot, &mut opponent];
    run_round(&config, &mut agents)
        .expect("deterministic round")
        .records
}

fn body(records: &[ReplayRecord]) -> Vec<serde_json::Value> {
    records
        .iter()
        .filter(|record| !matches!(record, ReplayRecord::Header(_)))
        .map(|record| {
            let mut value = serde_json::to_value(record).expect("serialize record");
            // `think_ms` is wall-clock time, not a decision: the deterministic
            // mode promises the same game, not the same timing.
            if let Some(placements) = value.get_mut("placements").and_then(|v| v.as_array_mut()) {
                for placement in placements {
                    if let Some(object) = placement.as_object_mut() {
                        object.remove("think_ms");
                    }
                }
            }
            value
        })
        .collect()
}

#[test]
fn deterministic_search_repeats_the_same_game() {
    // The deterministic mode forces the sequential path, so the requested
    // thread count must not change the game (the official round uses 8).
    for threads in [2, 8] {
        let first = run(9001, threads);
        let second = run(9001, threads);
        assert!(first.len() > 1, "the round must produce turns");
        assert_eq!(body(&first), body(&second), "threads={threads}");
    }
}
