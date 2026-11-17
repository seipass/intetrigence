//! The real intetrigence bot as a referee [`Agent`].
//!
//! Wraps `intetrigence_bot::Searcher` (anytime beam search, `--ms 100`, up to 8
//! threads). The `View` → `RootState` mapping follows `docs/arena.md` §D1.1:
//! `combo` is the value used by this placement, `b2b_streak` is 0-based, and
//! `pending_garbage` includes packets that have not arrived yet.
//!
//! Search starts at the active piece's actual position, including after
//! Clutch. Additional legal placements from that position follow the beam's
//! ranked candidates as a fallback.

use std::collections::HashSet;
use std::time::Instant;

use intetrigence_bot::{
    EvalContext, Movegen, Placement as BotPlacement, RootState, SearchConfig, SearchTiming,
    Searcher, unique_by_cells,
};
use intetrigence_core::{Board, ClearInput, PieceKind, compute_attack};

use crate::agent::{
    Agent, AgentAudit, AgentError, BotDescriptor, Candidate, Commit, RoundContext, Suggestion,
};
use crate::engine::View;

#[derive(Clone, Debug)]
pub struct BotAgentConfig {
    pub search: SearchConfig,
    pub name: String,
    pub version: String,
    pub commit: Option<String>,
    /// Human-readable arguments for the replay header.
    pub args: Vec<String>,
}

impl BotAgentConfig {
    pub fn new(search: SearchConfig) -> Self {
        let budget_ms = search.budget.as_millis();
        let threads = search.threads;
        let depth = search.depth;
        let width = search.width;
        let mode = if search.deterministic {
            "fixed"
        } else {
            "anytime"
        };
        Self {
            search,
            name: "intetrigence".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            commit: None,
            args: vec![
                "--ms".into(),
                budget_ms.to_string(),
                "--threads".into(),
                threads.to_string(),
                "--depth".into(),
                depth.to_string(),
                "--width".into(),
                width.to_string(),
                format!("mode={mode}"),
            ],
        }
    }
}

pub struct BotAgent {
    config: BotAgentConfig,
    searcher: Searcher,
    movegen: Movegen,
    think_times: Vec<f64>,
    clutch_fallbacks: u64,
    searches: u64,
    nodes_total: u64,
    extra_ms_total: f64,
}

impl BotAgent {
    pub fn new(config: BotAgentConfig) -> Self {
        Self {
            searcher: Searcher::new(config.search.clone()),
            config,
            movegen: Movegen::new(),
            think_times: Vec::new(),
            clutch_fallbacks: 0,
            searches: 0,
            nodes_total: 0,
            extra_ms_total: 0.0,
        }
    }

    pub fn clutch_fallbacks(&self) -> u64 {
        self.clutch_fallbacks
    }

    pub fn think_times(&self) -> &[f64] {
        &self.think_times
    }

    fn context(view: &View) -> EvalContext {
        EvalContext {
            multiplier: view.multiplier,
            b2b_streak: view.b2b_streak,
            combo: view.combo,
            pieces_placed: view.pieces_placed,
            incoming: view.pending_garbage,
            arrived: view.arrived_garbage,
            sent_total: view.sent_total,
            received_total: view.received_total,
            t_slot: 0,
        }
    }

    /// Attack implied by the placement's spin, using the referee's own scoring
    /// inputs. Recorded as the bot's declared attack for the scoring audit.
    fn declared_attack(view: &View, placement: &BotPlacement) -> u32 {
        let mut board = view.board.clone();
        board
            .place(
                placement.piece.kind,
                placement.piece.rotation,
                placement.piece.x,
                placement.piece.y,
            )
            .expect("a generated placement is legal");
        let clear = board.clear_lines();
        compute_attack(&ClearInput {
            lines: clear.lines,
            spin: placement.spin,
            combo: view.combo,
            b2b_streak: view.b2b_streak,
            perfect_clear: clear.perfect_clear,
            garbage_cleared: u32::from(clear.garbage_lines),
            multiplier: view.multiplier,
        })
        .total
    }

    fn candidate(view: &View, placement: &BotPlacement) -> Candidate {
        let mut cells = placement.cells();
        cells.sort_unstable();
        Candidate {
            piece: placement.piece.kind,
            cells,
            used_hold: None,
            claimed_spin: Some(placement.spin),
            claimed_attack: Some(Self::declared_attack(view, placement)),
        }
    }

    /// Simple fallback score for placements outside the final beam.
    fn fallback_score(board: &Board, placement: &BotPlacement) -> f64 {
        let mut copy = board.clone();
        copy.place(
            placement.piece.kind,
            placement.piece.rotation,
            placement.piece.x,
            placement.piece.y,
        )
        .expect("a generated placement is legal");
        let clear = copy.clear_lines();
        let mut score = 140.0 * f64::from(clear.lines) + 60.0 * f64::from(clear.garbage_lines);
        if clear.perfect_clear {
            score += 800.0;
        }
        if placement.spin != intetrigence_core::SpinType::None {
            score += 30.0;
        }
        let mut holes = 0usize;
        let mut aggregate = 0usize;
        for x in 0..Board::WIDTH {
            let height = copy.column_height(x);
            aggregate += height;
            let mut seen_block = false;
            for y in (0..Board::HEIGHT).rev() {
                if copy.occupied_at(x, y) {
                    seen_block = true;
                } else if seen_block {
                    holes += 1;
                }
            }
        }
        score - 14.0 * holes as f64 - 1.2 * aggregate as f64
    }

    fn append_fallback(&mut self, view: &View, candidates: &mut Vec<Candidate>) {
        let mut seen: HashSet<[(i32, i32); 4]> =
            candidates.iter().map(|candidate| candidate.cells).collect();
        let raw = self.movegen.generate_from(&view.board, view.active);
        let mut placements: Vec<BotPlacement> = unique_by_cells(raw);
        placements.sort_by(|a, b| {
            let a_score = Self::fallback_score(&view.board, a);
            let b_score = Self::fallback_score(&view.board, b);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for placement in placements {
            let mut candidate = Self::candidate(view, &placement);
            candidate.used_hold = Some(false);
            if seen.insert(candidate.cells) {
                candidates.push(candidate);
            }
        }
    }
}

impl Agent for BotAgent {
    fn descriptor(&self) -> BotDescriptor {
        BotDescriptor {
            name: self.config.name.clone(),
            version: Some(self.config.version.clone()),
            commit: self.config.commit.clone(),
            args: self.config.args.clone(),
            visible_rows: 40,
            supports_180: true,
            build_id: Some(crate::BUILD_ID.into()),
            threads: Some(self.config.search.threads as u32),
            suggest_delay_ms: None,
            retry_empty_suggest_ms: None,
        }
    }

    fn begin_round(&mut self, _context: &RoundContext) -> Result<(), AgentError> {
        self.searcher = Searcher::new(self.config.search.clone());
        self.movegen = Movegen::new();
        self.think_times.clear();
        self.clutch_fallbacks = 0;
        self.searches = 0;
        self.nodes_total = 0;
        self.extra_ms_total = 0.0;
        Ok(())
    }

    fn suggest(&mut self, view: &View) -> Result<Suggestion, AgentError> {
        let started = Instant::now();
        let root = RootState {
            board: view.board.clone(),
            current: view.active.kind,
            preview: view.preview.clone(),
            hold: view.hold,
            context: Self::context(view),
        };
        let mut candidates = Vec::new();
        let mut nodes = None;
        let mut extra_ms = None;
        let mut depth = None;
        let mut width = None;
        let mut fallback = true;
        let mut aborted = false;
        let timing =
            SearchTiming::from_queue(view.frame, view.frames_per_turn, &view.garbage_packets);
        match self
            .searcher
            .search_timed_from(&root, view.active, view.can_hold, &timing)
        {
            Some(result) => {
                self.searches += 1;
                self.nodes_total += result.nodes;
                let elapsed_ms = result.elapsed.as_secs_f64() * 1000.0;
                self.extra_ms_total += elapsed_ms;
                nodes = Some(result.nodes);
                extra_ms = Some(elapsed_ms);
                depth = Some(result.depth as u32);
                width = Some(result.width as u32);
                fallback = result.fallback;
                aborted = result.aborted;
                for candidate in &result.candidates {
                    let mut converted = Self::candidate(view, &candidate.first.placement);
                    converted.used_hold = Some(candidate.first.hold);
                    candidates.push(converted);
                }
            }
            None => {
                // The search returned no candidates. Try the unpruned roots.
                self.clutch_fallbacks += 1;
            }
        }
        self.append_fallback(view, &mut candidates);
        if candidates.is_empty() {
            return Err(AgentError::Referee(
                "intetrigence found no legal placement".into(),
            ));
        }
        let think_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.think_times.push(think_ms);
        Ok(Suggestion {
            candidates,
            think_ms,
            nodes,
            extra_ms,
            depth,
            width,
            fallback,
            aborted,
        })
    }

    fn commit(&mut self, _commit: &Commit) -> Result<(), AgentError> {
        // The search is stateless per call; the next `View` carries the
        // authoritative board, queue, hold, combo and B2B.
        Ok(())
    }

    fn end_round(&mut self) -> Result<(), AgentError> {
        Ok(())
    }

    fn audit(&self) -> AgentAudit {
        AgentAudit {
            info_name: self.config.name.clone(),
            info_version: self.config.version.clone(),
            info_author: "intetrigence".into(),
            nodes_total: self.nodes_total,
            extra_ms_total: self.extra_ms_total,
            suggestions_with_stats: self.searches,
            decision_log: None,
            decisions_logged: 0,
            cells_above_row20: false,
            hold_matches: 0,
            hold_mismatches: 0,
            start_resends: Vec::new(),
            clutch_fallbacks: self.clutch_fallbacks,
            queue_invariant_violations: 0,
            command_line: Some(format!("intetrigence {}", self.config.args.join(" "))),
            bun_version: None,
        }
    }
}

/// Pieces a hold would bring in, for tests and diagnostics.
pub fn hold_piece(view: &View) -> Option<PieceKind> {
    view.hold_piece()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Engine, Sheet};
    use intetrigence_bot::spawn_piece;
    use intetrigence_core::Game;

    #[test]
    fn clutch_position_uses_beam_search_from_the_actual_active_piece() {
        let mut board = Board::new();
        let mut first = spawn_piece(PieceKind::I);
        let gap = first
            .cells()
            .into_iter()
            .fold(0, |mask, (x, _)| mask | 1 << x);
        board.set_row(0, Board::FULL_ROW & !gap);
        board.set_row(22, gap);
        first.y -= 2;
        let mut engine = Engine::new(1, Sheet::A, 12);
        let mut game = Game::from_position(1, board, first).unwrap();
        assert_eq!(game.hard_drop().unwrap().lock.clear.lines, 1);
        let active = game.active().unwrap();
        assert_eq!(active.y, spawn_piece(active.kind).y + 1);
        engine.players[0].game = game;
        let view = engine.view(0);
        let mut bot = BotAgent::new(BotAgentConfig::new(SearchConfig {
            depth: 1,
            width: 16,
            threads: 1,
            deterministic: true,
            ..SearchConfig::default()
        }));
        let suggestion = bot.suggest(&view).unwrap();
        assert_eq!(suggestion.depth, Some(1));
        assert!(!suggestion.fallback);
        assert_eq!(bot.clutch_fallbacks(), 0);
        let adopted = engine.adopt(0, &suggestion, true).unwrap();
        assert_eq!(adopted.candidate_index, 0);
        let applied = engine.apply(0, &adopted.target, None).unwrap();
        assert_eq!(applied.piece, active.kind);
        assert!(!applied.hold);
    }

    #[test]
    fn agent_sees_an_inflight_packet_at_its_future_lock() {
        let mut engine = Engine::new(1, Sheet::A, 12);
        engine.turn = 2;
        engine.begin_turn();
        engine.players[0].garbage.receive(3, 12);
        let view = engine.view(0);
        assert_eq!(view.frame, 24);
        assert_eq!(view.arrived_garbage, 0);
        let mut bot = BotAgent::new(BotAgentConfig::new(SearchConfig {
            depth: 4,
            width: 16,
            threads: 2,
            deterministic: true,
            ..SearchConfig::default()
        }));
        let suggestion = bot.suggest(&view).unwrap();
        assert_eq!(suggestion.depth, Some(2));
        assert_eq!(
            engine.adopt(0, &suggestion, true).unwrap().candidate_index,
            0
        );
    }
}
