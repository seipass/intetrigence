//! In-process stand-in bot for M0 plumbing.
//!
//! **This is not intetrigence.** `crates/bot` (search + evaluation) is owned by
//! bot-dev and is still a skeleton; M0's acceptance criterion is the referee,
//! audit and replay path (PLAN.md §5 M0), so this bot only has to produce
//! legal, grounded, reachable placements and exercise hold. It is deterministic
//! and uses the referee's own movegen, which also makes it a useful regression
//! opponent for the harness.
//!
//! The real bot plugs into the same [`Agent`] trait.

use intetrigence_core::{Board, ClearInfo, ClearInput, PieceKind, SpinType, compute_attack};

use crate::agent::{Agent, AgentError, BotDescriptor, Candidate, Commit, RoundContext, Suggestion};
use crate::engine::View;
use crate::movegen::{self, Placement};

#[derive(Clone, Debug)]
pub struct BaselineWeights {
    pub line: f64,
    pub garbage_line: f64,
    pub perfect_clear: f64,
    pub spin: f64,
    pub hole: f64,
    pub aggregate_height: f64,
    pub bumpiness: f64,
    pub max_height: f64,
    pub pending_urgency: f64,
    pub hold_cost: f64,
}

impl Default for BaselineWeights {
    fn default() -> Self {
        Self {
            line: 140.0,
            garbage_line: 60.0,
            perfect_clear: 800.0,
            spin: 30.0,
            hole: 14.0,
            aggregate_height: 1.2,
            bumpiness: 0.9,
            max_height: 5.0,
            pending_urgency: 25.0,
            hold_cost: 2.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BaselineBot {
    weights: BaselineWeights,
    name: String,
}

impl BaselineBot {
    pub fn new() -> Self {
        Self {
            weights: BaselineWeights::default(),
            name: "intetrigence-baseline".into(),
        }
    }

    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            weights: BaselineWeights::default(),
            name: name.into(),
        }
    }

    fn consider(
        &self,
        view: &View,
        hold: bool,
        kind: PieceKind,
        best: &mut Option<(f64, Candidate)>,
    ) {
        let start = if hold {
            movegen::spawn_state(kind)
        } else {
            view.active
        };
        let moves = movegen::generate(&view.board, start, hold);
        for placement in moves.placements() {
            let mut board = view.board.clone();
            board
                .place(placement.kind, placement.rotation, placement.x, placement.y)
                .expect("a reachable placement is legal");
            let clear = board.clear_lines();
            let score = self.evaluate(&board, &clear, placement, view, hold);
            if best.as_ref().is_none_or(|(other, _)| score > *other) {
                let attack = compute_attack(&ClearInput {
                    lines: clear.lines,
                    spin: placement.spin,
                    combo: view.combo,
                    b2b_streak: view.b2b_streak,
                    perfect_clear: clear.perfect_clear,
                    garbage_cleared: clear.garbage_lines as u32,
                    multiplier: view.multiplier,
                });
                *best = Some((
                    score,
                    Candidate {
                        piece: placement.kind,
                        cells: placement.cells,
                        claimed_spin: Some(placement.spin),
                        claimed_attack: Some(attack.total),
                    },
                ));
            }
        }
    }

    fn evaluate(
        &self,
        board: &Board,
        clear: &ClearInfo,
        placement: &Placement,
        view: &View,
        hold: bool,
    ) -> f64 {
        let weights = &self.weights;
        let mut score = 0.0;
        score += weights.line * clear.lines as f64;
        score += weights.garbage_line * clear.garbage_lines as f64;
        if clear.perfect_clear {
            score += weights.perfect_clear;
        }
        if placement.spin != SpinType::None {
            score += weights.spin;
        }
        score -= weights.hole * count_holes(board) as f64;

        let heights: Vec<usize> = (0..Board::WIDTH).map(|x| board.column_height(x)).collect();
        let aggregate: usize = heights.iter().sum();
        score -= weights.aggregate_height * aggregate as f64;
        let bumpiness: usize = heights
            .windows(2)
            .map(|pair| pair[0].abs_diff(pair[1]))
            .sum();
        score -= weights.bumpiness * bumpiness as f64;
        let max_height = heights.iter().copied().max().unwrap_or(0);
        score -= weights.max_height * max_height as f64;

        // Pending garbage makes clears (and downstacking) more urgent.
        score += weights.pending_urgency * view.pending_garbage as f64 * clear.lines as f64;
        if hold {
            score -= weights.hold_cost;
        }
        score
    }
}

impl Default for BaselineBot {
    fn default() -> Self {
        Self::new()
    }
}

fn count_holes(board: &Board) -> usize {
    let mut holes = 0;
    for x in 0..Board::WIDTH {
        let mut seen_block = false;
        for y in (0..Board::HEIGHT).rev() {
            if board.occupied_at(x, y) {
                seen_block = true;
            } else if seen_block {
                holes += 1;
            }
        }
    }
    holes
}

impl Agent for BaselineBot {
    fn descriptor(&self) -> BotDescriptor {
        BotDescriptor {
            name: self.name.clone(),
            version: Some("m0-placeholder".into()),
            commit: None,
            args: vec!["--in-process".into(), "--baseline".into()],
            visible_rows: 40,
            supports_180: true,
            build_id: Some(crate::BUILD_ID.into()),
        }
    }

    fn begin_round(&mut self, _context: &RoundContext) -> Result<(), AgentError> {
        Ok(())
    }

    fn suggest(&mut self, view: &View) -> Result<Suggestion, AgentError> {
        let started = std::time::Instant::now();
        let mut best: Option<(f64, Candidate)> = None;
        self.consider(view, false, view.active.kind, &mut best);
        if view.can_hold {
            if let Some(kind) = view.hold_piece() {
                self.consider(view, true, kind, &mut best);
            }
        }
        let think_ms = started.elapsed().as_secs_f64() * 1000.0;
        let Some((_, candidate)) = best else {
            return Err(AgentError::Referee(
                "baseline found no reachable placement".into(),
            ));
        };
        Ok(Suggestion {
            candidates: vec![candidate],
            think_ms,
            nodes: None,
            extra_ms: None,
            depth: None,
            width: None,
            fallback: false,
            aborted: false,
        })
    }

    fn commit(&mut self, _commit: &Commit) -> Result<(), AgentError> {
        Ok(())
    }

    fn end_round(&mut self) -> Result<(), AgentError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Engine, Sheet};

    #[test]
    fn baseline_returns_a_legal_candidate() {
        let mut engine = Engine::new(3, Sheet::A, 12);
        engine.begin_turn();
        let view = engine.view(0);
        let mut bot = BaselineBot::new();
        let suggestion = bot.suggest(&view).unwrap();
        assert_eq!(suggestion.candidates.len(), 1);
        let target = suggestion.candidates[0].target();
        let moves = movegen::generate(&view.board, view.active, false);
        assert!(moves.get(&target.cells).is_some());
    }

    #[test]
    fn baseline_prefers_a_line_clear() {
        let mut engine = Engine::new(1, Sheet::A, 12);
        // Fill row 0 except column 0 so a vertical I or a horizontal piece can
        // clear it.
        let mut board = Board::new();
        board.set_row(0, Board::FULL_ROW & !0b1);
        engine.players[0].game = intetrigence_core::Game::from_position(
            intetrigence_core::derive_seeds(1).piece_seed,
            board,
            engine.players[0].game.active().unwrap(),
        )
        .unwrap();
        engine.begin_turn();
        let view = engine.view(0);
        let mut bot = BaselineBot::new();
        let suggestion = bot.suggest(&view).unwrap();
        let target = suggestion.candidates[0].target();
        let mut copy = view.board.clone();
        let moves = movegen::generate(&view.board, view.active, false);
        let placement = moves.get(&target.cells).unwrap();
        copy.place(placement.kind, placement.rotation, placement.x, placement.y)
            .unwrap();
        assert!(copy.clear_lines().lines > 0);
    }
}
