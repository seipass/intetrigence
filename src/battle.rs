use crate::{
    arena::MatchAgent,
    game::{Game, Outcome, RulesProfile},
    hoiko::{HoikoConfig, HoikoSearcher},
    search::{Budget, SearchStats, Searcher},
    tbp_client::TbpClient,
    timing::{plan_placement, trace_movement, MovementPlan, MovementTraceFrame, TimingProfile},
};
use intetrigence_engine::data::{Board, Piece, Placement};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleResult {
    pub seed: u64,
    pub swapped: bool,
    pub winner: Option<usize>,
    pub frames: u64,
    pub pieces: [u32; 2],
    pub attack: [u32; 2],
    pub lines: [u32; 2],
}

enum Backend {
    Intetrigence(Searcher),
    Hoiko(HoikoSearcher),
    ColdClear2 {
        client: TbpClient,
        strict_time: bool,
        needs_resync: bool,
    },
}

impl Backend {
    fn new(
        agent: MatchAgent,
        game: &Game,
        weights: Option<&serde_json::Value>,
        hoiko_config: &HoikoConfig,
        opponent: &Path,
        opponent_config: Option<&Path>,
        strict_external_time: bool,
    ) -> Result<Self, String> {
        match agent {
            MatchAgent::Intetrigence => Ok(Self::Intetrigence(Searcher::new(true, weights))),
            MatchAgent::Hoiko => Ok(Self::Hoiko(HoikoSearcher::new(hoiko_config.clone()))),
            MatchAgent::ColdClear2 => {
                let mut client = TbpClient::spawn(opponent, opponent_config)?;
                if strict_external_time {
                    client.start_paused(game)?;
                } else {
                    client.start(game)?;
                }
                Ok(Self::ColdClear2 {
                    client,
                    strict_time: strict_external_time,
                    needs_resync: false,
                })
            }
        }
    }

    fn choose(
        &mut self,
        game: &Game,
        budget: Budget,
    ) -> Result<(Option<Placement>, SearchStats), String> {
        match self {
            Self::Intetrigence(searcher) => Ok(searcher.choose(game, budget)),
            Self::Hoiko(searcher) => {
                let (placement, stats) = searcher.choose(game, budget);
                Ok((
                    placement,
                    SearchStats {
                        nodes: stats.nodes,
                        iterations: stats.depth as u64,
                        elapsed_us: stats.elapsed_us,
                        reused_tree: false,
                    },
                ))
            }
            Self::ColdClear2 {
                client,
                strict_time,
                needs_resync,
            } => {
                if *needs_resync {
                    if *strict_time {
                        client.resync_paused(game)?;
                    } else {
                        client.resync(game)?;
                    }
                    *needs_resync = false;
                }
                let (placement, nodes, elapsed_us) = if *strict_time {
                    client.choose_sliced(game, budget.milliseconds)?
                } else {
                    client.choose(game, budget.milliseconds)?
                };
                Ok((
                    placement,
                    SearchStats {
                        nodes,
                        iterations: 0,
                        elapsed_us,
                        reused_tree: false,
                    },
                ))
            }
        }
    }

    fn advance(
        &mut self,
        before: &Game,
        placement: Placement,
        after: &Game,
        outcome: &Outcome,
    ) -> Result<(), String> {
        if let Self::ColdClear2 {
            client,
            needs_resync,
            ..
        } = self
        {
            client.advance(before, placement, after)?;
            *needs_resync = outcome.garbage_applied > 0;
        }
        Ok(())
    }
}

struct ScheduledLock {
    frame: u64,
    placement: Placement,
    plan: MovementPlan,
    stats: SearchStats,
    active: Piece,
    hold_before: Option<Piece>,
    next_before: Vec<Piece>,
}
#[derive(Clone, Copy)]
enum VisualCell {
    Empty,
    Block(Piece),
    Garbage,
}

#[derive(Clone)]
struct VisualBoard {
    cells: [[VisualCell; 10]; 40],
}

impl Default for VisualBoard {
    fn default() -> Self {
        Self {
            cells: [[VisualCell::Empty; 10]; 40],
        }
    }
}

impl VisualBoard {
    fn apply_lock(&mut self, placement: Placement, garbage: Board, garbage_applied: u32) {
        for (x, y) in placement.location.cells() {
            if (0..10).contains(&x) && (0..40).contains(&y) {
                self.cells[y as usize][x as usize] = VisualCell::Block(placement.location.piece);
            }
        }
        self.remove_full_rows();
        self.add_garbage(garbage, garbage_applied as usize);
    }

    fn remove_full_rows(&mut self) {
        let mut compacted = [[VisualCell::Empty; 10]; 40];
        let mut target = 0;
        for row in 0..40 {
            if self.cells[row]
                .iter()
                .all(|cell| !matches!(cell, VisualCell::Empty))
            {
                continue;
            }
            if target < 40 {
                compacted[target] = self.cells[row];
                target += 1;
            }
        }
        self.cells = compacted;
    }

    fn add_garbage(&mut self, garbage: Board, count: usize) {
        let count = count.min(40);
        if count == 0 {
            return;
        }
        for row in (count..40).rev() {
            self.cells[row] = self.cells[row - count];
        }
        for row in 0..count {
            for column in 0..10 {
                self.cells[row][column] = if garbage.cols[column] & (1u64 << row) != 0 {
                    VisualCell::Garbage
                } else {
                    VisualCell::Empty
                };
            }
        }
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!(self
            .cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| match cell {
                        VisualCell::Empty => serde_json::Value::Null,
                        VisualCell::Garbage => serde_json::json!("garbage"),
                        VisualCell::Block(piece) => serde_json::to_value(piece).unwrap(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_matchup(
    seed: u64,
    swapped: bool,
    player0: MatchAgent,
    player1: MatchAgent,
    piece_limit: u32,
    budget: Budget,
    rules: RulesProfile,
    weights: Option<&serde_json::Value>,
    hoiko_config: &HoikoConfig,
    opponent: &Path,
    opponent_config: Option<&Path>,
    strict_external_time: bool,
    mut replay: Option<&mut dyn Write>,
) -> Result<BattleResult, String> {
    if piece_limit == 0 {
        return Err("battle piece limit must be positive".into());
    }
    if [player0, player1].contains(&MatchAgent::ColdClear2)
        && (budget.milliseconds == 0 || budget.iterations != 0)
    {
        return Err(
            "cold_clear_2 movement battles require --ms and do not support --iterations".into(),
        );
    }

    let agents = [player0.name(), player1.name()];
    let mut games = [
        Game::new_with_rules(seed, rules),
        Game::new_with_rules(seed ^ 0x9e3779b97f4a7c15, rules),
    ];
    let mut garbage = [
        ChaCha8Rng::seed_from_u64(seed ^ 12345),
        ChaCha8Rng::seed_from_u64(seed ^ 67890),
    ];
    let mut visual = [VisualBoard::default(), VisualBoard::default()];
    let mut backends = [
        Backend::new(
            player0,
            &games[0],
            weights,
            hoiko_config,
            opponent,
            opponent_config,
            strict_external_time,
        )?,
        Backend::new(
            player1,
            &games[1],
            weights,
            hoiko_config,
            opponent,
            opponent_config,
            strict_external_time,
        )?,
    ];
    let profile = TimingProfile::guideline_level(1);
    let mut scheduled: [Option<ScheduledLock>; 2] = [None, None];
    let mut frame = 0u64;

    if let Some(writer) = replay.as_mut() {
        writeln!(
            writer,
            "{}",
            serde_json::json!({
                "type": "battle_start",
                "replay_version": 3,
                "seed": seed,
                "swapped": swapped,
                "piece_limit": piece_limit,
                "ms": budget.milliseconds,
                "iterations": budget.iterations,
                "rules": rules.name(),
                "agents": agents,
                "clock_hz": 60,
                "movement": "frame_inputs",
                "movement_trace": "active_placement_after_each_input",
                "initial": [initial_state(&games[0], agents[0]), initial_state(&games[1], agents[1])],
                "board_cells": "row_major_bottom_up; null|block|garbage",
                "board_colors": "row_major_bottom_up; null|I|O|T|S|Z|J|L|garbage",
            })
        )
        .map_err(|error| error.to_string())?;
    }

    for player in 0..2 {
        schedule_player(
            player,
            frame,
            &mut games,
            &mut backends,
            &mut scheduled,
            budget,
            profile,
        )?;
    }

    while !games.iter().any(|game| game.dead)
        && games.iter().map(|game| game.pieces).max().unwrap_or(0) < piece_limit
    {
        let Some(next_frame) = scheduled.iter().flatten().map(|event| event.frame).min() else {
            break;
        };
        frame = next_frame;
        let locking = [
            scheduled[0]
                .as_ref()
                .is_some_and(|event| event.frame == frame),
            scheduled[1]
                .as_ref()
                .is_some_and(|event| event.frame == frame),
        ];
        let mut sent = [0u32; 2];

        for player in 0..2 {
            if !locking[player] {
                continue;
            }
            let event = scheduled[player].take().expect("locking event exists");
            let before = games[player].clone();
            let movement_trace = trace_movement(
                before.board,
                event.placement,
                event.plan.hold_used,
                profile,
                &event.plan.inputs,
            )
            .map_err(|error| format!("failed to trace movement: {error:?}"))?;
            let outcome = games[player].play(event.placement)?;
            sent[player] = outcome.sent;
            visual[player].apply_lock(
                event.placement,
                games[player].garbage_board(),
                outcome.garbage_applied,
            );
            backends[player].advance(&before, event.placement, &games[player], &outcome)?;
            if let Some(writer) = replay.as_mut() {
                writeln!(
                    writer,
                    "{}",
                    move_record(
                        player,
                        agents[player],
                        frame,
                        event,
                        outcome,
                        &games[player],
                        &visual[player],
                        &movement_trace,
                    )
                )
                .map_err(|error| error.to_string())?;
            }
        }

        games[0].receive(sent[1], &mut garbage[0]);
        games[1].receive(sent[0], &mut garbage[1]);
        if games.iter().any(|game| game.dead)
            || games.iter().map(|game| game.pieces).max().unwrap_or(0) >= piece_limit
        {
            break;
        }
        for player in 0..2 {
            if locking[player] {
                schedule_player(
                    player,
                    frame,
                    &mut games,
                    &mut backends,
                    &mut scheduled,
                    budget,
                    profile,
                )?;
            }
        }
    }

    let result = BattleResult {
        seed,
        swapped,
        winner: match (games[0].dead, games[1].dead) {
            (true, false) => Some(1),
            (false, true) => Some(0),
            _ => None,
        },
        frames: frame,
        pieces: [games[0].pieces, games[1].pieces],
        attack: [games[0].attack, games[1].attack],
        lines: [games[0].lines, games[1].lines],
    };
    if let Some(writer) = replay.as_mut() {
        writeln!(
            writer,
            "{}",
            serde_json::json!({
                "type": "battle_result",
                "result": result,
                "dead": [games[0].dead, games[1].dead],
                "winner_agent": result.winner.map(|player| agents[player]),
            })
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn schedule_player(
    player: usize,
    frame: u64,
    games: &mut [Game; 2],
    backends: &mut [Backend; 2],
    scheduled: &mut [Option<ScheduledLock>; 2],
    budget: Budget,
    profile: TimingProfile,
) -> Result<(), String> {
    let opponent = games[1 - player].opponent_snapshot();
    games[player].set_opponent_snapshot(opponent);
    let active = games[player]
        .queue
        .front()
        .copied()
        .ok_or("empty queue while scheduling movement")?;
    let hold_before = games[player].hold;
    let next_before = games[player]
        .queue
        .iter()
        .skip(1)
        .take(5)
        .copied()
        .collect();
    let (placement, stats) = backends[player].choose(&games[player], budget)?;
    let Some(placement) = placement else {
        games[player].dead = true;
        return Ok(());
    };
    let hold_used = placement.location.piece != active;
    let plan =
        plan_placement(games[player].board, placement, hold_used, profile).map_err(|error| {
            format!(
                "{player} selected a placement without a human input path: {error:?}; placement={placement:?}; board={:?}",
                games[player].board.cols
            )
        })?;
    scheduled[player] = Some(ScheduledLock {
        frame: frame + plan.inputs.len() as u64,
        placement,
        plan,
        stats,
        active,
        hold_before,
        next_before,
    });
    Ok(())
}

fn initial_state(game: &Game, agent: &str) -> serde_json::Value {
    serde_json::json!({
        "agent": agent,
        "active": game.queue.front().copied(),
        "hold": game.hold,
        "next": game.queue.iter().skip(1).take(5).copied().collect::<Vec<_>>(),
    })
}

fn move_record(
    player: usize,
    agent: &str,
    frame: u64,
    event: ScheduledLock,
    outcome: Outcome,
    after: &Game,
    visual: &VisualBoard,
    movement_trace: &[MovementTraceFrame],
) -> serde_json::Value {
    serde_json::json!({
        "type": "battle_move",
        "frame": frame,
        "player": player,
        "agent": agent,
        "move": event.placement,
        "inputs": event.plan.inputs,
        "movement_frames": event.plan.inputs.len(),
        "active": event.active,
        "hold_used": event.plan.hold_used,
        "hold_before": event.hold_before,
        "next_before": event.next_before,
        "hold_after": after.hold,
        "next_after": after.queue.iter().take(5).copied().collect::<Vec<_>>(),
        "outcome": outcome,
        "stats": event.stats,
        "board": after.board.cols,
        "board_cells": after.board_cells_json(),
        "board_colors": visual.json(),
        "movement_trace": movement_trace,
        "pending": after.pending,
    })
}
