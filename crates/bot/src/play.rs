//! Self-contained single-player driver.
//!
//! The driver plays a complete game through [`intetrigence_core::Game`] using
//! only the searcher's chosen [`crate::search::Move`] and its recorded input
//! sequence, so every move is a legal, replayable turn. It records the think
//! time of each move and the S2 attack the engine scored, which is what the
//! M2 acceptance check reports.
//!
//! The referee (`crates/arena`) will drive the same searcher through its own
//! turn loop; this module exists so the bot can be exercised without it.

use std::time::{Duration, Instant};

use intetrigence_core::{Action, ClearInput, Game, GarbageQueue, S2Scoring};

use crate::eval::EvalContext;
use crate::movegen::spawn_piece;
use crate::search::{RootState, SearchConfig, Searcher};

/// Per-game statistics.
#[derive(Clone, Debug)]
pub struct GameReport {
    pub seed: u64,
    /// Placements made.
    pub turns: u32,
    pub lines: u32,
    /// Attack lines the S2 engine credited (before cancelling).
    pub attack: u32,
    /// Attack lines that left the board towards the opponent.
    pub sent_total: u32,
    pub topped_out: bool,
    /// Garbage rows the queue inserted into the board.
    pub garbage_inserted: u32,
    /// Garbage rows cleared by line clears.
    pub garbage_cleared: u32,
    /// Attack rows that cancelled queued garbage.
    pub garbage_cancelled: u32,
    /// Turns where the spawn was not at its standard position (Clutch push-up)
    /// and the search could not be used.
    pub clutch_fallbacks: u32,
    pub think_times: Vec<Duration>,
    /// Every input applied, in order, so the game can be replayed.
    pub actions: Vec<Action>,
}

impl GameReport {
    pub fn max_think(&self) -> Duration {
        self.think_times.iter().copied().max().unwrap_or_default()
    }

    pub fn mean_think(&self) -> Duration {
        if self.think_times.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.think_times.iter().sum();
        total / self.think_times.len() as u32
    }

    pub fn median_think(&self) -> Duration {
        if self.think_times.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.think_times.clone();
        sorted.sort_unstable();
        sorted[sorted.len() / 2]
    }

    /// 95th percentile think time.
    pub fn p95_think(&self) -> Duration {
        if self.think_times.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.think_times.clone();
        sorted.sort_unstable();
        let index = (sorted.len() * 95 / 100).min(sorted.len() - 1);
        sorted[index]
    }
}

/// Plays one game and returns its report.
///
/// Stops on top-out or after `max_turns` placements. Every turn is executed
/// through the engine's input surface, so the resulting game is a legal replay
/// of `(seed, actions)`.
pub fn play_game(seed: u64, config: &SearchConfig, max_turns: u32) -> GameReport {
    play_game_with_garbage(seed, config, max_turns, &[])
}

/// Plays one game with scripted garbage.
///
/// `injections` is `(turn, lines)`: at the start of `turn`, `lines` rows are
/// queued to arrive immediately. Garbage is inserted after a placement that
/// cleared nothing, exactly like the referee (`docs/s2-spec.md` §7.5). This is
/// the cheap proxy for the versus defence: it removes the opponent but keeps
/// the failure mode (a spike the stack cannot absorb).
pub fn play_game_with_garbage(
    seed: u64,
    config: &SearchConfig,
    max_turns: u32,
    injections: &[(u32, u32)],
) -> GameReport {
    let mut game = Game::new(seed);
    let mut garbage = GarbageQueue::new(seed ^ 0x5DEE_CE66_D1CE_B00F);
    let mut next_injection = 0usize;
    let mut scoring = S2Scoring::new();
    let mut searcher = Searcher::new(config.clone());
    let mut report = GameReport {
        seed,
        turns: 0,
        lines: 0,
        attack: 0,
        sent_total: 0,
        topped_out: false,
        garbage_inserted: 0,
        garbage_cleared: 0,
        garbage_cancelled: 0,
        clutch_fallbacks: 0,
        think_times: Vec::new(),
        actions: Vec::new(),
    };

    while report.turns < max_turns && !game.is_over() {
        let Some(active) = game.active() else {
            break;
        };
        let frame = u64::from(report.turns) * 12;
        while next_injection < injections.len() && injections[next_injection].0 <= report.turns {
            garbage.receive(injections[next_injection].1, frame);
            next_injection += 1;
        }
        let board = game.board().clone();
        let preview = game.preview(5);
        let context = EvalContext {
            b2b_streak: scoring.state.b2b_streak,
            combo: scoring.state.combo,
            pieces_placed: game.stats().pieces_placed,
            incoming: garbage.queue_total(),
            t_slot: 0,
        };

        let standard = spawn_piece(active.kind);
        let (hold, placement, spin) = if active == standard {
            let root = RootState {
                board,
                current: active.kind,
                preview,
                hold: game.held_piece(),
                context,
            };
            let start = Instant::now();
            let Some(result) = searcher.search(&root) else {
                // No legal placement: the game is over.
                break;
            };
            report.think_times.push(start.elapsed());
            (
                result.best.hold,
                result.best.placement,
                result.best.placement.spin,
            )
        } else {
            // Clutch pushed the spawn up: the search's spawn assumption does
            // not hold, so drop the piece straight down. Still legal.
            report.clutch_fallbacks += 1;
            report.think_times.push(Duration::ZERO);
            // The standard spawn may be blocked (that is why Clutch pushed the
            // piece up), so explore from the piece's actual position.
            let placement = crate::movegen::generate_from(&board, active)
                .into_iter()
                .find(|placement| {
                    placement.piece.rotation == active.rotation && placement.piece.x == active.x
                })
                .expect("the active piece always has a resting position");
            (false, placement, placement.spin)
        };

        if hold {
            game.apply(Action::Hold);
            report.actions.push(Action::Hold);
        }
        for action in placement.actions.as_slice() {
            game.apply(*action);
            report.actions.push(*action);
        }
        let Some(clear) = game.last_clear() else {
            break;
        };

        let attack = scoring.score(ClearInput {
            lines: clear.lines,
            spin,
            combo: 0,
            b2b_streak: 0,
            perfect_clear: clear.perfect_clear,
            garbage_cleared: u32::from(clear.garbage_lines),
            multiplier: 1.0,
        });
        report.attack += attack.total;
        // Attack cancels queued garbage first, exactly like the referee
        // (`docs/s2-spec.md` §7.3/§7.4). The opener phase doubles the budget
        // for the first 14 placements.
        let opener = game.stats().pieces_placed < 14 && garbage.queue_total() >= report.sent_total;
        let cancel_budget = if opener { attack.total } else { 0 };
        let cancelled = garbage.cancel(attack.total, cancel_budget);
        report.garbage_cancelled += cancelled.cancelled;
        report.sent_total += cancelled.sent;
        report.lines += u32::from(clear.lines);
        report.garbage_cleared += u32::from(clear.garbage_lines);
        report.turns += 1;

        // The referee inserts garbage only when the placement cleared nothing.
        if clear.lines == 0 && !game.is_over() {
            let holes = garbage.take_insertable(0, frame);
            if !holes.is_empty() {
                report.garbage_inserted += holes.len() as u32;
                if game.insert_garbage(&holes).is_err() {
                    // The stack was too tall to take the rows: top-out.
                    report.topped_out = true;
                    break;
                }
            }
        }
    }

    report.topped_out = game.is_over();
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quick_config(budget_ms: u64) -> SearchConfig {
        SearchConfig {
            depth: 4,
            width: 24,
            max_width: 48,
            threads: 2,
            budget: Duration::from_millis(budget_ms),
            ..SearchConfig::default()
        }
    }

    #[test]
    fn plays_a_legal_game() {
        let report = play_game(0x5EED, &quick_config(5), 40);
        assert!(report.turns > 0, "the bot must place at least one piece");
        assert_eq!(report.clutch_fallbacks, 0);
        assert_eq!(report.think_times.len(), report.turns as usize);
        assert!(report.attack >= report.lines / 4);
    }

    #[test]
    fn think_time_stays_within_budget() {
        let budget = Duration::from_millis(10);
        let report = play_game(7, &quick_config(10), 30);
        // The anytime loop checks the deadline between levels, so a level can
        // overshoot by its own duration. Allow a generous factor for CI noise.
        assert!(
            report.max_think() <= budget * 6,
            "max think {:?} exceeds the {budget:?} budget by too much",
            report.max_think()
        );
        assert!(report.median_think() <= budget * 3);
    }

    #[test]
    fn different_seeds_produce_different_games() {
        let a = play_game(1, &quick_config(2), 20);
        let b = play_game(2, &quick_config(2), 20);
        assert!(a.turns > 0 && b.turns > 0);
        assert_ne!((a.lines, a.attack), (b.lines, b.attack));
    }
}
