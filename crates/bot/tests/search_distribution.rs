//! Diagnostic: what the anytime loop completes in a real game, turn by turn.
//!
//! The synthetic profile in `search_profile.rs` uses a hand-built six-row
//! board. A real game reaches turn 200 with a bumpy stack and garbage arriving,
//! and the question is whether the depth and width hold up there. This plays
//! the garbage-pressure proxy and buckets the completed search statistics by
//! turn, so a depth that collapses late is visible.
//!
//! ```text
//! cargo test -p intetrigence-bot --test search_distribution -- --ignored --nocapture
//! ```

use intetrigence_bot::{SearchConfig, SearchStats, play_game_with_setup};
use std::time::Duration;

fn bucket(turn: usize) -> &'static str {
    match turn {
        0..=49 => "turn 1-50",
        50..=99 => "turn 51-100",
        100..=149 => "turn 101-150",
        150..=199 => "turn 151-200",
        _ => "turn 201+",
    }
}

fn summarise(label: &str, stats: &[SearchStats], turns: u32) {
    println!("\n{label} ({turns} turns)");
    let fallbacks = stats.iter().filter(|s| s.fallback).count();
    println!(
        "  fallback {fallbacks}/{} ({:.1}%)",
        stats.len(),
        100.0 * fallbacks as f64 / stats.len().max(1) as f64
    );
    let mut buckets: Vec<(&str, Vec<&SearchStats>)> = Vec::new();
    for (index, stat) in stats.iter().enumerate() {
        let name = bucket(index);
        match buckets.iter_mut().find(|(n, _)| *n == name) {
            Some((_, list)) => list.push(stat),
            None => buckets.push((name, vec![stat])),
        }
    }
    println!(
        "  {:<14} {:>5} {:>6} {:>7} {:>7} {:>7}",
        "bucket", "moves", "depth", "width", "nodes", "iters"
    );
    for (name, list) in buckets {
        if list.is_empty() {
            continue;
        }
        let n = list.len() as f64;
        let depth = list.iter().map(|s| s.depth as f64).sum::<f64>() / n;
        let width = list.iter().map(|s| s.width as f64).sum::<f64>() / n;
        let nodes = list.iter().map(|s| s.nodes as f64).sum::<f64>() / n;
        let iters = list.iter().map(|s| s.iterations as f64).sum::<f64>() / n;
        println!("  {name:<14} {n:5.0} {depth:6.1} {width:7.0} {nodes:7.0} {iters:7.1}");
    }
}

#[test]
#[ignore = "diagnostic: minutes of search"]
fn real_game_search_distribution() {
    // The first run of this measured depth ~2 and it was wrong: `cargo test`
    // builds in debug, where the search is several times slower, so the
    // completed depth was an artefact of the build profile. The referee runs
    // `--release`, so only a release measurement means anything.
    assert!(
        !cfg!(debug_assertions),
        "run this with --release: the depth it reports is build-profile dependent"
    );
    let injections: Vec<(u32, u32)> = (20..400u32).step_by(8).map(|t| (t, 3)).collect();
    for (label, budget_ms, threads) in [
        ("100 ms / 8 threads", 100u64, 8usize),
        ("100 ms / 1 thread", 100, 1),
    ] {
        let config = SearchConfig {
            budget: Duration::from_millis(budget_ms),
            threads,
            ..SearchConfig::default()
        };
        let report = play_game_with_setup(9001, &config, 400, &injections, 0);
        summarise(label, &report.search_stats, report.turns);
        println!("  topped_out {}", report.topped_out);
    }
}
