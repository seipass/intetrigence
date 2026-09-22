use crate::{
    game::{Game, RulesProfile},
    hoiko::{HoikoConfig, HoikoSearcher},
    search::{Budget, SearchStats, Searcher},
    tbp_client::TbpClient,
};
use intetrigence_engine::data::{Piece, Placement};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::{io::Write, path::Path};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct MatchResult {
    pub seed: u64,
    pub swapped: bool,
    pub winner: Option<usize>,
    pub turns: u32,
    pub pieces: [u32; 2],
    pub attack: [u32; 2],
    pub lines: [u32; 2],
}

/// Match participants understood by the local arena launcher.  `ColdClear2`
/// is a separately spawned TBP executable; the other two participants run in
/// this process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchAgent {
    Hoiko,
    Intetrigence,
    ColdClear2,
}

impl MatchAgent {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "hoiko" => Some(Self::Hoiko),
            "intetrigence" | "intelligence" => Some(Self::Intetrigence),
            "coldclear2" | "coldclear" | "cc2" => Some(Self::ColdClear2),
            _ => None,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Hoiko => "hoiko",
            Self::Intetrigence => "intetrigence",
            Self::ColdClear2 => "cold_clear_2",
        }
    }
}

/// Run the reverse-engineered Hoiko backend against the current Rust searcher.
/// `swapped=false` places Hoiko in player 0; `swapped=true` puts it in player
/// 1 while keeping the paired seed and referee behavior identical.
pub fn run_hoiko_with_pps_rules(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    pps: Option<f64>,
    rules: RulesProfile,
    config: &HoikoConfig,
    mut replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    if pps.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err("pps must be a finite positive number".into());
    }
    let mut games = [
        Game::new_with_rules(seed, rules),
        Game::new_with_rules(seed ^ 0x9e3779b97f4a7c15, rules),
    ];
    let mut garbage = [
        ChaCha8Rng::seed_from_u64(seed ^ 12345),
        ChaCha8Rng::seed_from_u64(seed ^ 67890),
    ];
    let hoiko_player = usize::from(swapped);
    let agents = if swapped {
        ["intetrigence", "hoiko"]
    } else {
        ["hoiko", "intetrigence"]
    };
    let hoiko = HoikoSearcher::new(config.clone());
    let mut intetrigence = [Searcher::new(true, None), Searcher::new(true, None)];
    if let Some(w) = replay.as_mut() {
        writeln!(
            w,
            "{}",
            serde_json::json!({
                "type":"start",
                "replay_version": 2,
                "seed":seed,
                "swapped":swapped,
                "turn_limit":limit,
                "ms":budget.milliseconds,
                "iterations":budget.iterations,
                "pps":pps,
                "rules":rules.name(),
                "agents":agents,
                "upstream":"hoiko-reverse-engineered",
                "initial":[replay_initial_state(&games[0],agents[0]),replay_initial_state(&games[1],agents[1])],
                "board_cells":"row_major_bottom_up; null|block|garbage",
            })
        ).map_err(|e| e.to_string())?;
    }
    let mut turns = 0;
    let turn_period = pps.map(|value| Duration::from_secs_f64(1.0 / value));
    for turn in 0..limit {
        let turn_start = Instant::now();
        turns = turn + 1;
        let snapshots = [games[0].opponent_snapshot(), games[1].opponent_snapshot()];
        games[0].set_opponent_snapshot(snapshots[1]);
        games[1].set_opponent_snapshot(snapshots[0]);
        let mut decisions = [
            (None, SearchStats::default()),
            (None, SearchStats::default()),
        ];
        for player in 0..2 {
            let (mv, stats) = if player == hoiko_player {
                let (mv, hs) = hoiko.choose(&games[player], budget);
                (
                    mv,
                    SearchStats {
                        nodes: hs.nodes,
                        iterations: hs.depth as u64,
                        elapsed_us: hs.elapsed_us,
                        reused_tree: false,
                    },
                )
            } else {
                intetrigence[player].choose(&games[player], budget)
            };
            decisions[player] = (mv, stats);
        }
        let mut sent = [0; 2];
        for (player, (mv, stats)) in decisions.into_iter().enumerate() {
            let active = games[player].queue.front().copied();
            let hold_before = games[player].hold;
            let next_before = games[player]
                .queue
                .iter()
                .skip(1)
                .take(5)
                .copied()
                .collect();
            let hold_used = mv.is_some_and(|placement| Some(placement.location.piece) != active);
            let outcome = if let Some(mv) = mv {
                let outcome = games[player].play(mv)?;
                sent[player] = outcome.sent;
                Some(outcome)
            } else {
                games[player].dead = true;
                None
            };
            if let Some(w) = replay.as_mut() {
                writeln!(
                    w,
                    "{}",
                    replay_move_json(
                        turn,
                        player,
                        agents[player],
                        mv,
                        outcome,
                        stats,
                        active,
                        hold_before,
                        next_before,
                        hold_used,
                        &games[player],
                    )
                )
                .map_err(|e| e.to_string())?;
            }
        }
        games[0].receive(sent[1], &mut garbage[0]);
        games[1].receive(sent[0], &mut garbage[1]);
        if games.iter().any(|game| game.dead) {
            break;
        }
        if let Some(period) = turn_period {
            if let Some(remaining) = period.checked_sub(turn_start.elapsed()) {
                std::thread::sleep(remaining);
            }
        }
    }
    let winner = match (games[0].dead, games[1].dead) {
        (true, false) => Some(1),
        (false, true) => Some(0),
        _ => None,
    };
    let result = MatchResult {
        seed,
        swapped,
        winner,
        turns,
        pieces: [games[0].pieces, games[1].pieces],
        attack: [games[0].attack, games[1].attack],
        lines: [games[0].lines, games[1].lines],
    };
    if let Some(w) = replay.as_mut() {
        writeln!(
            w,
            "{}",
            serde_json::json!({"type":"result","result":result,"winner_agent":result.winner.map(|player| agents[player])})
        ).map_err(|e| e.to_string())?;
    }
    Ok(result)
}

fn internal_agents(swapped: bool) -> [&'static str; 2] {
    if swapped {
        ["cold_clear_2_baseline", "intetrigence"]
    } else {
        ["intetrigence", "cold_clear_2_baseline"]
    }
}

fn replay_initial_state(game: &Game, agent: &str) -> serde_json::Value {
    serde_json::json!({
        "agent": agent,
        "active": game.queue.front().copied(),
        "hold": game.hold,
        "next": game.queue.iter().skip(1).take(5).copied().collect::<Vec<_>>(),
    })
}

fn replay_move_json(
    turn: u32,
    player: usize,
    agent: &str,
    mv: Option<Placement>,
    outcome: Option<crate::game::Outcome>,
    stats: SearchStats,
    active: Option<Piece>,
    hold_before: Option<Piece>,
    next_before: Vec<Piece>,
    hold_used: bool,
    after: &Game,
) -> serde_json::Value {
    serde_json::json!({
        "type": "move",
        "turn": turn,
        "player": player,
        "agent": agent,
        "move": mv,
        "active": active,
        "hold_used": hold_used,
        "hold_before": hold_before,
        "next_before": next_before,
        "hold_after": after.hold,
        "next_after": after.queue.iter().take(5).copied().collect::<Vec<_>>(),
        "outcome": outcome,
        "stats": stats,
        // `board` remains the compact legacy occupancy format. `board_cells`
        // is row-major from bottom to top and labels occupied cells.
        "board": after.board.cols,
        "board_cells": after.board_cells_json(),
        "pending": after.pending,
    })
}

#[derive(Clone)]
enum ExternalLocalAgent {
    Intetrigence,
    Hoiko(HoikoConfig),
}

impl ExternalLocalAgent {
    const fn name(&self) -> &'static str {
        match self {
            Self::Intetrigence => "intetrigence",
            Self::Hoiko(_) => "hoiko",
        }
    }
}

enum LocalBackend {
    Intetrigence(Searcher),
    Hoiko(HoikoSearcher),
}

impl LocalBackend {
    fn new(agent: ExternalLocalAgent, weights: Option<&serde_json::Value>) -> Self {
        match agent {
            ExternalLocalAgent::Intetrigence => Self::Intetrigence(Searcher::new(true, weights)),
            ExternalLocalAgent::Hoiko(config) => Self::Hoiko(HoikoSearcher::new(config)),
        }
    }

    fn choose(&mut self, game: &Game, budget: Budget) -> (Option<Placement>, SearchStats) {
        match self {
            Self::Intetrigence(searcher) => searcher.choose(game, budget),
            Self::Hoiko(searcher) => {
                let (mv, stats) = searcher.choose(game, budget);
                (
                    mv,
                    SearchStats {
                        nodes: stats.nodes,
                        iterations: stats.depth as u64,
                        elapsed_us: stats.elapsed_us,
                        reused_tree: false,
                    },
                )
            }
        }
    }
}

pub fn run(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_with_pps(seed, swapped, limit, budget, weights, None, replay)
}

/// Run a match with an optional fixed turn cadence. A turn is one locked piece
/// for each player. When `pps` is set, the arena waits until the next turn
/// boundary, so both players receive the same real-time piece rate.
pub fn run_with_pps(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_with_pps_rules(
        seed,
        swapped,
        limit,
        budget,
        weights,
        pps,
        RulesProfile::Arena,
        replay,
    )
}

/// Run a match under an explicit versus rules profile.
pub fn run_with_pps_rules(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    rules: RulesProfile,
    mut replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    if pps.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err("pps must be a finite positive number".into());
    }
    let mut games = [
        Game::new_with_rules(seed, rules),
        Game::new_with_rules(seed ^ 0x9e3779b97f4a7c15, rules),
    ];
    let mut garbage = [
        ChaCha8Rng::seed_from_u64(seed ^ 12345),
        ChaCha8Rng::seed_from_u64(seed ^ 67890),
    ];
    let improved = [!swapped, swapped];
    let agents = internal_agents(swapped);
    let mut searchers = [
        Searcher::new(improved[0], weights),
        Searcher::new(improved[1], weights),
    ];
    if let Some(w) = replay.as_mut() {
        writeln!(
            w,
            "{}",
            serde_json::json!({
                "type":"start",
                "replay_version": 2,
                "seed":seed,
                "swapped":swapped,
                "turn_limit":limit,
                "ms":budget.milliseconds,
                "iterations":budget.iterations,
                "pps":pps,
                "rules":rules.name(),
                "upstream":"ed8b19327b6bd1410ddd873d8611485bd45d8fae",
                "weights":weights,
                "agents": agents,
                "initial": [
                    replay_initial_state(&games[0], agents[0]),
                    replay_initial_state(&games[1], agents[1]),
                ],
                "board_cells": "row_major_bottom_up; null|block|garbage",
            })
        )
        .map_err(|e| e.to_string())?;
    }
    let mut turns = 0;
    let turn_period = pps.map(|value| Duration::from_secs_f64(1.0 / value));
    for turn in 0..limit {
        let turn_start = Instant::now();
        turns = turn + 1;
        let decisions = [
            searchers[0].choose(&games[0], budget),
            searchers[1].choose(&games[1], budget),
        ];
        let mut sent = [0; 2];
        for (i, (mv, stats)) in decisions.into_iter().enumerate() {
            let active = games[i].queue.front().copied();
            let hold_before = games[i].hold;
            let next_before = games[i].queue.iter().skip(1).take(5).copied().collect();
            let hold_used = mv.is_some_and(|placement| Some(placement.location.piece) != active);
            let outcome = if let Some(mv) = mv {
                let o = games[i].play(mv)?;
                sent[i] = o.sent;
                Some(o)
            } else {
                games[i].dead = true;
                None
            };
            if let Some(w) = replay.as_mut() {
                writeln!(
                    w,
                    "{}",
                    replay_move_json(
                        turn,
                        i,
                        agents[i],
                        mv,
                        outcome,
                        stats,
                        active,
                        hold_before,
                        next_before,
                        hold_used,
                        &games[i],
                    )
                )
                .map_err(|e| e.to_string())?;
            }
        }
        // Both players decide before either observes this turn's outgoing attack.
        games[0].receive(sent[1], &mut garbage[0]);
        games[1].receive(sent[0], &mut garbage[1]);
        if games.iter().any(|g| g.dead) {
            break;
        }
        if let Some(period) = turn_period {
            if let Some(remaining) = period.checked_sub(turn_start.elapsed()) {
                std::thread::sleep(remaining);
            }
        }
    }
    let winner = match (games[0].dead, games[1].dead) {
        (true, false) => Some(1),
        (false, true) => Some(0),
        _ => None,
    };
    let result = MatchResult {
        seed,
        swapped,
        winner,
        turns,
        pieces: [games[0].pieces, games[1].pieces],
        attack: [games[0].attack, games[1].attack],
        lines: [games[0].lines, games[1].lines],
    };
    if let Some(w) = replay.as_mut() {
        let winner_agent = result.winner.map(|player| agents[player]);
        writeln!(
            w,
            "{}",
            serde_json::json!({"type":"result","result":result,"winner_agent":winner_agent})
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(result)
}

/// Run a match against a separately built TBP executable.
///
/// TBP has no in-game garbage update message. The external process keeps its
/// tree through ordinary `play`/`new_piece` messages and is restarted only
/// after a garbage rise changes its board. This is deliberately explicit:
/// both sides see the same board and randomizer state.
pub fn run_external(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    opponent: &Path,
    opponent_config: Option<&Path>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_external_with_rules(
        seed,
        swapped,
        limit,
        budget,
        weights,
        pps,
        RulesProfile::Arena,
        opponent,
        opponent_config,
        replay,
    )
}

pub fn run_external_with_rules(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    rules: RulesProfile,
    opponent: &Path,
    opponent_config: Option<&Path>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_external_mode(
        seed,
        swapped,
        limit,
        budget,
        weights,
        pps,
        rules,
        opponent,
        opponent_config,
        ExternalLocalAgent::Intetrigence,
        false,
        replay,
    )
}

/// Run an external TBP opponent with alternating, non-overlapping thinking
/// windows. The child process is suspended while the local searcher thinks,
/// then receives the same wall-clock window before its suggestion is read.
/// This is Unix-only at runtime because it uses process suspension signals.
pub fn run_external_strict(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    opponent: &Path,
    opponent_config: Option<&Path>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_external_strict_with_rules(
        seed,
        swapped,
        limit,
        budget,
        weights,
        pps,
        RulesProfile::Arena,
        opponent,
        opponent_config,
        replay,
    )
}

pub fn run_external_strict_with_rules(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    rules: RulesProfile,
    opponent: &Path,
    opponent_config: Option<&Path>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_external_mode(
        seed,
        swapped,
        limit,
        budget,
        weights,
        pps,
        rules,
        opponent,
        opponent_config,
        ExternalLocalAgent::Intetrigence,
        true,
        replay,
    )
}

/// Run Hoiko against an external TBP opponent such as Cold Clear 2.
pub fn run_hoiko_external_with_rules(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    pps: Option<f64>,
    rules: RulesProfile,
    config: &HoikoConfig,
    opponent: &Path,
    opponent_config: Option<&Path>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_external_mode(
        seed,
        swapped,
        limit,
        budget,
        None,
        pps,
        rules,
        opponent,
        opponent_config,
        ExternalLocalAgent::Hoiko(config.clone()),
        false,
        replay,
    )
}

/// Strict-time variant of [`run_hoiko_external_with_rules`].
pub fn run_hoiko_external_strict_with_rules(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    pps: Option<f64>,
    rules: RulesProfile,
    config: &HoikoConfig,
    opponent: &Path,
    opponent_config: Option<&Path>,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    run_external_mode(
        seed,
        swapped,
        limit,
        budget,
        None,
        pps,
        rules,
        opponent,
        opponent_config,
        ExternalLocalAgent::Hoiko(config.clone()),
        true,
        replay,
    )
}

fn run_external_mode(
    seed: u64,
    swapped: bool,
    limit: u32,
    budget: Budget,
    weights: Option<&serde_json::Value>,
    pps: Option<f64>,
    rules: RulesProfile,
    opponent: &Path,
    opponent_config: Option<&Path>,
    local_agent: ExternalLocalAgent,
    strict_external_time: bool,
    mut replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    if budget.milliseconds == 0 || budget.iterations != 0 {
        return Err(
            "external TBP evaluation requires --ms and does not support --iterations".into(),
        );
    }
    if pps.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err("pps must be a finite positive number".into());
    }
    let mut games = [
        Game::new_with_rules(seed, rules),
        Game::new_with_rules(seed ^ 0x9e3779b97f4a7c15, rules),
    ];
    let mut garbage = [
        ChaCha8Rng::seed_from_u64(seed ^ 12345),
        ChaCha8Rng::seed_from_u64(seed ^ 67890),
    ];
    let improved_player = usize::from(swapped);
    let external_player = 1 - improved_player;
    let local_name = local_agent.name();
    let agents = if swapped {
        ["cold_clear_2", local_name]
    } else {
        [local_name, "cold_clear_2"]
    };
    let mut local = LocalBackend::new(local_agent, weights);
    let mut external = TbpClient::spawn(opponent, opponent_config)?;
    if strict_external_time {
        external.start_paused(&games[external_player])?;
    } else {
        external.start(&games[external_player])?;
    }
    if let Some(w) = replay.as_mut() {
        writeln!(
            w,
            "{}",
            serde_json::json!({
                "type": "start",
                "replay_version": 2,
                "seed": seed,
                "swapped": swapped,
                "turn_limit": limit,
                "ms": budget.milliseconds,
                "iterations": budget.iterations,
                "pps": pps,
                "rules": rules.name(),
                "upstream": "ed8b19327b6bd1410ddd873d8611485bd45d8fae",
                "weights": weights,
                "external_opponent": opponent,
                "external_config": opponent_config,
                "strict_external_time": strict_external_time,
                "agents": agents,
                "initial": [
                    replay_initial_state(&games[0], agents[0]),
                    replay_initial_state(&games[1], agents[1]),
                ],
                "board_cells": "row_major_bottom_up; null|block|garbage",
            })
        )
        .map_err(|e| e.to_string())?;
    }
    let mut turns = 0;
    let mut external_needs_resync = false;
    let turn_period = pps.map(|value| Duration::from_secs_f64(1.0 / value));
    for turn in 0..limit {
        let turn_start = Instant::now();
        turns = turn + 1;
        // Hoiko's versus evaluator uses the opponent's visible board and
        // combo as a profile signal.  Keep this snapshot at the decision
        // boundary so a local Hoiko backend sees exactly the same state that
        // the referee presents to the external TBP process.
        let snapshots = [games[0].opponent_snapshot(), games[1].opponent_snapshot()];
        games[0].set_opponent_snapshot(snapshots[1]);
        games[1].set_opponent_snapshot(snapshots[0]);
        if external_needs_resync {
            if strict_external_time {
                external.resync_paused(&games[external_player])?;
            } else {
                external.resync(&games[external_player])?;
            }
            external_needs_resync = false;
        }
        let (improved_mv, improved_stats) = local.choose(&games[improved_player], budget);
        let (external_mv, external_nodes, external_elapsed_us) = if strict_external_time {
            external.choose_sliced(&games[external_player], budget.milliseconds)?
        } else {
            external.choose(&games[external_player], budget.milliseconds)?
        };
        let mut decisions = [
            (None, SearchStats::default()),
            (None, SearchStats::default()),
        ];
        decisions[improved_player] = (improved_mv, improved_stats);
        decisions[external_player] = (
            external_mv,
            SearchStats {
                nodes: external_nodes,
                iterations: 0,
                elapsed_us: external_elapsed_us,
                reused_tree: false,
            },
        );

        let mut sent = [0; 2];
        for (i, (mv, stats)) in decisions.into_iter().enumerate() {
            let active = games[i].queue.front().copied();
            let hold_before = games[i].hold;
            let next_before = games[i].queue.iter().skip(1).take(5).copied().collect();
            let hold_used = mv.is_some_and(|placement| Some(placement.location.piece) != active);
            let outcome = if let Some(mv) = mv {
                let before_external = (i == external_player).then(|| games[i].clone());
                let outcome = games[i].play(mv)?;
                sent[i] = outcome.sent;
                if let Some(before) = before_external {
                    external.advance(&before, mv, &games[i])?;
                    external_needs_resync = outcome.garbage_applied > 0;
                }
                Some(outcome)
            } else {
                games[i].dead = true;
                None
            };
            if let Some(w) = replay.as_mut() {
                writeln!(
                    w,
                    "{}",
                    replay_move_json(
                        turn,
                        i,
                        agents[i],
                        mv,
                        outcome,
                        stats,
                        active,
                        hold_before,
                        next_before,
                        hold_used,
                        &games[i],
                    )
                )
                .map_err(|e| e.to_string())?;
            }
        }
        games[0].receive(sent[1], &mut garbage[0]);
        games[1].receive(sent[0], &mut garbage[1]);
        if games.iter().any(|g| g.dead) {
            break;
        }
        if let Some(period) = turn_period {
            if let Some(remaining) = period.checked_sub(turn_start.elapsed()) {
                std::thread::sleep(remaining);
            }
        }
    }
    let winner = match (games[0].dead, games[1].dead) {
        (true, false) => Some(1),
        (false, true) => Some(0),
        _ => None,
    };
    let result = MatchResult {
        seed,
        swapped,
        winner,
        turns,
        pieces: [games[0].pieces, games[1].pieces],
        attack: [games[0].attack, games[1].attack],
        lines: [games[0].lines, games[1].lines],
    };
    if let Some(w) = replay.as_mut() {
        let winner_agent = result.winner.map(|player| agents[player]);
        writeln!(
            w,
            "{}",
            serde_json::json!({"type": "result", "result": result, "winner_agent": winner_agent})
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(result)
}

/// Run one of the supported pairings with the named participant in player 0.
///
/// A `ColdClear2` participant is launched through TBP.  Hoiko and
/// intetrigence are in-process backends, so the same function can be used for
/// either side of a paired seed run.  Exactly one external participant is
/// supported per match; this keeps the referee and process timing semantics
/// explicit.
pub fn run_matchup(
    seed: u64,
    player0: MatchAgent,
    player1: MatchAgent,
    limit: u32,
    budget: Budget,
    pps: Option<f64>,
    rules: RulesProfile,
    hoiko_config: &HoikoConfig,
    opponent: &Path,
    opponent_config: Option<&Path>,
    strict_external_time: bool,
    replay: Option<&mut dyn Write>,
) -> Result<MatchResult, String> {
    if player0 == player1 {
        return Err("matchup requires two different agents".into());
    }
    match (player0, player1) {
        (MatchAgent::Hoiko, MatchAgent::Intetrigence)
        | (MatchAgent::Intetrigence, MatchAgent::Hoiko) => {
            if strict_external_time {
                return Err("--strict-external-time requires cold_clear_2".into());
            }
            run_hoiko_with_pps_rules(
                seed,
                player1 == MatchAgent::Hoiko,
                limit,
                budget,
                pps,
                rules,
                hoiko_config,
                replay,
            )
        }
        (MatchAgent::Hoiko, MatchAgent::ColdClear2)
        | (MatchAgent::ColdClear2, MatchAgent::Hoiko) => {
            let swapped = player1 == MatchAgent::Hoiko;
            if strict_external_time {
                run_hoiko_external_strict_with_rules(
                    seed,
                    swapped,
                    limit,
                    budget,
                    pps,
                    rules,
                    hoiko_config,
                    opponent,
                    opponent_config,
                    replay,
                )
            } else {
                run_hoiko_external_with_rules(
                    seed,
                    swapped,
                    limit,
                    budget,
                    pps,
                    rules,
                    hoiko_config,
                    opponent,
                    opponent_config,
                    replay,
                )
            }
        }
        (MatchAgent::Intetrigence, MatchAgent::ColdClear2)
        | (MatchAgent::ColdClear2, MatchAgent::Intetrigence) => {
            let swapped = player1 == MatchAgent::Intetrigence;
            if strict_external_time {
                run_external_strict_with_rules(
                    seed,
                    swapped,
                    limit,
                    budget,
                    None,
                    pps,
                    rules,
                    opponent,
                    opponent_config,
                    replay,
                )
            } else {
                run_external_with_rules(
                    seed,
                    swapped,
                    limit,
                    budget,
                    None,
                    pps,
                    rules,
                    opponent,
                    opponent_config,
                    replay,
                )
            }
        }
        (MatchAgent::ColdClear2, MatchAgent::ColdClear2)
        | (MatchAgent::Hoiko, MatchAgent::Hoiko)
        | (MatchAgent::Intetrigence, MatchAgent::Intetrigence) => {
            unreachable!("same-agent pairings are rejected above")
        }
    }
}

pub fn wilson(wins: u32, n: u32) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let n = n as f64;
    let p = wins as f64 / n;
    let z = 1.959963984540054;
    let center = (p + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / (1.0 + z * z / n);
    (center - half, center + half)
}

/// Conservative one-sided 95% lower bound for mean victory rate.
/// Each independent seed pair is one bounded [0,1] observation.
/// Draws count as non-wins. Valid for a fixed sample size, not repeated peeking.
pub fn paired_victory_bound(matches: &[MatchResult]) -> Result<serde_json::Value, String> {
    if matches.is_empty() || !matches.len().is_multiple_of(2) {
        return Err("complete seed pairs required".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut wins = 0u32;
    for pair in matches.chunks_exact(2) {
        if pair[0].seed != pair[1].seed
            || pair[0].swapped == pair[1].swapped
            || !seen.insert(pair[0].seed)
        {
            return Err("each distinct seed must have exactly one match per side".into());
        }
        for m in pair {
            if m.winner.is_some_and(|p| (p == 0) != m.swapped) {
                wins += 1;
            }
        }
    }
    let rate = wins as f64 / matches.len() as f64;
    let lower = (rate - (20.0f64.ln() / (2.0 * seen.len() as f64)).sqrt()).max(0.0);
    Ok(
        serde_json::json!({"independent_pairs":seen.len(),"victory_rate":rate,"one_sided_95_lower":lower,"method":"Hoeffding across seed pairs; draws are non-wins; fixed sample size required","above_half":lower>0.5}),
    )
}

/// The same paired bound for a caller that names the target participant
/// directly. Each tuple is `(seed, target_won)` and the caller must provide
/// two opposite-side matches per seed; `None` records a draw.
pub fn paired_victory_bound_for(
    outcomes: &[(u64, Option<bool>)],
) -> Result<serde_json::Value, String> {
    if outcomes.is_empty() || !outcomes.len().is_multiple_of(2) {
        return Err("complete seed pairs required".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut wins = 0u32;
    for pair in outcomes.chunks_exact(2) {
        if pair[0].0 != pair[1].0 || !seen.insert(pair[0].0) {
            return Err("each distinct seed must have exactly one pair".into());
        }
        wins += pair
            .iter()
            .filter(|(_, result)| *result == Some(true))
            .count() as u32;
    }
    let observations = outcomes.len() as f64;
    let rate = wins as f64 / observations;
    let lower = (rate - (20.0f64.ln() / (2.0 * seen.len() as f64)).sqrt()).max(0.0);
    Ok(serde_json::json!({
        "independent_pairs": seen.len(),
        "victory_rate": rate,
        "one_sided_95_lower": lower,
        "method": "Hoeffding across seed pairs; draws are non-wins; fixed sample size required",
        "above_half": lower > 0.5,
    }))
}
