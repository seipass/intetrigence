//! Diagnostic: the depth the anytime loop actually completes, on `official`.
//!
//! The loop deepens before it widens, so with a high enough cap the normal
//! move should reach depth 10 while a heavy move falls back to whatever
//! completed inside the budget. This reports the distribution.
//!
//! ```text
//! cargo test --release -p intetrigence-bot --test anytime_depth -- --ignored --nocapture
//! ```

use intetrigence_bot::{SearchConfig, play_game_with_setup};
use std::time::Duration;

#[test]
#[ignore = "diagnostic: minutes of search"]
fn anytime_depth_distribution_on_official() {
    if cfg!(debug_assertions) {
        panic!("run with --release");
    }
    for (depth, width, max_width) in [(12usize, 96usize, 768usize), (12, 128, 768), (16, 96, 768)] {
        let config = SearchConfig {
            deterministic: false,
            depth,
            width,
            max_width,
            threads: 8,
            budget: Duration::from_millis(100),
            ..SearchConfig::default()
        };
        let report = play_game_with_setup(9001, &config, 400, &[], 0);
        let mut times = report.think_times.clone();
        times.sort();
        let n = times.len();
        let mut depths: Vec<usize> = report.search_stats.iter().map(|s| s.depth).collect();
        depths.sort();
        let pct = |p: usize| depths.get((n * p) / 100).copied().unwrap_or(0);
        let over = times
            .iter()
            .filter(|t| **t > Duration::from_millis(100))
            .count();
        println!(
            "cap d{depth} w{width}: median {:?} p99 {:?} max {:?} over100 {over} | depth p50 {} p90 {} p99 {} max {}",
            times[n / 2],
            times[(n * 99) / 100],
            times[n - 1],
            pct(50),
            pct(90),
            pct(99),
            depths.last().copied().unwrap_or(0),
        );
    }
}
