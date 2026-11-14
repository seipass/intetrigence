//! TBP agent: hosts an external bot (attax) through [`intetrigence_tbp::Host`].
//!
//! Implements checklist C–G of `docs/attax-integration.md` §8 on the referee
//! side: `start` payloads, `play` for every adopted placement, `new_piece`
//! refills to a six-entry queue, `start` resend after garbage, hold inference,
//! think-time recording and the `ATTAX_TBP_LOG` cross-check.

use std::io::Write as _;
use std::path::PathBuf;

use intetrigence_core::PieceKind;
use intetrigence_tbp::{
    Host, HostConfig, Mirror, Move, Piece, Randomizer, SpinLabel, StartPayload, location_cells,
    location_of_core,
};

use crate::agent::{
    Agent, AgentAudit, AgentError, BotDescriptor, Candidate, Commit, RoundContext, Suggestion,
};
use crate::attax_log::AttaxLog;
use crate::engine::{View, board_hash};

#[derive(Clone, Debug)]
pub struct TbpAgentConfig {
    pub host: HostConfig,
    pub commit: Option<String>,
    pub bun_version: Option<String>,
    /// `ATTAX_TBP_LOG` output path for this game.
    pub decision_log: Option<PathBuf>,
    /// Display name for the bot (default `tbp-bot`).
    pub name: Option<String>,
    /// Rows the bot can see (default 20). Used for forfeit classification.
    pub visible_rows: u32,
    /// Whether the bot's input set includes 180-degree rotations (attax: no).
    pub supports_180: bool,
    /// Delay before each `suggest`, giving a continuously-searching bot a
    /// budget (CC2/clys have no think-time option). Included in `think_ms`.
    pub suggest_delay_ms: u64,
    /// Retry an empty `suggest` for this long (CC2 can answer before its
    /// worker has expanded the root). Zero disables retries, which is required
    /// for bots whose empty suggestion is a real forfeit (attax).
    pub retry_empty_suggest_ms: u64,
    /// JSONL transcript of every host message and suggestion (measurement
    /// completeness, protocol v1.21 §8.3.1).
    pub transcript: Option<PathBuf>,
}

impl TbpAgentConfig {
    pub fn new(host: HostConfig) -> Self {
        Self {
            host,
            commit: None,
            bun_version: None,
            decision_log: None,
            name: None,
            visible_rows: 40,
            supports_180: true,
            suggest_delay_ms: 0,
            retry_empty_suggest_ms: 0,
            transcript: None,
        }
    }
}

pub struct TbpAgent {
    config: TbpAgentConfig,
    host: Option<Host>,
    mirror: Mirror,
    descriptor: BotDescriptor,
    nodes_total: u64,
    extra_ms_total: f64,
    suggestions_with_stats: u64,
    inferred_holds: Vec<bool>,
    hold_matches: u64,
    hold_mismatches: u64,
    start_resends: Vec<(String, u64, u32)>,
    info_author: String,
    decision_log: Option<String>,
    decisions_logged: u64,
    cells_above_row20: bool,
    queue_invariant_violations: u64,
    transcript: Option<PathBuf>,
}

impl TbpAgent {
    pub fn new(config: TbpAgentConfig) -> Self {
        let transcript = config.transcript.clone();
        let descriptor = BotDescriptor {
            name: config.name.clone().unwrap_or_else(|| "tbp-bot".into()),
            version: None,
            commit: config.commit.clone(),
            visible_rows: config.visible_rows,
            supports_180: config.supports_180,
            args: {
                let mut args = vec![config.host.program.clone()];
                args.extend(config.host.args.clone());
                args
            },
        };
        Self {
            config,
            host: None,
            mirror: Mirror::default(),
            descriptor,
            nodes_total: 0,
            extra_ms_total: 0.0,
            suggestions_with_stats: 0,
            inferred_holds: Vec::new(),
            hold_matches: 0,
            hold_mismatches: 0,
            start_resends: Vec::new(),
            info_author: String::new(),
            decision_log: None,
            decisions_logged: 0,
            cells_above_row20: false,
            queue_invariant_violations: 0,
            transcript,
        }
    }

    fn transcript_log(&self, value: serde_json::Value) {
        transcript_log(self.transcript.as_deref(), value);
    }

    pub fn queue_invariant_violations(&self) -> u64 {
        self.queue_invariant_violations
    }

    fn start_payload(
        board: &intetrigence_core::Board,
        queue: &[PieceKind],
        bag_state: &[PieceKind],
        hold: Option<PieceKind>,
        combo: u32,
        b2b_streak: u32,
    ) -> StartPayload {
        StartPayload {
            board: board_to_payload(board),
            queue: queue.iter().copied().map(Piece::from_core).collect(),
            hold: hold.map(Piece::from_core),
            combo,
            back_to_back: b2b_streak >= 1,
            randomizer: if bag_state.is_empty() {
                None
            } else {
                Some(Randomizer::SevenBag {
                    bag_state: bag_state.iter().copied().map(Piece::from_core).collect(),
                })
            },
        }
    }
}

/// `new_piece` messages needed after a `play`: the tail of the authoritative
/// six-entry queue starting at the adapter's current queue length.
///
/// Normal lock: 1 piece. Hold with an empty slot: 2 pieces (the adapter shifted
/// the current piece and the swapped-in piece, `docs/attax-integration.md` §5.4).
pub fn refill_pieces(queue: &[PieceKind], current_len: usize) -> Vec<Piece> {
    queue
        .iter()
        .skip(current_len)
        .copied()
        .map(Piece::from_core)
        .collect()
}

fn transcript_log(path: Option<&std::path::Path>, value: serde_json::Value) {
    let Some(path) = path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{value}");
    }
}

fn board_to_payload(board: &intetrigence_core::Board) -> Vec<Vec<Option<char>>> {
    (0..intetrigence_core::Board::HEIGHT)
        .map(|y| {
            let row = board.row(y);
            (0..intetrigence_core::Board::WIDTH)
                .map(|x| (row & (1u16 << x) != 0).then_some('G'))
                .collect()
        })
        .collect()
}

impl Agent for TbpAgent {
    fn descriptor(&self) -> BotDescriptor {
        self.descriptor.clone()
    }

    fn begin_round(&mut self, context: &RoundContext) -> Result<(), AgentError> {
        let mut host_config = self.config.host.clone();
        if let Some(path) = &self.config.decision_log {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            host_config
                .env
                .push(("ATTAX_TBP_LOG".into(), path.display().to_string()));
            self.decision_log = Some(path.display().to_string());
        }
        let mut host =
            Host::spawn(host_config).map_err(|error| AgentError::Io(error.to_string()))?;
        self.descriptor.name = host.info.name.clone();
        self.descriptor.version = Some(host.info.version.clone());
        self.info_author = host.info.author.clone();

        let mut queue = Vec::with_capacity(6);
        queue.push(context.initial.active.kind);
        queue.extend(context.initial.preview.iter().copied());
        let payload = Self::start_payload(
            &context.initial.board,
            &queue,
            &context.initial.bag_state,
            context.initial.hold,
            context.initial.combo,
            context.initial.b2b_streak,
        );
        self.transcript_log(serde_json::json!({"type": "start", "payload": &payload}));
        host.start(&payload, "initial", board_hash(&context.initial.board), 0);
        self.mirror = Mirror::from_start(&payload);
        self.host = Some(host);
        Ok(())
    }

    fn suggest(&mut self, view: &View) -> Result<Suggestion, AgentError> {
<<<<<<< HEAD
        // Opt-in board dump for desync diagnosis: the referee's board at the
        // moment of `suggest`, in the same 40-row `|`-joined format as the
        // attax adapter's `ATTAX_TBP_BOARD_LOG`. Default off.
        if let Ok(path) = std::env::var("TBP_BOARD_LOG") {
            let mut rows = Vec::with_capacity(intetrigence_core::Board::HEIGHT);
            for y in 0..intetrigence_core::Board::HEIGHT {
                let bits = view.board.row(y);
                rows.push(
                    (0..intetrigence_core::Board::WIDTH)
                        .map(|x| if bits & (1 << x) != 0 { '1' } else { '0' })
                        .collect::<String>(),
                );
            }
            let line = serde_json::json!({
                "turn": view.turn,
                "board": rows.join("|"),
                "hold": format!("{:?}", view.hold),
                "active": format!("{:?}", view.active.kind),
=======
        // Opt-in referee board dump for adapter desync diagnosis. The attax
        // adapter writes its own board with `ATTAX_TBP_BOARD_LOG`; both are
        // one line per `suggest`, row 0 = bottom.
        if let Some(path) = std::env::var_os("TBP_BOARD_LOG") {
            let board = board_to_payload(&view.board)
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell| if cell.is_some() { '1' } else { '0' })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("|");
            let line = serde_json::json!({
                "turn": view.turn,
                "board": board,
                "hold": view.hold.map(|kind| format!("{kind:?}")),
                "active": {
                    "kind": format!("{:?}", view.active.kind),
                    "rotation": format!("{:?}", view.active.rotation),
                    "x": view.active.x,
                    "y": view.active.y,
                },
>>>>>>> 7ac1499
                "combo": view.combo,
                "b2b": view.b2b_streak,
            });
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
<<<<<<< HEAD
=======
                use std::io::Write;
>>>>>>> 7ac1499
                let _ = writeln!(file, "{line}");
            }
        }
        let delay = self.config.suggest_delay_ms;
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        let retry_ms = self.config.retry_empty_suggest_ms;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(retry_ms);
        let mut total_think = 0.0;
        let payload = loop {
            let host = self
                .host
                .as_mut()
                .ok_or_else(|| AgentError::Protocol("suggest before begin_round".into()))?;
            let payload = host
                .suggest()
                .map_err(|error| AgentError::Io(error.to_string()))?;
            total_think += host.think_ms.last().copied().unwrap_or(0.0);
            if !payload.moves.is_empty() || retry_ms == 0 || std::time::Instant::now() >= deadline {
                break payload;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        transcript_log(
            self.transcript.as_deref(),
            serde_json::json!({"type": "suggestion", "payload": &payload}),
        );
        let think_ms = delay as f64 + total_think;
        let (nodes, extra_ms) = match &payload.move_info {
            Some(info) => {
                self.suggestions_with_stats += 1;
                self.nodes_total += info.nodes;
                let extra = info.extra_ms();
                if let Some(ms) = extra {
                    self.extra_ms_total += ms;
                }
                (Some(info.nodes), extra)
            }
            None => (None, None),
        };
        let candidates = payload
            .moves
            .iter()
            .map(|mv| Candidate {
                piece: mv.location.kind.to_core(),
                cells: location_cells(&mv.location),
                claimed_spin: Some(mv.spin.to_core()),
                claimed_attack: None,
            })
            .collect();
        Ok(Suggestion {
            candidates,
            think_ms,
            nodes,
            extra_ms,
        })
    }

    fn commit(&mut self, commit: &Commit) -> Result<(), AgentError> {
        let host = self
            .host
            .as_mut()
            .ok_or_else(|| AgentError::Protocol("commit before begin_round".into()))?;
        let mv = Move {
            location: location_of_core(commit.piece, commit.rotation, commit.x, commit.y),
            spin: SpinLabel::from_core(commit.spin),
        };
        transcript_log(
            self.transcript.as_deref(),
            serde_json::json!({"type": "play", "move": &mv}),
        );
        host.play(mv)
            .map_err(|error| AgentError::Io(error.to_string()))?;
        let inferred_hold = self.mirror.apply_play(&mv);
        self.inferred_holds.push(inferred_hold);
        self.mirror.combo = commit.combo;
        self.mirror.b2b = commit.tbp_b2b();

        if commit.topped_out {
            // The round is over; the adapter's queue state no longer matters.
            return Ok(());
        }
        if commit.garbage_inserted {
            // Checklist E1/E2: the board changed outside `play`, so resend the
            // full state (`start`) with the current queue/hold/combo/b2b.
            let payload = Self::start_payload(
                &commit.board,
                &commit.queue,
                &commit.bag_state,
                commit.hold_piece,
                commit.combo,
                commit.b2b_streak,
            );
            transcript_log(
                self.transcript.as_deref(),
                serde_json::json!({"type": "start", "reason": "garbage", "payload": &payload}),
            );
            host.start(&payload, "garbage", commit.board_hash, commit.turn);
            self.mirror = Mirror::from_start(&payload);
        } else {
            // Checklist D5: refill until the adapter's queue is six entries.
            // A hold with an empty slot makes the adapter consume two entries,
            // so this sends two `new_piece` messages in that case.
            for piece in refill_pieces(&commit.queue, self.mirror.queue.len()) {
                transcript_log(
                    self.transcript.as_deref(),
                    serde_json::json!({"type": "new_piece", "piece": piece}),
                );
                host.new_piece(piece)
                    .map_err(|error| AgentError::Io(error.to_string()))?;
                self.mirror.push(piece);
            }
            if self.mirror.queue.len() < 6 {
                self.queue_invariant_violations += 1;
            }
        }

        let expected: Vec<Piece> = commit.queue.iter().copied().map(Piece::from_core).collect();
        if self.mirror.queue != expected {
            self.queue_invariant_violations += 1;
        }
        Ok(())
    }

    fn end_round(&mut self) -> Result<(), AgentError> {
        self.transcript_log(serde_json::json!({"type": "quit"}));
        if let Some(mut host) = self.host.take() {
            self.start_resends = host
                .start_resends
                .iter()
                .map(|resend| (resend.reason.clone(), resend.board_hash, resend.turn))
                .collect();
            host.quit()
                .map_err(|error| AgentError::Io(error.to_string()))?;
        }
        if let Some(path) = &self.decision_log {
            let path = PathBuf::from(path);
            if path.exists() {
                let log = AttaxLog::read(&path)
                    .map_err(|error| AgentError::Io(format!("read {}: {error}", path.display())))?;
                self.decisions_logged = log.decisions.len() as u64;
                self.cells_above_row20 = log.any_cells_above_row20();
                let holds = log.holds();
                for (index, inferred) in self.inferred_holds.iter().enumerate() {
                    match holds.get(index).copied().flatten() {
                        Some(primary) if primary == *inferred => self.hold_matches += 1,
                        Some(_) => self.hold_mismatches += 1,
                        None => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn audit(&self) -> AgentAudit {
        AgentAudit {
            info_name: self.descriptor.name.clone(),
            info_version: self.descriptor.version.clone().unwrap_or_default(),
            info_author: self.info_author.clone(),
            nodes_total: self.nodes_total,
            extra_ms_total: self.extra_ms_total,
            suggestions_with_stats: self.suggestions_with_stats,
            decision_log: self.decision_log.clone(),
            decisions_logged: self.decisions_logged,
            cells_above_row20: self.cells_above_row20,
            hold_matches: self.hold_matches,
            hold_mismatches: self.hold_mismatches,
            start_resends: self.start_resends.clone(),
            clutch_fallbacks: 0,
            queue_invariant_violations: self.queue_invariant_violations,
            command_line: Some(self.config.host.command_line()),
            bun_version: self.config.bun_version.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intetrigence_tbp::{Location, Orientation};

    #[test]
    fn refill_after_normal_and_hold_locks() {
        let queue = [
            PieceKind::I,
            PieceKind::O,
            PieceKind::T,
            PieceKind::S,
            PieceKind::Z,
            PieceKind::J,
        ];
        assert_eq!(refill_pieces(&queue, 5), vec![Piece::J]);
        assert_eq!(refill_pieces(&queue, 4), vec![Piece::Z, Piece::J]);
    }

    #[test]
    fn hold_turn_refill_restores_the_six_entry_queue() {
        // Adapter state before the turn: current T, preview I O S Z J.
        let mut mirror = Mirror::new(
            vec![Piece::T, Piece::I, Piece::O, Piece::S, Piece::Z, Piece::J],
            None,
            0,
            false,
        );
        // Hold with an empty slot: T goes to hold, I is placed, the adapter
        // consumes two queue entries.
        let mv = Move {
            location: Location {
                kind: Piece::I,
                orientation: Orientation::North,
                x: 4,
                y: 0,
            },
            spin: SpinLabel::None,
        };
        assert!(mirror.apply_play(&mv));
        assert_eq!(mirror.queue, vec![Piece::O, Piece::S, Piece::Z, Piece::J]);

        // The referee's authoritative queue after the lock: current O, preview
        // S Z J L T.
        let authoritative = [
            PieceKind::O,
            PieceKind::S,
            PieceKind::Z,
            PieceKind::J,
            PieceKind::L,
            PieceKind::T,
        ];
        let refill = refill_pieces(&authoritative, mirror.queue.len());
        assert_eq!(refill, vec![Piece::L, Piece::T]);
        for piece in &refill {
            mirror.push(*piece);
        }
        let expected: Vec<Piece> = authoritative
            .iter()
            .copied()
            .map(Piece::from_core)
            .collect();
        assert_eq!(mirror.queue, expected);
    }
}
