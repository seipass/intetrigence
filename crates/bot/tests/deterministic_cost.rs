//! Diagnostic: what a fixed schedule costs, worst case.
//!
//! Deterministic mode has no deadline, so the worst-case think time is a
//! property of the schedule and the board. The official round needs it under
//! the 100 ms budget, so this reports the max and p99 per schedule.
//!
//! ```text
//! cargo test --release -p intetrigence-bot --test deterministic_cost -- --ignored --nocapture
//! ```

use intetrigence_bot::{SearchConfig, play_game_with_setup};
use std::time::Duration;

#[test]
#[ignore = "diagnostic: minutes of search"]
fn deterministic_schedule_cost() {
    if cfg!(debug_assertions) {
        panic!("run with --release");
    }
    // `official` is the profile the round uses: an empty board, which has the
    // most candidates. `garbage-pressure` starts buried and may be cheaper.
    let injections: Vec<(u32, u32)> = Vec::new();
    println!(
        "{:<18} {:>6} {:>8} {:>8} {:>8} {:>7}",
        "schedule", "moves", "median", "p99", "max", "over100"
    );
    for (depth, width) in [(4usize, 128usize), (6, 96), (6, 128), (8, 96), (8, 64)] {
        let config = SearchConfig {
            deterministic: true,
            depth,
            width,
            threads: 8,
            budget: Duration::from_millis(100),
            ..SearchConfig::default()
        };
        // 400 turns on the official profile: long enough to reach the tall,
        // sparse boards where the candidate count peaks.
        let report = play_game_with_setup(9001, &config, 400, &injections, 0);
        let mut times = report.think_times.clone();
        times.sort();
        let n = times.len();
        let median = times.get(n / 2).copied().unwrap_or_default();
        let p99 = times.get((n * 99) / 100).copied().unwrap_or_default();
        let max = times.last().copied().unwrap_or_default();
        let over = times
            .iter()
            .filter(|t| **t > Duration::from_millis(100))
            .count();
        println!(
            "d{depth} w{width:<14} {n:6} {:>8.1} {:>8.1} {:>8.1} {over:7}",
            median.as_secs_f64() * 1000.0,
            p99.as_secs_f64() * 1000.0,
            max.as_secs_f64() * 1000.0,
        );
    }
}
