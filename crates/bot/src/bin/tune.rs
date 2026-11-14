//! SPSA tuner for the S2 evaluation weights.
//!
//! # Why a proxy
//!
//! One 20-game match against attax costs ~10 minutes, so tuning directly on
//! win rate is out of reach. The proxy is a single-player game with a
//! scripted garbage schedule: baseline rows plus periodic spikes, with the
//! bot's attack cancelling queued garbage exactly like the referee. That keeps
//! the failure mode the M2 gate exposed (a spike the stack cannot absorb)
//! while removing the opponent, so one evaluation is ~1.5 s instead of 30 s.
//!
//! The tuner optimises **multipliers** on a subset of weights, so the
//! hand-set scale is preserved and the search space is bounded.
//!
//! # Staged design
//!
//! 1. SPSA on the proxy over tuning seeds only (this binary).
//! 2. Validate the result on held-back proxy seeds (different from the SPSA
//!    seeds) to catch proxy overfitting.
//! 3. Verify on the real match (`scripts/m2-match.sh`, tuning seeds
//!    9001-9010 x both sheets). Held-out seeds (10001+) are never used here.
//!
//! Usage:
//! ```text
//! cargo run --release -p intetrigence-bot --bin tune -- --iterations 20
//! ```

use std::time::Duration;

use intetrigence_bot::{SearchConfig, Weights, play_game_with_garbage};

/// Tunable parameters as multipliers on the hand-set weights.
const PARAMS: &[&str] = &[
    "danger_projected",
    "garbage_rows",
    "garbage_clear",
    "holes",
    "hole_rows",
    "attack",
];

const MIN_MULT: f64 = 0.4;
const MAX_MULT: f64 = 2.5;

fn apply(base: &Weights, mult: &[f64]) -> Weights {
    let mut w = *base;
    for (name, &m) in PARAMS.iter().zip(mult) {
        match *name {
            "danger_projected" => w.danger_projected *= m,
            "garbage_rows" => w.garbage_rows *= m,
            "garbage_clear" => w.garbage_clear *= m,
            "holes" => w.holes *= m,
            "hole_rows" => w.hole_rows *= m,
            "attack" => w.attack *= m,
            other => panic!("unknown parameter {other}"),
        }
    }
    w
}

/// Baseline rows every 12 turns plus a spike every 40 turns. Tuned so the
/// untuned bot dies between turns 45 and 208 on the proxy seeds, which leaves
/// room for improvement without saturating at the turn cap.
fn schedule() -> Vec<(u32, u32)> {
    let mut injections = Vec::new();
    for turn in 0..300u32 {
        if turn % 12 == 0 {
            injections.push((turn, 2));
        }
        if turn > 0 && turn % 40 == 0 {
            injections.push((turn, 12));
        }
    }
    injections
}

fn objective(weights: &Weights, seeds: &[u64], config: &SearchConfig) -> f64 {
    let mut config = config.clone();
    config.weights = *weights;
    let injections = schedule();
    let mut total = 0.0;
    for &seed in seeds {
        let report = play_game_with_garbage(seed, &config, 300, &injections);
        // Survival is the primary signal; attack keeps the tuner from
        // collapsing into pure defence.
        total += f64::from(report.turns) + 0.05 * f64::from(report.attack);
    }
    total / seeds.len() as f64
}

/// The proxy must be **deterministic**: a time budget would make the beam
/// width vary with machine load, which is pure noise for a gradient estimate.
/// A fixed width at `max_width` stops the anytime loop after one iteration.
fn proxy_config() -> SearchConfig {
    SearchConfig {
        depth: 6,
        width: 32,
        max_width: 32,
        threads: 1,
        budget: Duration::from_secs(600),
        ..SearchConfig::default()
    }
}

fn main() {
    let mut iterations = 20usize;
    let mut seed_from = 1u64;
    let mut seed_to = 6u64;
    let mut check: Option<Vec<f64>> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iterations" => iterations = args.next().unwrap().parse().unwrap(),
            "--seeds" => {
                let spec = args.next().unwrap();
                let (a, b) = spec.split_once('-').expect("seeds as A-B");
                seed_from = a.parse().unwrap();
                seed_to = b.parse().unwrap();
            }
            "--check" => {
                let spec = args.next().expect("--check needs multipliers");
                check = Some(spec.split(',').map(|v| v.parse().unwrap()).collect());
            }
            other => panic!("unknown argument {other}"),
        }
    }

    let base = Weights::default();
    let config = proxy_config();
    let seeds: Vec<u64> = (seed_from..=seed_to).collect();

    if let Some(mult) = check {
        assert_eq!(mult.len(), PARAMS.len(), "one multiplier per parameter");
        let score = objective(&apply(&base, &mult), &seeds, &config);
        println!("check {mult:?}: objective {score:.2} (seeds {seed_from}..{seed_to})");
        return;
    }

    let mut theta = vec![1.0f64; PARAMS.len()];
    let mut best = theta.clone();
    let mut best_score = objective(&apply(&base, &theta), &seeds, &config);
    println!("start: objective {best_score:.2} (seeds {seed_from}..{seed_to})");

    // SPSA with a normalised step: the raw gradient is O(100) because the
    // objective is in turns, so the update is scaled by the largest component
    // and capped. A plain `a * gradient` immediately pinned every parameter to
    // a bound in the first run.
    let (step, c, gamma) = (0.10, 0.12, 0.101);
    let mut rng = intetrigence_core::SplitMix64::new(0xC0FF_EE00_1234_5678);

    for k in 0..iterations {
        let step_k = step / (k as f64 + 1.0).powf(0.25);
        let c_k = c / (k as f64 + 1.0).powf(gamma);
        let delta: Vec<f64> = (0..PARAMS.len())
            .map(|_| if rng.below(2) == 0 { -1.0 } else { 1.0 })
            .collect();

        let plus: Vec<f64> = theta
            .iter()
            .zip(&delta)
            .map(|(t, d)| (t + c_k * d).clamp(MIN_MULT, MAX_MULT))
            .collect();
        let minus: Vec<f64> = theta
            .iter()
            .zip(&delta)
            .map(|(t, d)| (t - c_k * d).clamp(MIN_MULT, MAX_MULT))
            .collect();

        let j_plus = objective(&apply(&base, &plus), &seeds, &config);
        let j_minus = objective(&apply(&base, &minus), &seeds, &config);

        let gradient: Vec<f64> = (0..theta.len())
            .map(|i| (j_plus - j_minus) / (2.0 * c_k * delta[i]))
            .collect();
        let scale = gradient
            .iter()
            .fold(0.0f64, |m, g| m.max(g.abs()))
            .max(1e-9);
        for i in 0..theta.len() {
            theta[i] = (theta[i] + step_k * gradient[i] / scale).clamp(MIN_MULT, MAX_MULT);
        }

        let score = objective(&apply(&base, &theta), &seeds, &config);
        if score > best_score {
            best_score = score;
            best = theta.clone();
        }
        println!(
            "iter {:2}: J+ {j_plus:.1} J- {j_minus:.1} theta {score:.1} best {best_score:.1}",
            k + 1
        );
    }

    println!("\nbest multipliers (objective {best_score:.2}):");
    for (name, &m) in PARAMS.iter().zip(&best) {
        println!("  {name:18} x{m:.3}");
    }
    let tuned = apply(&base, &best);
    println!("\ntuned weights:");
    println!("{tuned:#?}");
}
