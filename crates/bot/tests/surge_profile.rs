//! Diagnostic: the surge release *size* distribution.
//!
//! Across 372 real games against attax the bot released surge 1,602 times, but
//! 80% of those bursts were 4-7 lines, which a bot sitting at height 3-5
//! absorbs. This harness runs the composite defence profile (start on a tall
//! board, then absorb late spikes) and reports how big the releases are, so the
//! charge-value terms can be A/B'd on the metric that matters.
//!
//! ```text
//! SURGE_EXPONENT=1.5 SURGE_HOLD=-60 cargo test -p intetrigence-bot \
//!     --test surge_profile -- --ignored --nocapture
//! ```
//!
//! The numbers are a diagnostic proxy, never a strength claim.

use intetrigence_bot::{SearchConfig, Weights, play_game_with_setup};
use std::collections::BTreeMap;

fn weights_from_env() -> Weights {
    let mut weights = Weights::default();
    if let Ok(v) = std::env::var("SURGE_EXPONENT") {
        weights.surge_exponent = v.parse().expect("SURGE_EXPONENT is a number");
    }
    if let Ok(v) = std::env::var("SURGE_HOLD") {
        weights.surge_hold_penalty = v.parse().expect("SURGE_HOLD is a number");
    }
    if let Ok(v) = std::env::var("SURVIVAL") {
        weights.survival = v != "0";
    }
    weights
}

#[test]
#[ignore = "diagnostic: minutes of search, prints a distribution"]
fn surge_release_size_distribution() {
    let config = SearchConfig {
        weights: weights_from_env(),
        ..SearchConfig::default()
    };
    // A steady trickle, not a buried start: the bot has to build a streak
    // while absorbing garbage, which is the state its surge logic runs in.
    // (Starting at 16 rows puts it in survival mode from turn 0 and it never
    // builds a streak at all -- measured, 0 releases in 6 games.)
    let injections: Vec<(u32, u32)> = (20..200u32).step_by(8).map(|t| (t, 2)).collect();

    let mut releases = Vec::new();
    let mut turns = 0u32;
    let mut topped_out = 0;
    for seed in 9001..=9006u64 {
        let report = play_game_with_setup(seed, &config, 200, &injections, 0);
        turns += report.turns;
        topped_out += u32::from(report.topped_out);
        println!(
            "seed {seed}: turns {} lines {} attack {} releases {:?} topped_out {}",
            report.turns, report.lines, report.attack, report.surge_releases, report.topped_out
        );
        releases.extend(report.surge_releases.iter().copied());
    }

    let mut hist: BTreeMap<u32, u32> = BTreeMap::new();
    for size in &releases {
        *hist.entry(*size).or_insert(0) += 1;
    }
    let big = releases.iter().filter(|size| **size >= 8).count();
    let huge = releases.iter().filter(|size| **size >= 12).count();
    let total: u32 = releases.iter().sum();
    let per_game = releases.len() as f64 / 6.0;
    let big_pct = 100.0 * big as f64 / releases.len().max(1) as f64;
    println!(
        "releases {} ({per_game:.1}/game) lines {total} big(>=8) {big} ({big_pct:.1}%) huge(>=12) {huge}",
        releases.len(),
    );
    println!("turns {turns} topped_out {topped_out}");
    println!("histogram {hist:?}");
}
