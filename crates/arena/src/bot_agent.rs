//! The real intetrigence bot as a referee [`Agent`].
//!
//! Wraps `intetrigence_bot::Searcher` (anytime beam search, `--ms 100`, up to 8
//! threads). The `View` → `RootState` mapping follows `docs/arena.md` §D1.1:
//! `combo` is the value used by this placement, `b2b_streak` is 0-based, and
//! `pending_garbage` includes packets that have not arrived yet.
//!
//! The search assumes the standard spawn. After a garbage insertion the active
//! piece is pushed up, and a blocked standard spawn makes the search return
//! `None`; in either case the agent appends placements generated from the
//! **actual** active piece as a safety net, so the referee always receives a
//! legal candidate.

use std::collections::HashSet;
use std::time::Instant;

use intetrigence_bot::{
    EvalContext, Movegen, Placement as BotPlacement, RootState, SearchConfig, Searcher,
    unique_by_cells,
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
            b2b_streak: view.b2b_streak,
            combo: view.combo,
            pieces_placed: view.pieces_placed,
            incoming: view.pending_garbage,
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
            claimed_spin: Some(placement.spin),
            claimed_attack: Some(Self::declared_attack(view, placement)),
        }
    }

    /// Simple fallback score for the rare non-standard-spawn case.
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
            let candidate = Self::candidate(view, &placement);
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
        match self.searcher.search(&root) {
            Some(result) => {
                self.searches += 1;
                self.nodes_total += result.nodes;
                let elapsed_ms = result.elapsed.as_secs_f64() * 1000.0;
                self.extra_ms_total += elapsed_ms;
                nodes = Some(result.nodes);
                extra_ms = Some(elapsed_ms);
                depth = Some(result.depth as u32);
                width = Some(result.width as u32);
                fallback = result.depth <= 1;
                for candidate in &result.candidates {
                    let converted = Self::candidate(view, &candidate.first.placement);
                    candidates.push(converted);
                }
            }
            None => {
                // The standard spawn is blocked; the actual active piece (pushed
                // up by garbage or Clutch) can still be placed.
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
