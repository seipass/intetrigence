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
use crate::engine::{Engine, Sheet, board_hash};
use crate::replay::{
    BotRecord, GarbageRecord, HashRecord, Header, PlacementRecord, PlayerRecord, ReplayRecord,
    ResultRecord, SCHEMA, TurnRecord, piece_name, spin_name,
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
}

impl RoundConfig {
    pub fn new(seed: u64, sheet: Sheet) -> Self {
        Self {
            seed,
            sheet,
            turn_cap: 1000,
            frames_per_turn: 12,
            rules: "docs/s2-spec.md@v1.3.6".into(),
            protocol: "docs/s2-referee-protocol.md@v1.2".into(),
            started_at: String::new(),
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
    TopOut { loser: usize },
    TurnCap,
    Forfeit { slot: usize, detail: String },
}

pub fn run_round(
    config: &RoundConfig,
    agents: &mut [&mut dyn Agent; 2],
) -> Result<RoundReport, RoundError> {
    let mut engine = Engine::new(config.seed, config.sheet, config.frames_per_turn);
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
    let mut records = vec![ReplayRecord::Header(header(config, &bots))];

    let mut think: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    let mut indices: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    let mut scoring: [ScoringAudit; 2] = [ScoringAudit::default(), ScoringAudit::default()];
    let mut reachability_rejections = 0u32;

    let mut end: Option<End> = None;
    let mut turns_played = 0u32;

    'turns: for t in 0..config.turn_cap {
        engine.turn = t;
        engine.begin_turn();
        turns_played = t + 1;

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
                    });
                    push_turn_end(&mut records, t, turn_record, garbage_records, &engine);
                    break 'turns;
                }
            };
            think[slot].push(suggestion.think_ms);

            let adopted = match engine.adopt(slot, &suggestion) {
                Ok(adopted) => adopted,
                Err(error) => {
                    reachability_rejections += 1;
                    end = Some(End::Forfeit {
                        slot,
                        detail: error.to_string(),
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
    let (winner, reason, detail) = match &end {
        End::TopOut { loser } => (Some(1 - loser), "TopOut".to_string(), None),
        End::TurnCap => {
            let a = &engine.players[0];
            let b = &engine.players[1];
            let winner = if a.received_total() != b.received_total() {
                Some(if a.received_total() < b.received_total() {
                    0
                } else {
                    1
                })
            } else if a.sent_total() != b.sent_total() {
                Some(if a.sent_total() > b.sent_total() {
                    0
                } else {
                    1
                })
            } else {
                None
            };
            (winner, "TurnCap".to_string(), None)
        }
        End::Forfeit { slot, detail } => {
            (Some(1 - slot), "Forfeit".to_string(), Some(detail.clone()))
        }
    };

    let per_player = [summarize(&engine, 0), summarize(&engine, 1)];
    let outcome = RoundOutcome {
        winner,
        reason: reason.clone(),
        detail: detail.clone(),
        turns: turns_played,
        final_frame: engine.frame,
        per_player: per_player.clone(),
    };

    records.push(ReplayRecord::Result(ResultRecord {
        winner: winner.map(|slot| slot as u8),
        reason,
        detail,
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

    let audit = GameAudit {
        seed: config.seed,
        sheet: config.sheet.as_str().to_string(),
        started_at: config.started_at.clone(),
        winner: winner.map(|slot| slot as u8),
        reason: outcome.reason.clone(),
        detail: outcome.detail.clone(),
        turns: turns_played,
        totals: [
            crate::audit::PlayerTotals {
                sent_total: per_player[0].sent_total,
                received_total: per_player[0].received_total,
                attack_total: per_player[0].attack_total,
                pieces_placed: per_player[0].pieces_placed,
            },
            crate::audit::PlayerTotals {
                sent_total: per_player[1].sent_total,
                received_total: per_player[1].received_total,
                attack_total: per_player[1].attack_total,
                pieces_placed: per_player[1].pieces_placed,
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
    };

    Ok(RoundReport {
        outcome,
        records,
        audit,
        agent_audits,
    })
}

fn header(config: &RoundConfig, bots: &[BotRecord; 2]) -> Header {
    let mut slots = BTreeMap::new();
    slots.insert("0".to_string(), "intetrigence".to_string());
    slots.insert("1".to_string(), "attax".to_string());
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
    let engine = Engine::new(config.seed, config.sheet, config.frames_per_turn);
    let bots = [
        BotRecord::from(&agents[0].descriptor()),
        BotRecord::from(&agents[1].descriptor()),
    ];
    let per_player = [summarize(&engine, 0), summarize(&engine, 1)];
    let winner = Some(1 - slot);
    let records = vec![
        ReplayRecord::Header(header(config, &bots)),
        ReplayRecord::Result(ResultRecord {
            winner: winner.map(|slot| slot as u8),
            reason: "Forfeit".to_string(),
            detail: Some(detail.clone()),
            turns: 0,
            final_frame: 0,
            per_player: per_player.iter().map(player_record).collect(),
        }),
    ];
    let outcome = RoundOutcome {
        winner,
        reason: "Forfeit".to_string(),
        detail: Some(detail.clone()),
        turns: 0,
        final_frame: 0,
        per_player: per_player.clone(),
    };
    let audit = GameAudit {
        seed: config.seed,
        sheet: config.sheet.as_str().to_string(),
        started_at: config.started_at.clone(),
        winner: winner.map(|slot| slot as u8),
        reason: "Forfeit".to_string(),
        detail: Some(detail),
        turns: 0,
        totals: [crate::audit::PlayerTotals::default(); 2],
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

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
