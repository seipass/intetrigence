//! Round runner: paired turns, seat order, adoption, replay records and audit.
//!
//! Implements `docs/s2-referee-protocol.md` §2.3 / §3.2:
//!
//! ```text
//! for t in 0..turn_cap:
//!     frame = t * 12
//!     piece = bag[t]                      # per-player bag, same piece_seed
//!     for seat in [1, 2]:
//!         move = ask_bot(slot, view)
//!         verify + adopt                  # first reachable candidate
//!         apply (hold, path, lock, score, cancel, send, insert)
//!         if topped_out: end
//! ```
//!
//! The runner never reads the clock for game state; `frame` is derived from the
//! turn number. Wall-clock time is only recorded for the audit.

use std::collections::BTreeMap;

use crate::agent::{Agent, AgentAudit, Commit, RoundContext};
use crate::audit::{
    FirstChoiceAudit, GameAudit, ScoringAudit, StartResendAudit, ThinkStats, attax_audit,
};
use crate::engine::{Engine, Profile, Sheet, board_hash};
use crate::replay::{
    BotRecord, ForfeitCause, GarbageRecord, HashRecord, Header, PlacementRecord, PlayerRecord,
    ReplayRecord, ResultRecord, SCHEMA, TurnRecord, piece_name, spin_name,
};

#[derive(Clone, Debug)]
pub struct RoundConfig {
    pub seed: u64,
    pub sheet: Sheet,
    pub turn_cap: u32,
    pub frames_per_turn: u64,
    pub rules: String,
    pub protocol: String,
    pub started_at: String,
    /// Match profile (protocol v1.11 §1.3). `Official` is the empty board.
    pub profile: Profile,
    /// Deterministic search (fixed schedule, no wall-clock deadline).
    pub deterministic: bool,
    /// Worker threads requested for the search (recorded in the header).
    pub threads: u32,
}

impl RoundConfig {
    pub fn new(seed: u64, sheet: Sheet) -> Self {
        Self {
            seed,
            sheet,
            turn_cap: 1000,
            frames_per_turn: 12,
            rules: "docs/s2-spec.md@v1.3.17".into(),
            protocol: "docs/s2-referee-protocol.md@v1.54".into(),
            deterministic: false,
            threads: 8,
            started_at: String::new(),
            profile: Profile::Official,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerSummary {
    pub slot: usize,
    pub seat: u8,
    pub sent_total: u32,
    pub received_total: u32,
    pub attack_events: u32,
    pub attack_total: u32,
    pub max_combo: u32,
    pub max_b2b: u32,
    pub surge_released: u32,
    pub pieces_placed: u32,
    pub lines_cleared: u32,
    pub holds: u32,
    pub topped_out: bool,
}

#[derive(Clone, Debug)]
pub struct RoundOutcome {
    pub winner: Option<usize>,
    pub reason: String,
    pub detail: Option<String>,
    pub forfeit_cause: Option<ForfeitCause>,
    /// Slot that forfeited (no-contest); `None` otherwise.
    pub forfeit_slot: Option<u8>,
    pub turns: u32,
    pub final_frame: u64,
    pub per_player: [PlayerSummary; 2],
}

#[derive(Clone, Debug)]
pub struct RoundReport {
    pub outcome: RoundOutcome,
    pub records: Vec<ReplayRecord>,
    pub audit: GameAudit,
    pub agent_audits: [AgentAudit; 2],
}

#[derive(Clone, Debug)]
pub enum RoundError {
    /// Referee invariant violation: abort the match and fix the referee
    /// (`docs/s2-referee-protocol.md` §7).
    Referee(String),
}

impl std::fmt::Display for RoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoundError::Referee(message) => write!(f, "referee bug: {message}"),
        }
    }
}

impl std::error::Error for RoundError {}

enum End {
    TopOut {
        loser: usize,
    },
    /// No legal placement with the current piece or hold (protocol v1.6 §7).
    /// Not an S2 rule: a live active piece is reachable from its spawn, so this
    /// branch is defensive and unreachable. Recorded as `Stuck` with
    /// `winner = None` (unresolved, protocol v1.27 §7.1.3.1).
    Stuck {
        #[allow(dead_code)]
        loser: usize,
    },
    TurnCap,
    Forfeit {
        slot: usize,
        detail: String,
        cause: ForfeitCause,
    },
}

pub fn run_round(
    config: &RoundConfig,
    agents: &mut [&mut dyn Agent; 2],
) -> Result<RoundReport, RoundError> {
    let start_garbage = config.profile.start_garbage(config.seed);
    let mut engine = Engine::with_start_garbage(
        config.seed,
        config.sheet,
        config.frames_per_turn,
        &start_garbage,
    );
    engine.begin_turn();

    for slot in 0..2 {
        let context = RoundContext {
            seed: config.seed,
            sheet: config.sheet,
            slot,
            seat: config.sheet.seat_of(slot),
            initial: engine.view(slot),
        };
        if let Err(error) = agents[slot].begin_round(&context) {
            return Ok(forfeit_report(config, agents, slot, error.to_string()));
        }
    }

    let bots = [
        BotRecord::from(&agents[0].descriptor()),
        BotRecord::from(&agents[1].descriptor()),
    ];
    let mut records = vec![ReplayRecord::Header(header(config, &bots, &start_garbage))];

    let mut think: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    let mut indices: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    let mut scoring: [ScoringAudit; 2] = [ScoringAudit::default(), ScoringAudit::default()];
    let mut reachability_rejections = 0u32;
    let mut heights: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    let mut b2b_turns = [0u32; 2];

    let mut end: Option<End> = None;
    let mut turns_played = 0u32;

    let mut search: [Vec<crate::audit::SearchTurn>; 2] = [Vec::new(), Vec::new()];
    'turns: for t in 0..config.turn_cap {
        engine.turn = t;
        engine.begin_turn();
        turns_played = t + 1;

        // Diagnostic garbage pressure: inject a packet to both players at the
        // start of the turn and record it before the turn record so the
        // verifier replays the same injection order.
        if let Some(rows) = config.profile.injection_amount(t) {
            for slot in 0..2 {
                engine.players[slot].garbage.receive(rows, engine.frame);
                records.push(ReplayRecord::Garbage(GarbageRecord {
                    t,
                    from: 2,
                    to: slot as u8,
                    amount: rows,
                    arrival_frame: engine.frame,
                }));
            }
        }

        heights[0].push(engine.players[0].game.board().height() as u32);
        heights[1].push(engine.players[1].game.board().height() as u32);

        let mut turn_record = TurnRecord {
            t,
            frame: engine.frame,
            placements: Vec::new(),
        };
        let mut garbage_records: Vec<GarbageRecord> = Vec::new();

        for seat_index in 0..2 {
            let slot = engine.order[seat_index];
            let view = engine.view(slot);
            let suggestion = match agents[slot].suggest(&view) {
                Ok(suggestion) => suggestion,
                Err(error) => {
                    end = Some(End::Forfeit {
                        slot,
                        detail: error.to_string(),
                        cause: ForfeitCause::Other,
                    });
                    push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
                    break 'turns;
                }
            };
            think[slot].push(suggestion.think_ms);
            search[slot].push(crate::audit::SearchTurn {
                turn: t,
                depth: suggestion.depth,
                width: suggestion.width,
                nodes: suggestion.nodes,
                fallback: suggestion.fallback,
                aborted: suggestion.aborted,
            });

            let adopted =
                match engine.adopt(slot, &suggestion, agents[slot].descriptor().supports_180) {
                    Ok(adopted) => adopted,
                    Err(crate::engine::AdoptError::NoMoves) => {
                        end = Some(End::Stuck { loser: slot });
                        push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
                        break 'turns;
                    }
                    Err(error) => {
                        reachability_rejections += 1;
                        let cause = classify_forfeit(
                            &engine,
                            slot,
                            agents[slot].descriptor().visible_rows,
                            agents[slot].descriptor().supports_180,
                        );
                        end = Some(End::Forfeit {
                            slot,
                            detail: error.to_string(),
                            cause,
                        });
                        push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
                        break 'turns;
                    }
                };
            indices[slot].push(adopted.candidate_index as u32);

            let applied = match engine.apply(slot, &adopted.target, adopted.claimed_spin) {
                Ok(applied) => applied,
                Err(error) => {
                    return Err(RoundError::Referee(format!(
                        "t{t} slot{slot}: adopt succeeded but apply failed: {error}"
                    )));
                }
            };

            if applied.b2b_after >= 4 {
                b2b_turns[slot] += 1;
            }
            scoring[slot].record(
                t,
                slot as u8,
                adopted.claimed_spin,
                applied.spin,
                applied.declared_attack,
                applied.attack_total,
            );

            turn_record.placements.push(PlacementRecord {
                slot: slot as u8,
                piece: piece_name(applied.piece).to_string(),
                rotation: applied.rotation as u8,
                x: applied.x,
                y: applied.y,
                hold: applied.hold,
                spin: spin_name(applied.spin).to_string(),
                lines: applied.lines,
                garbage_cleared: applied.garbage_cleared,
                perfect_clear: applied.perfect_clear,
                events: applied.events.clone(),
                sent: applied.sent,
                cancelled: applied.cancelled,
                attack: applied.attack_total,
                combo_before: applied.combo_before,
                b2b_before: applied.b2b_before,
                combo_after: applied.combo_after,
                b2b_after: applied.b2b_after,
                multiplier: applied.multiplier,
                think_ms: round3(suggestion.think_ms),
                candidate_index: adopted.candidate_index as u32,
                claimed_spin: adopted.claimed_spin.map(|spin| spin_name(spin).to_string()),
                declared_attack: applied.declared_attack,
            });
            for (amount, arrival_frame) in &applied.packets {
                garbage_records.push(GarbageRecord {
                    t,
                    from: slot as u8,
                    to: (1 - slot) as u8,
                    amount: *amount,
                    arrival_frame: *arrival_frame,
                });
            }

            let mut queue = Vec::with_capacity(6);
            queue.push(
                engine.players[slot]
                    .game
                    .active()
                    .map(|piece| piece.kind)
                    .unwrap_or(applied.piece),
            );
            queue.extend(engine.players[slot].game.preview(5));
            let commit = Commit {
                turn: t,
                frame: engine.frame,
                piece: applied.piece,
                rotation: applied.rotation,
                x: applied.x,
                y: applied.y,
                hold: applied.hold,
                spin: applied.spin,
                lines: applied.lines,
                events: applied.events.clone(),
                sent: applied.sent,
                cancelled: applied.cancelled,
                board: engine.players[slot].game.board().clone(),
                board_hash: applied.board_hash,
                queue,
                bag_state: crate::engine::tail_bag_state(&mut engine.players[slot].game),
                hold_piece: engine.players[slot].game.held_piece(),
                combo: applied.combo_after,
                b2b_streak: applied.b2b_after,
                garbage_inserted: !applied.inserted.is_empty(),
                topped_out: applied.topped_out,
            };
            if let Err(error) = agents[slot].commit(&commit) {
                end = Some(End::Forfeit {
                    slot,
                    detail: error.to_string(),
                    cause: ForfeitCause::Other,
                });
                push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
                break 'turns;
            }

            if applied.topped_out {
                end = Some(End::TopOut { loser: slot });
                push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
                break 'turns;
            }
        }

        push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
    }

    let end = end.unwrap_or(End::TurnCap);
    let (winner, reason, detail, forfeit_cause) = match &end {
        End::TopOut { loser } => (Some(1 - loser), "TopOut".to_string(), None, None),
        // "No legal placement" is not an S2 rule: a live active piece is by
        // definition reachable from its spawn, so this branch is defensive and
        // unreachable. It is recorded as unresolved (no winner) rather than
        // inventing a TopOut; the gate rejects any game that is not a TopOut
        // decision, so a stuck game fails the round.
        End::Stuck { loser } => (
            None,
            "Stuck".to_string(),
            Some(format!("unresolved: no legal placement (slot {loser})")),
            None,
        ),
        // A turn-cap is not a decision. TETR.IO has no "higher attack wins on
        // timeout" rule: a versus game ends when a player tops out. Reporting a
        // winner here would turn a stalemate into a fabricated result, so the
        // game is recorded as unresolved (no winner) and the round gate treats
        // any turn-cap as a failure.
        End::TurnCap => (
            None,
            "TurnCap".to_string(),
            Some("unresolved: turn cap reached".to_string()),
            None,
        ),
        End::Forfeit {
            slot: _,
            detail,
            cause,
        } => (
            None,
            "Forfeit".to_string(),
            Some(detail.clone()),
            Some(*cause),
        ),
    };
    let forfeit_slot = match &end {
        End::Forfeit { slot, .. } => Some(*slot as u8),
        _ => None,
    };

    let per_player = [summarize(&engine, 0), summarize(&engine, 1)];
    let outcome = RoundOutcome {
        winner,
        reason: reason.clone(),
        detail: detail.clone(),
        forfeit_cause,
        forfeit_slot,
        turns: turns_played,
        final_frame: engine.frame,
        per_player: per_player.clone(),
    };

    records.push(ReplayRecord::Result(ResultRecord {
        winner: winner.map(|slot| slot as u8),
        reason,
        detail,
        forfeit_cause,
        forfeit_slot,
        turns: turns_played,
        final_frame: engine.frame,
        per_player: per_player.iter().map(player_record).collect(),
    }));

    for agent in agents.iter_mut() {
        let _ = agent.end_round();
    }
    let agent_audits = [agents[0].audit(), agents[1].audit()];

    let start_resends = agent_audits
        .iter()
        .flat_map(|audit| audit.start_resends.iter())
        .map(|(reason, hash, turn)| StartResendAudit {
            reason: reason.clone(),
            board_hash: *hash,
            turn: *turn,
        })
        .collect();

    let attax = agent_audits[1]
        .command_line
        .as_ref()
        .map(|_| attax_audit(&agent_audits[1], bots[1].commit.clone()));

    let mut opener_lines = [0u32; 2];
    let mut opener_placements = [0u32; 2];
    let mut spin_placements = [0u32; 2];
    for record in &records {
        if let ReplayRecord::Turn(turn) = record {
            for placement in &turn.placements {
                let slot = placement.slot as usize;
                if slot < 2 {
                    if placement.spin != "none" {
                        spin_placements[slot] += 1;
                    }
                    if opener_placements[slot] < 14 {
                        opener_lines[slot] += u32::from(placement.lines);
                        opener_placements[slot] += 1;
                    }
                }
            }
        }
    }
    let audit = GameAudit {
        seed: config.seed,
        sheet: config.sheet.as_str().to_string(),
        started_at: config.started_at.clone(),
        profile: config.profile.as_str(),
        winner: winner.map(|slot| slot as u8),
        reason: outcome.reason.clone(),
        detail: outcome.detail.clone(),
        forfeit_cause,
        forfeit_slot,
        turns: turns_played,
        surge_released: [per_player[0].surge_released, per_player[1].surge_released],
        max_b2b_streak: [per_player[0].max_b2b, per_player[1].max_b2b],
        turns_with_b2b_ge_4: b2b_turns,
        heights: [
            crate::audit::HeightStats::from_samples(&heights[0]),
            crate::audit::HeightStats::from_samples(&heights[1]),
        ],
        totals: [
            crate::audit::PlayerTotals {
                sent_total: per_player[0].sent_total,
                received_total: per_player[0].received_total,
                attack_total: per_player[0].attack_total,
                pieces_placed: per_player[0].pieces_placed,
                lines_cleared: per_player[0].lines_cleared,
            },
            crate::audit::PlayerTotals {
                sent_total: per_player[1].sent_total,
                received_total: per_player[1].received_total,
                attack_total: per_player[1].attack_total,
                pieces_placed: per_player[1].pieces_placed,
                lines_cleared: per_player[1].lines_cleared,
            },
        ],
        think: [
            ThinkStats::from_samples(&think[0]),
            ThinkStats::from_samples(&think[1]),
        ],
        first_choice: [
            FirstChoiceAudit::from_indices(&indices[0]),
            FirstChoiceAudit::from_indices(&indices[1]),
        ],
        scoring,
        reachability_rejections,
        start_resends,
        attax,
        bots,
        replay: String::new(),
        verify: crate::replay::VerifyReport::default(),
        queue_invariant_violations: agent_audits[1].queue_invariant_violations,
        bot_fallbacks: agent_audits[0].clutch_fallbacks,
        bot_nodes_total: agent_audits[0].nodes_total,
        search,
        opener_lines,
        spin_placements,
        repeat: 0,
    };

    Ok(RoundReport {
        outcome,
        records,
        audit,
        agent_audits,
    })
}

fn header(config: &RoundConfig, bots: &[BotRecord; 2], start_garbage: &[u8]) -> Header {
    let mut slots = BTreeMap::new();
    // The slot name is the bot that actually played: hard-coding `attax` made
    // every CC2/clys replay lie about its opponent (and the renderer showed it).
    slots.insert("0".to_string(), bots[0].name.clone());
    slots.insert("1".to_string(), bots[1].name.clone());
    let mut seats = BTreeMap::new();
    seats.insert("1".to_string(), config.sheet.order()[0] as u8);
    seats.insert("2".to_string(), config.sheet.order()[1] as u8);
    let mut bot_map = BTreeMap::new();
    bot_map.insert("0".to_string(), bots[0].clone());
    bot_map.insert("1".to_string(), bots[1].clone());
    Header {
        schema: SCHEMA.to_string(),
        seed: config.seed,
        sheet: config.sheet.as_str().to_string(),
        slots,
        seats,
        rules: config.rules.clone(),
        protocol: config.protocol.clone(),
        bots: bot_map,
        started_at: config.started_at.clone(),
        turn_cap: config.turn_cap,
        frames_per_turn: config.frames_per_turn,
        preview: 5,
        profile: config.profile.as_str(),
        start_garbage: start_garbage.to_vec(),
        deterministic: config.deterministic,
        threads: config.threads,
    }
}

fn push_turn_end(
    records: &mut Vec<ReplayRecord>,
    t: u32,
    turn_record: TurnRecord,
    garbage: Vec<GarbageRecord>,
    engine: &Engine,
) {
    records.push(ReplayRecord::Turn(turn_record));
    records.extend(garbage.into_iter().map(ReplayRecord::Garbage));
    records.push(ReplayRecord::Hash(HashRecord {
        t,
        boards: [
            board_hash(engine.players[0].game.board()),
            board_hash(engine.players[1].game.board()),
        ],
    }));
}

fn summarize(engine: &Engine, slot: usize) -> PlayerSummary {
    let player = &engine.players[slot];
    let stats = player.game.stats();
    PlayerSummary {
        slot,
        seat: engine.sheet.seat_of(slot),
        sent_total: player.sent_total(),
        received_total: player.received_total(),
        attack_events: player.attack_events,
        attack_total: player.attack_total,
        max_combo: player.max_combo,
        max_b2b: player.max_b2b,
        surge_released: player.surge_released,
        pieces_placed: stats.pieces_placed,
        lines_cleared: stats.lines_cleared,
        holds: stats.holds,
        topped_out: player.game.is_over(),
    }
}

fn player_record(summary: &PlayerSummary) -> PlayerRecord {
    PlayerRecord {
        slot: summary.slot as u8,
        seat: summary.seat,
        sent_total: summary.sent_total,
        received_total: summary.received_total,
        attack_events: summary.attack_events,
        attack_total: summary.attack_total,
        max_combo: summary.max_combo,
        max_b2b: summary.max_b2b,
        surge_released: summary.surge_released,
        pieces_placed: summary.pieces_placed,
        lines_cleared: summary.lines_cleared,
        holds: summary.holds,
        topped_out: summary.topped_out,
    }
}

/// A round that never started: the bot could not spawn or handshake.
fn forfeit_report(
    config: &RoundConfig,
    agents: &mut [&mut dyn Agent; 2],
    slot: usize,
    detail: String,
) -> RoundReport {
    let start_garbage = config.profile.start_garbage(config.seed);
    let engine = Engine::new(config.seed, config.sheet, config.frames_per_turn);
    let bots = [
        BotRecord::from(&agents[0].descriptor()),
        BotRecord::from(&agents[1].descriptor()),
    ];
    let per_player = [summarize(&engine, 0), summarize(&engine, 1)];
    let winner: Option<usize> = None;
    let records = vec![
        ReplayRecord::Header(header(config, &bots, &start_garbage)),
        ReplayRecord::Result(ResultRecord {
            winner: winner.map(|slot| slot as u8),
            reason: "Forfeit".to_string(),
            detail: Some(detail.clone()),
            forfeit_cause: Some(ForfeitCause::Unknown),
            forfeit_slot: Some(slot as u8),
            turns: 0,
            final_frame: 0,
            per_player: per_player.iter().map(player_record).collect(),
        }),
    ];
    let outcome = RoundOutcome {
        winner,
        reason: "Forfeit".to_string(),
        detail: Some(detail.clone()),
        forfeit_cause: Some(ForfeitCause::Unknown),
        forfeit_slot: Some(slot as u8),
        turns: 0,
        final_frame: 0,
        per_player: per_player.clone(),
    };
    let mut opener_lines = [0u32; 2];
    let mut opener_placements = [0u32; 2];
    let mut spin_placements = [0u32; 2];
    for record in &records {
        if let ReplayRecord::Turn(turn) = record {
            for placement in &turn.placements {
                let slot = placement.slot as usize;
                if slot < 2 {
                    if placement.spin != "none" {
                        spin_placements[slot] += 1;
                    }
                    if opener_placements[slot] < 14 {
                        opener_lines[slot] += u32::from(placement.lines);
                        opener_placements[slot] += 1;
                    }
                }
            }
        }
    }
    let audit = GameAudit {
        seed: config.seed,
        sheet: config.sheet.as_str().to_string(),
        started_at: config.started_at.clone(),
        profile: config.profile.as_str(),
        winner: winner.map(|slot| slot as u8),
        reason: "Forfeit".to_string(),
        detail: Some(detail),
        forfeit_cause: Some(ForfeitCause::Unknown),
        forfeit_slot: Some(slot as u8),
        turns: 0,
        surge_released: [0, 0],
        max_b2b_streak: [0, 0],
        turns_with_b2b_ge_4: [0, 0],
        totals: [crate::audit::PlayerTotals::default(); 2],
        heights: [crate::audit::HeightStats::default(); 2],
        think: [ThinkStats::default(), ThinkStats::default()],
        first_choice: [FirstChoiceAudit::default(), FirstChoiceAudit::default()],
        scoring: [ScoringAudit::default(), ScoringAudit::default()],
        reachability_rejections: 0,
        start_resends: Vec::new(),
        attax: None,
        bots,
        replay: String::new(),
        verify: crate::replay::VerifyReport::default(),
        queue_invariant_violations: 0,
        bot_fallbacks: 0,
        bot_nodes_total: 0,
        search: [Vec::new(), Vec::new()],
        opener_lines: [0, 0],
        spin_placements: [0, 0],
        repeat: 0,
    };
    for agent in agents.iter_mut() {
        let _ = agent.end_round();
    }
    RoundReport {
        outcome,
        records,
        audit,
        agent_audits: [AgentAudit::default(), AgentAudit::default()],
    }
}

/// Classifies a forfeit (protocol v1.8 §7.1).
///
/// `Vision` when the bot's declared visible rows cannot contain any legal
/// placement: the stack reaches the visible rows, or every legal placement uses
/// the hidden buffer. `Other` when a legal placement exists entirely inside the
/// bot's visible rows (its own movegen is broken). `Unknown` when the bot did
/// not declare `visible_rows`.
pub(crate) fn classify_forfeit(
    engine: &Engine,
    slot: usize,
    visible_rows: u32,
    supports_180: bool,
) -> ForfeitCause {
    if visible_rows == 0 {
        return ForfeitCause::Unknown;
    }
    let player = &engine.players[slot];
    let Some(active) = player.game.active() else {
        return ForfeitCause::Other;
    };
    // The only criterion is whether the referee has a legal placement that fits
    // entirely inside the bot's visible rows. The S2 spawn is in the hidden
    // buffer (row 21), so "the active is above row 20" says nothing about the
    // stack: a low board still has visible placements. (The old check returned
    // Vision for every spawn, which misclassified 194 attax games.)
    let board = player.game.board();
    let moves = crate::movegen::generate_with_180(board, active, false, supports_180);
    if moves.is_empty() {
        return ForfeitCause::Other;
    }
    let any_visible = moves.placements().iter().any(|placement| {
        placement
            .cells
            .iter()
            .all(|&(_, y)| (y as u32) < visible_rows)
    });
    if any_visible {
        ForfeitCause::Other
    } else {
        ForfeitCause::Vision
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use intetrigence_core::{ActivePiece, Board, FULL_ROW, Game, PieceKind, Rotation};

    #[test]
    fn low_board_with_hidden_spawn_is_not_vision() {
        let engine = Engine::new(1, Sheet::A, 12);
        // The active piece spawns at row 21 (hidden), but the board is empty:
        // the referee has visible placements, so the bot's empty suggestion is
        // its own failure, not a vision limitation.
        assert!(engine.players[0].game.active().is_some());
        assert_eq!(classify_forfeit(&engine, 0, 20, true), ForfeitCause::Other);
    }

    #[test]
    fn full_visible_rows_are_vision() {
        // A tall stack with a diagonal hole per row: no piece fits entirely in
        // rows 0..19, and no row is full (so `from_position` accepts it).
        let mut board = Board::new();
        for y in 0..20 {
            board.set_row(y, FULL_ROW & !(1u16 << (y % 10)));
        }
        let active = ActivePiece {
            kind: PieceKind::T,
            rotation: Rotation::Zero,
            x: 3,
            y: 20,
        };
        let game = Game::from_position(1, board, active).expect("legal test position");
        let mut engine = Engine::new(1, Sheet::A, 12);
        engine.players[0].game = game;
        assert_eq!(classify_forfeit(&engine, 0, 20, true), ForfeitCause::Vision);
    }
}
