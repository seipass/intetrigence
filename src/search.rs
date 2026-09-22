use crate::{
    game::{attack, Game, RulesProfile},
    opener::OpenerBook,
};
use intetrigence_engine::{
    bot::{Bot, BotConfig, BotOptions},
    data::{Board, GameState, Placement},
};
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub milliseconds: u64,
    pub iterations: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            milliseconds: 20,
            iterations: 0,
        }
    }
}
#[derive(Default, Serialize)]
pub struct SearchStats {
    pub nodes: u64,
    pub iterations: u64,
    pub elapsed_us: u128,
    pub reused_tree: bool,
}
fn state(game: &Game) -> (GameState, Vec<intetrigence_engine::data::Piece>) {
    let mut queue: Vec<_> = game.queue.iter().copied().collect();
    let reserve = game.hold.unwrap_or_else(|| queue.remove(0));
    let mut bag: enumset::EnumSet<_> = game.bag.iter().copied().collect();
    for &p in queue.iter().rev() {
        if bag == enumset::EnumSet::all() {
            bag.clear();
        }
        bag.insert(p);
    }
    (
        GameState {
            board: game.board,
            bag,
            reserve,
            back_to_back: game.b2b,
            combo: game.combo,
        },
        queue,
    )
}
/// A match-local search tree. Reuse follows the same play/new-piece lifecycle as TBP.
pub struct Searcher {
    improved: bool,
    weights: Option<serde_json::Value>,
    cache: Option<Cached>,
    opener: OpenerBook,
}
struct Cached {
    bot: Backend,
    expected: GameState,
    queue: Vec<intetrigence_engine::data::Piece>,
    pressure: usize,
    stack_danger: u8,
    speculate: bool,
}
enum Backend {
    Improved(Bot),
    Original(cold_clear_2::bot::Bot),
}
fn convert<T: serde::Serialize, U: serde::de::DeserializeOwned>(value: T) -> U {
    serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
}
impl Searcher {
    pub fn new(improved: bool, weights: Option<&serde_json::Value>) -> Self {
        Self {
            improved,
            weights: weights.cloned(),
            cache: None,
            opener: OpenerBook::default(),
        }
    }
    pub fn choose(&mut self, game: &Game, budget: Budget) -> (Option<Placement>, SearchStats) {
        self.choose_inner(game, budget, true)
    }

    /// Choose a legal placement without applying any referee-specific attack
    /// table or scoring profile. This is used by transport adapters whose
    /// server is authoritative and may expose a different ruleset.
    pub fn choose_transport(
        &mut self,
        game: &Game,
        budget: Budget,
    ) -> (Option<Placement>, SearchStats) {
        self.choose_inner(game, budget, false)
    }

    fn choose_inner(
        &mut self,
        game: &Game,
        budget: Budget,
        apply_rules_profile: bool,
    ) -> (Option<Placement>, SearchStats) {
        let start = Instant::now();
        let mut stats = SearchStats::default();
        if game.dead || game.queue.is_empty() {
            return (None, stats);
        }
        if self.improved {
            if let Some(placement) = self.opener.choose(game) {
                self.cache = None;
                stats.elapsed_us = start.elapsed().as_micros();
                return (Some(placement), stats);
            }
        }
        let (root, queue) = state(game);
        // The historical Arena evaluator uses pending garbage as a pressure
        // signal. Guideline's remaining queue is inserted after every lock,
        // so the one-ply policy below evaluates the actual post-lock board
        // instead of applying that older heuristic a second time.
        let pressure =
            if apply_rules_profile && self.improved && matches!(game.rules, RulesProfile::Arena) {
                game.pending.len().min(16)
            } else {
                0
            };
        let stack_danger = if self.improved {
            arena_stack_danger(game)
        } else {
            0
        };
        let reuse = self.cache.as_ref().is_some_and(|c| {
            c.expected == root
                && queue.starts_with(&c.queue)
                && c.pressure == pressure
                && c.stack_danger == stack_danger
                && c.speculate == game.speculate
        });
        stats.reused_tree = reuse;
        if reuse {
            let c = self.cache.as_mut().unwrap();
            for &p in &queue[c.queue.len()..] {
                match &mut c.bot {
                    Backend::Improved(b) => b.new_piece(p),
                    Backend::Original(b) => b.new_piece(convert(p)),
                }
            }
            c.queue = queue.clone();
        } else {
            let bot = if self.improved {
                let mut config: BotConfig = self
                    .weights
                    .as_ref()
                    .map(|w| serde_json::from_value(w.clone()).expect("invalid weights"))
                    .unwrap_or_default();
                if apply_rules_profile
                    && self.weights.is_none()
                    && matches!(game.rules, RulesProfile::Guideline)
                {
                    apply_guideline_weights(&mut config);
                }
                let danger = pressure as f32 / 8.0;
                // Keep additional headroom for an attack that arrives at the
                // next referee boundary.  A fixed pressure multiplier only
                // reacts after garbage is already queued; stack danger is
                // visible one turn earlier and is independent of the seed.
                let stack_danger = stack_danger as f32;
                config.freestyle_weights.height_upper_half *= 1.0 + danger + stack_danger;
                config.freestyle_weights.height_upper_quarter *= 1.0 + danger + stack_danger * 1.5;
                config.freestyle_weights.normal_clears[1] += danger;
                config.freestyle_weights.normal_clears[2] += danger;
                Backend::Improved(Bot::new(
                    BotOptions {
                        speculate: game.speculate,
                        config: Arc::new(config),
                    },
                    root,
                    &queue,
                ))
            } else {
                use cold_clear_2::{
                    bot::{Bot, BotOptions},
                    data::GameState,
                };
                let root = GameState {
                    board: cold_clear_2::data::Board {
                        cols: root.board.cols,
                    },
                    bag: root
                        .bag
                        .iter()
                        .map(convert::<_, cold_clear_2::data::Piece>)
                        .collect(),
                    reserve: convert(root.reserve),
                    back_to_back: root.back_to_back,
                    combo: root.combo,
                };
                Backend::Original(Bot::new(
                    BotOptions {
                        speculate: game.speculate,
                        config: Arc::new(Default::default()),
                    },
                    root,
                    &queue.iter().copied().map(convert).collect::<Vec<_>>(),
                ))
            };
            self.cache = Some(Cached {
                bot,
                expected: root,
                queue: queue.clone(),
                pressure,
                stack_danger,
                speculate: game.speculate,
            });
        }
        let c = self.cache.as_mut().unwrap();
        let workers = if budget.iterations == 0 {
            std::env::var("INTETRIGENCE_SEARCH_WORKERS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|&value| value > 0)
                .unwrap_or_else(|| {
                    std::thread::available_parallelism()
                        .map(|parallelism| parallelism.get())
                        .unwrap_or(1)
                })
                .min(8)
        } else {
            1
        };
        if workers == 1 {
            loop {
                stats.nodes += match &c.bot {
                    Backend::Improved(b) => b.do_work().nodes,
                    Backend::Original(b) => b.do_work().nodes,
                };
                stats.iterations += 1;
                if if budget.iterations > 0 {
                    stats.iterations >= budget.iterations
                } else {
                    start.elapsed() >= Duration::from_millis(budget.milliseconds)
                } {
                    break;
                }
            }
        } else {
            let bot = &c.bot;
            let deadline = Duration::from_millis(budget.milliseconds);
            std::thread::scope(|scope| {
                let handles = (0..workers)
                    .map(|_| {
                        scope.spawn(|| {
                            let mut local = SearchStats::default();
                            while start.elapsed() < deadline {
                                local.nodes += match bot {
                                    Backend::Improved(b) => b.do_work().nodes,
                                    Backend::Original(b) => b.do_work().nodes,
                                };
                                local.iterations += 1;
                            }
                            local
                        })
                    })
                    .collect::<Vec<_>>();
                for handle in handles {
                    let local = handle.join().expect("parallel search worker panicked");
                    stats.nodes += local.nodes;
                    stats.iterations += local.iterations;
                }
            });
        }
        let expand_improved_roots = apply_rules_profile
            && self.improved
            && match game.rules {
                RulesProfile::Guideline => true,
                RulesProfile::Arena => arena_safe_offense_context(game),
            };
        let max_root_candidates = match game.rules {
            RulesProfile::Guideline => 64,
            RulesProfile::Arena => 8,
        };
        let moves: Vec<Placement> = match &c.bot {
            Backend::Improved(b) if expand_improved_roots => b
                .suggest_all()
                .into_iter()
                .take(max_root_candidates)
                .collect(),
            Backend::Improved(b) => b.suggest(),
            Backend::Original(b) => convert(b.suggest()),
        };
        let mv = if apply_rules_profile && self.improved {
            match game.rules {
                RulesProfile::Guideline => choose_guideline_move(game, &moves),
                RulesProfile::Arena => choose_arena_move(game, &moves),
            }
        } else {
            moves
                .iter()
                .copied()
                .find(|m| game.legal(*m) && !locks_out(*m))
        }
        .or_else(|| {
            // TBP permits a queue containing only the active piece. The
            // upstream speculative layer ranks future pieces, so use every
            // legal root, including HOLD, when its retained suggestions do
            // not match the current referee board.
            let mut pieces = vec![game.queue[0]];
            if let Some(piece) = game.hold.or_else(|| game.queue.get(1).copied()) {
                if !pieces.contains(&piece) {
                    pieces.push(piece);
                }
            }
            pieces
                .iter()
                .copied()
                .flat_map(|piece| {
                    intetrigence_engine::movegen::find_moves(&game.board, piece)
                        .into_iter()
                        .map(|(placement, _)| placement)
                })
                .find(|placement| game.legal(*placement) && !locks_out(*placement))
                .or_else(|| {
                    // If every reachable root is a top-out, preserve the
                    // referee's legal-action contract and return one rather
                    // than manufacturing an invalid placement.
                    pieces
                        .iter()
                        .copied()
                        .flat_map(|piece| {
                            intetrigence_engine::movegen::find_moves(&game.board, piece)
                                .into_iter()
                                .map(|(placement, _)| placement)
                        })
                        .find(|placement| game.legal(*placement))
                })
        });
        if let Some(mv) = mv {
            if !c.queue.is_empty() {
                match &mut c.bot {
                    Backend::Improved(b) => b.advance(mv),
                    Backend::Original(b) => b.advance(convert(mv)),
                }
                // Track the real game's state separately; do not repair
                // baseline search bugs.
                c.expected.advance(c.queue.remove(0), mv);
            }
        }
        stats.elapsed_us = start.elapsed().as_micros();
        (mv, stats)
    }
}

/// Quantize the visible stack height into the same bands used by the Arena
/// weight multiplier.  Keeping this in the cache key lets a retained tree
/// adopt the defensive profile as the stack rises instead of freezing the
/// weights from the opening position.
fn arena_stack_danger(game: &Game) -> u8 {
    let stack_height = game
        .board
        .cols
        .iter()
        .map(|&column| 64 - column.leading_zeros())
        .max()
        .unwrap_or(0);
    (stack_height.saturating_sub(12) / 4).min(3) as u8
}

fn apply_guideline_weights(config: &mut BotConfig) {
    // Match the public attack table used by Game::guideline_attack. Explicit
    // --weights files remain untouched so experiments can be reproduced.
    config.freestyle_weights.normal_clears = [0.0, 0.0, 1.0, 2.0, 4.0];
    config.freestyle_weights.mini_spin_clears = [0.0, 0.0, 1.0];
    config.freestyle_weights.spin_clears = [0.0, 2.0, 4.0, 6.0];
    config.freestyle_weights.back_to_back_clear = 1.0;
    config.freestyle_weights.combo_attack = 0.0;
    config.freestyle_weights.perfect_clear = 0.0;
    config.freestyle_weights.perfect_clear_override = false;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TacticalShape {
    max_height: u32,
    aggregate_height: u32,
    holes: u32,
    coveredness: u32,
    bumpiness: u32,
}

fn tactical_shape(board: &Board) -> TacticalShape {
    let mut shape = TacticalShape::default();
    let mut heights = [0; 10];
    for (index, &column) in board.cols.iter().enumerate() {
        let height = 64 - column.leading_zeros();
        heights[index] = height;
        shape.max_height = shape.max_height.max(height);
        shape.aggregate_height += height;
        if height == 0 {
            continue;
        }
        let covered_rows = (1u64 << height) - 1;
        let mut holes = !column & covered_rows;
        shape.holes += holes.count_ones();
        while holes != 0 {
            let row = holes.trailing_zeros();
            shape.coveredness += height - row - 1;
            holes &= holes - 1;
        }
    }
    shape.bumpiness = heights
        .windows(2)
        .map(|pair| pair[0].abs_diff(pair[1]))
        .sum();
    shape
}

/// Re-rank every explored Guideline root with exact referee outcomes. The DAG
/// remains the long-horizon prior; this layer corrects immediate attack,
/// cancellation, garbage rise, and terminal-next-piece blind spots.
fn choose_guideline_move(game: &Game, suggestions: &[Placement]) -> Option<Placement> {
    let candidates = if suggestions.is_empty() {
        all_legal_arena_moves(game)
    } else {
        suggestions.iter().take(64).copied().collect()
    };
    let current = tactical_shape(&game.board);
    let defensive = !game.pending.is_empty() || current.max_height >= 10 || current.holes >= 6;
    let opponent_height = game
        .opponent
        .map(|opponent| tactical_shape(&opponent.board).max_height)
        .unwrap_or(0);
    candidates
        .iter()
        .enumerate()
        .filter(|(_, mv)| game.legal(**mv) && !locks_out(**mv))
        .filter_map(|(rank, mv)| {
            let mut next = game.clone();
            let outcome = next.play(*mv).ok()?;
            if outcome.dead {
                return None;
            }
            let shape = tactical_shape(&next.board);
            let next_unplayable =
                (defensive || shape.max_height >= 14) && all_legal_arena_moves(&next).is_empty();
            let overflow = shape.max_height.saturating_sub(14);
            let attack_weight = 3.0 + opponent_height.saturating_sub(12) as f32 * 0.2;
            let height_weight = if defensive { 0.65 } else { 0.35 };
            let hole_weight = if defensive { 1.6 } else { 1.0 };
            let score = outcome.sent as f32 * attack_weight
                + outcome.cancelled as f32 * 4.0
                + outcome.lines as f32 * 0.35
                - outcome.garbage_applied as f32 * 5.0
                - shape.max_height as f32 * height_weight
                - shape.holes as f32 * hole_weight
                - shape.coveredness as f32 * 0.08
                - shape.bumpiness as f32 * 0.08
                - shape.aggregate_height as f32 * 0.01
                - (overflow * overflow) as f32 * 8.0
                - if next_unplayable { 10_000.0 } else { 0.0 }
                - rank as f32 * 0.08;
            Some((score, *mv))
        })
        .max_by(|(left, _), (right, _)| left.total_cmp(right))
        .map(|(_, mv)| mv)
}

/// Re-rank under Arena pressure or when the stack is already high. The
/// upstream evaluator does not receive the referee queue, so its score does
/// not include cancellation, post-lock garbage, or the resulting playable
/// next-piece state. Simulating the sampled candidates through `Game::play`
/// applies those referee semantics without exposing the opponent's board or
/// randomizer state.
fn choose_arena_move(game: &Game, suggestions: &[Placement]) -> Option<Placement> {
    let pending = game.pending.len();
    // Rebuilding the retained DAG after a garbage rise can consume a short
    // response window before it expands its first node. In that case the
    // engine returns no suggestion; use the complete legal root set as a
    // deterministic tactical fallback instead of move-generator order.
    let candidates = if suggestions.is_empty() {
        all_legal_arena_moves(game)
    } else {
        suggestions.iter().take(64).copied().collect()
    };
    let first_legal = candidates
        .iter()
        .copied()
        .find(|mv| game.legal(*mv) && !locks_out(*mv));
    let current_height = game
        .board
        .cols
        .iter()
        .map(|&column| 64 - column.leading_zeros())
        .max()
        .unwrap_or(0);
    let current_holes = game
        .board
        .cols
        .iter()
        .map(|&column| {
            let height = 64 - column.leading_zeros();
            let filled = if height == 0 { 0 } else { (1u64 << height) - 1 };
            (!column & filled).count_ones()
        })
        .sum::<u32>();
    let has_visible_garbage = game.garbage_board().cols.iter().any(|&column| column != 0);
    let opponent_height = game
        .opponent
        .map(|opponent| {
            opponent
                .board
                .cols
                .iter()
                .map(|&column| 64 - column.leading_zeros())
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let opponent_forecast = game
        .opponent
        .map(|opponent| arena_opponent_attack_forecast(&opponent))
        .unwrap_or_default();
    // Below this threshold the searcher's normal evaluator has enough room
    // to trade attack for shape. Once the stack is high, use the exact
    // one-ply Arena result to account for queued garbage and cancellation.
    if pending == 0 && current_height < 10 && opponent_height < 18 && !suggestions.is_empty() {
        if current_height <= 8 && current_holes <= 2 {
            if let Some(primary) = first_legal {
                let mut primary_state = game.clone();
                if let Ok(primary_outcome) = primary_state.play(primary) {
                    let primary_sent = primary_outcome.sent;
                    if let Some((_, attack)) = candidates
                        .iter()
                        .enumerate()
                        .filter(|(_, mv)| **mv != primary && game.legal(**mv) && !locks_out(**mv))
                        .filter_map(|(rank, &mv)| {
                            let mut next = game.clone();
                            let outcome = next.play(mv).ok()?;
                            if outcome.dead || outcome.sent < primary_sent.saturating_add(2) {
                                return None;
                            }
                            let (height, holes) = arena_shape(&next);
                            if height > current_height.saturating_add(3).max(8)
                                || holes > current_holes.saturating_add(1)
                            {
                                return None;
                            }
                            let score = i64::from(outcome.sent) * 100
                                + i64::from(outcome.lines) * 8
                                - i64::from(height) * 3
                                - i64::from(holes) * 8
                                - rank as i64;
                            Some((score, mv))
                        })
                        .max_by_key(|(score, _)| *score)
                    {
                        return Some(attack);
                    }
                }
            }
        }
        return first_legal;
    }

    let ranked = candidates
        .iter()
        .enumerate()
        .filter(|(_, mv)| game.legal(**mv) && !locks_out(**mv))
        .filter_map(|(rank, mv)| {
            let mut next = game.clone();
            let outcome = next.play(*mv).ok()?;
            if outcome.dead {
                return None;
            }
            let max_height = next
                .board
                .cols
                .iter()
                .map(|&column| 64 - column.leading_zeros())
                .max()
                .unwrap_or(0);
            let holes = next
                .board
                .cols
                .iter()
                .map(|&column| {
                    let height = 64 - column.leading_zeros();
                    let filled = if height == 0 { 0 } else { (1u64 << height) - 1 };
                    (!column & filled).count_ones()
                })
                .sum::<u32>();
            let next_unplayable =
                (pending > 0 || current_height >= 14) && all_legal_arena_moves(&next).is_empty();
            // Any line clear prevents Arena's post-lock garbage insertion.
            // Give that event a dominant, pressure-independent value; among
            // surviving clears, attack/cancellation and the resulting shape
            // break ties. If no clear is available, the height and hole terms
            // choose the least damaging way to absorb the queued rows.
            let pressure = pending as f32;
            let score = outcome.lines as f32 * if pending > 0 { 120.0 } else { 24.0 }
                + outcome.attack as f32 * 2.0
                + outcome.cancelled as f32 * if pending > 0 { 8.0 } else { 0.0 }
                - outcome.garbage_applied as f32 * 120.0
                - max_height as f32 * (1.0 + pressure * 0.08)
                - holes as f32 * (2.0 + pressure * 0.12)
                - if next_unplayable { 10_000.0 } else { 0.0 }
                - rank as f32 * 0.01;
            Some((score, *mv))
        })
        .max_by(|(left, _), (right, _)| left.total_cmp(right))
        .map(|(_, mv)| mv);

    let preferred = ranked
        .or(first_legal)
        .or_else(|| all_legal_arena_moves(game).into_iter().next())?;
    let mut preferred_state = game.clone();
    let preferred_outcome = preferred_state.play(preferred).ok();
    // A root can look attractive to the upstream evaluator while leaving no
    // legal placement for the next piece after the referee applies the
    // queued rows.  This is a terminal state one ply earlier than the actual
    // top-out, so inspect every legal root only in that case.  Ordinary
    // positions keep the engine's search order and are unaffected by this
    // safety net.
    let preferred_unplayable = preferred_outcome
        .as_ref()
        .is_some_and(|_| all_legal_arena_moves(&preferred_state).is_empty());
    if preferred_unplayable {
        if let Some(safe) = all_legal_arena_moves(game)
            .into_iter()
            .filter_map(|mv| {
                let mut next = game.clone();
                let outcome = next.play(mv).ok()?;
                if outcome.dead || all_legal_arena_moves(&next).is_empty() {
                    return None;
                }
                let (height, holes) = arena_shape(&next);
                Some((
                    outcome.lines as f32 * 160.0 + outcome.cancelled as f32 * 12.0
                        - height as f32 * 2.0
                        - holes as f32 * 3.0,
                    mv,
                ))
            })
            .max_by(|(left, _), (right, _)| left.total_cmp(right))
            .map(|(_, mv)| mv)
        {
            return Some(safe);
        }
    }
    // A single zero-attack clear can merely postpone a known garbage queue
    // until the stack is one piece taller.  Compare complete continuations
    // whenever a large queue is visible so cancellation, early absorption,
    // and downstacking are judged by the board they leave several locks
    // later instead of by the current line clear alone.
    let anticipated_rows = if opponent_forecast.immediate >= 4 {
        opponent_forecast.immediate.min(5) as usize
    } else {
        0
    };
    let followup_rows = if anticipated_rows > 0 && opponent_forecast.followup >= 4 {
        opponent_forecast.followup.min(5) as usize
    } else {
        0
    };
    let effective_pending = pending + anticipated_rows + followup_rows;
    if effective_pending >= 4 && current_height + effective_pending.min(8) as u32 >= 13 {
        let (plies, beam_width) = if effective_pending >= 8 {
            (7, 256)
        } else {
            (5, 64)
        };
        let representative_hole = game.pending.back().copied().unwrap_or(5);
        if let Some(planned) = arena_pressure_plan_with_incoming(
            game,
            [anticipated_rows, followup_rows],
            representative_hole,
            plies,
            beam_width,
        ) {
            return Some(planned);
        }
    }
    // Once an attack has already risen, `pending` is empty even though the
    // resulting board can be much more dangerous than it was one lock ago.
    // A tall board with buried garbage and many covered cells needs a
    // multi-lock downstack plan; the normal retained tree was rebuilt at the
    // rise and its first root can otherwise keep stacking above the damage.
    if pending == 0 && has_visible_garbage && current_height >= 14 && current_holes >= 8 {
        if let Some(planned) = arena_recovery_plan(game, 6, 96) {
            return Some(planned);
        }
    }
    // A queued five-row burst can make a locally good line clear a delayed
    // top-out: the clear leaves the queue untouched, and the next two pieces
    // then have to be placed before the rows are absorbed.  When the stack is
    // already high, test a short continuation with a representative burst
    // copied from the visible queue.  This is deliberately a rescue path for
    // emergency positions; normal Arena roots keep the retained search order.
    if pending >= 5 && current_height >= 14 {
        let burst = arena_burst_pattern(game);
        let delay = Some(2);
        let preferred_survives = preferred_outcome.as_ref().is_some_and(|outcome| {
            !outcome.dead && arena_survives_burst(&preferred_state, 3, delay, &burst)
        });
        if !preferred_survives {
            // Several roots may survive the short continuation. Preserve
            // room and cancellation instead of taking move-generator order.
            if let Some(safe) = all_legal_arena_moves(game)
                .into_iter()
                .filter_map(|mv| {
                    if mv == preferred {
                        return None;
                    }
                    let mut next = game.clone();
                    let outcome = next.play(mv).ok()?;
                    if outcome.dead || !arena_survives_burst(&next, 3, delay, &burst) {
                        return None;
                    }
                    let (height, holes) = arena_shape(&next);
                    let score = outcome.lines as f32 * 40.0
                        + outcome.attack as f32 * 10.0
                        + outcome.cancelled as f32 * 12.0
                        - outcome.garbage_applied as f32 * 100.0
                        - height as f32 * 3.0
                        - holes as f32 * 3.0;
                    Some((score, mv))
                })
                .max_by(|(left, _), (right, _)| left.total_cmp(right))
                .map(|(_, mv)| mv)
            {
                return Some(safe);
            }
        }
    }
    // An opponent can send garbage immediately after this lock, before it is
    // visible in `pending`.  When the stack is already high, check the
    // preferred result against every possible sixteen-row hole and switch only
    // if another legal root leaves strictly more continuations alive.
    if pending == 0 && current_height >= 8 {
        if let Some(preferred_state) = preferred_outcome
            .as_ref()
            .filter(|outcome| !outcome.dead)
            .map(|_| preferred_state.clone())
        {
            let preferred_holes = arena_surviving_garbage_holes(&preferred_state);
            if preferred_holes < 10 {
                if let Some((survival, _, safe)) = all_legal_arena_moves(game)
                    .into_iter()
                    .filter_map(|mv| {
                        let mut next = game.clone();
                        let outcome = next.play(mv).ok()?;
                        if outcome.dead {
                            return None;
                        }
                        let survival = arena_surviving_garbage_holes(&next);
                        let (height, holes) = arena_shape(&next);
                        Some((survival, (outcome.lines, height, holes), mv))
                    })
                    .max_by(
                        |(left_survival, left_shape, _), (right_survival, right_shape, _)| {
                            left_survival
                                .cmp(right_survival)
                                .then_with(|| left_shape.0.cmp(&right_shape.0))
                                .then_with(|| right_shape.1.cmp(&left_shape.1))
                                .then_with(|| right_shape.2.cmp(&left_shape.2))
                        },
                    )
                {
                    if survival > preferred_holes {
                        return Some(safe);
                    }
                }
            }
        }
    }
    // When the opponent is already near the top, a quiet placement can let
    // its next attack arrive before our own stack has room to recover.  Use
    // an offensive root only when our preferred move is otherwise safe and
    // the alternative both sends real attack and leaves a continuation.
    if game.opponent.is_some() {
        if opponent_height >= 18
            && current_height <= 14
            && pending == 0
            && preferred_outcome
                .as_ref()
                .is_some_and(|outcome| outcome.attack == 0 && outcome.garbage_applied == 0)
        {
            if let Some(attack) = all_legal_arena_moves(game)
                .into_iter()
                .filter_map(|mv| {
                    let mut next = game.clone();
                    let outcome = next.play(mv).ok()?;
                    if outcome.dead || outcome.attack < 2 {
                        return None;
                    }
                    let (height, holes) = arena_shape(&next);
                    if height > current_height + 4 || all_legal_arena_moves(&next).is_empty() {
                        return None;
                    }
                    Some((
                        outcome.attack as f32 * 100.0 + outcome.lines as f32 * 20.0
                            - height as f32 * 2.0
                            - holes as f32 * 3.0,
                        mv,
                    ))
                })
                .max_by(|(left, _), (right, _)| left.total_cmp(right))
                .map(|(_, mv)| mv)
            {
                return Some(attack);
            }
        }
    }
    // A large queued attack can make a line clear valuable even on a low
    // board.  The retained MCTS roots can omit a low-ranked clear, so scan
    // all legal placements before accepting a move that inserts those rows.
    if pending >= 4
        && preferred_outcome
            .as_ref()
            .is_some_and(|outcome| outcome.lines == 0 && outcome.garbage_applied > 0)
    {
        if let Some(clear) = all_legal_arena_moves(game)
            .into_iter()
            .filter_map(|mv| {
                let mut next = game.clone();
                let outcome = next.play(mv).ok()?;
                if outcome.dead || outcome.lines == 0 {
                    return None;
                }
                let (height, holes) = arena_shape(&next);
                Some((
                    outcome.lines as f32 * 100.0 - height as f32 - holes as f32 * 2.0,
                    mv,
                ))
            })
            .max_by(|(left, _), (right, _)| left.total_cmp(right))
            .map(|(_, mv)| mv)
        {
            return Some(clear);
        }
    }
    Some(preferred)
}

fn arena_shape(game: &Game) -> (u32, u32) {
    let shape = tactical_shape(&game.board);
    (shape.max_height, shape.holes)
}

fn arena_safe_offense_context(game: &Game) -> bool {
    if !game.pending.is_empty() {
        return false;
    }
    let (height, holes) = arena_shape(game);
    height <= 8 && holes <= 2
}

#[derive(Clone)]
struct ArenaPressurePath {
    first: Placement,
    state: Game,
    lines: u32,
    cancelled: u32,
    sent: u32,
}

/// Search a small deterministic beam through the exact visible garbage
/// queue.  The regular engine remains responsible for ordinary stacking;
/// this planner only runs when at least four incoming rows are already known.
#[cfg(test)]
fn arena_pressure_plan(game: &Game, plies: usize, beam_width: usize) -> Option<Placement> {
    arena_plan(
        game,
        plies,
        beam_width,
        ArenaPlanObjective::Pressure,
        [0; 2],
        5,
    )
}

fn arena_pressure_plan_with_incoming(
    game: &Game,
    rows_after_locks: [usize; 2],
    hole: u8,
    plies: usize,
    beam_width: usize,
) -> Option<Placement> {
    arena_plan(
        game,
        plies,
        beam_width,
        ArenaPlanObjective::Pressure,
        rows_after_locks,
        hole,
    )
}

fn arena_recovery_plan(game: &Game, plies: usize, beam_width: usize) -> Option<Placement> {
    arena_plan(
        game,
        plies,
        beam_width,
        ArenaPlanObjective::Recovery,
        [0; 2],
        5,
    )
}

#[derive(Clone, Copy)]
enum ArenaPlanObjective {
    Pressure,
    Recovery,
}

fn arena_plan(
    game: &Game,
    plies: usize,
    beam_width: usize,
    objective: ArenaPlanObjective,
    incoming_after_locks: [usize; 2],
    hole: u8,
) -> Option<Placement> {
    if plies == 0 || beam_width == 0 {
        return None;
    }
    let mut frontier = all_legal_arena_moves(game)
        .into_iter()
        .filter_map(|mv| {
            let mut state = game.clone();
            let outcome = state.play(mv).ok()?;
            if outcome.dead {
                return None;
            }
            if incoming_after_locks[0] > 0 {
                state
                    .pending
                    .extend(std::iter::repeat_n(hole, incoming_after_locks[0]));
            }
            Some(ArenaPressurePath {
                first: mv,
                state,
                lines: outcome.lines,
                cancelled: outcome.cancelled,
                sent: outcome.sent,
            })
        })
        .collect::<Vec<_>>();
    if frontier.is_empty() {
        return None;
    }
    arena_trim_pressure_beam(&mut frontier, beam_width, objective);

    for depth in 1..plies {
        let mut expanded = Vec::new();
        for path in frontier {
            for mv in all_legal_arena_moves(&path.state) {
                let mut state = path.state.clone();
                let Ok(outcome) = state.play(mv) else {
                    continue;
                };
                if outcome.dead {
                    continue;
                }
                if depth < incoming_after_locks.len() && incoming_after_locks[depth] > 0 {
                    state
                        .pending
                        .extend(std::iter::repeat_n(hole, incoming_after_locks[depth]));
                }
                expanded.push(ArenaPressurePath {
                    first: path.first,
                    state,
                    lines: path.lines + outcome.lines,
                    cancelled: path.cancelled + outcome.cancelled,
                    sent: path.sent + outcome.sent,
                });
            }
        }
        if expanded.is_empty() {
            return None;
        }
        arena_trim_pressure_beam(&mut expanded, beam_width, objective);
        frontier = expanded;
    }

    frontier
        .into_iter()
        // Do not select a leaf that is alive only because the next spawn has
        // not been attempted yet.
        .filter(|path| !all_legal_arena_moves(&path.state).is_empty())
        .max_by_key(|path| arena_plan_score(path, objective))
        .map(|path| path.first)
}

fn arena_trim_pressure_beam(
    paths: &mut Vec<ArenaPressurePath>,
    beam_width: usize,
    objective: ArenaPlanObjective,
) {
    paths.sort_unstable_by_key(|path| std::cmp::Reverse(arena_plan_score(path, objective)));
    paths.truncate(beam_width);
}

fn arena_plan_score(path: &ArenaPressurePath, objective: ArenaPlanObjective) -> i64 {
    let shape = tactical_shape(&path.state.board);
    let pending = path.state.pending.len() as u32;
    let garbage_cells = path
        .state
        .garbage_board()
        .cols
        .iter()
        .map(|column| column.count_ones())
        .sum::<u32>();
    // Arena inserts at most eight rows after one quiet lock. Include deferred
    // rows in the danger estimate so a clear is not mistaken for recovery.
    let projected_height = shape.max_height.saturating_add(pending.min(8));
    let danger = projected_height.saturating_sub(14);
    match objective {
        ArenaPlanObjective::Pressure => {
            i64::from(path.cancelled) * 80 + i64::from(path.sent) * 16 + i64::from(path.lines) * 5
                - i64::from(pending) * 30
                - i64::from(shape.max_height) * 4
                - i64::from(shape.holes) * 8
                - i64::from(shape.coveredness)
                - i64::from(shape.bumpiness)
                - i64::from(garbage_cells) * 3
                - i64::from(danger * danger) * 25
        }
        ArenaPlanObjective::Recovery => {
            i64::from(path.lines) * 12 + i64::from(path.sent) * 16
                - i64::from(pending) * 30
                - i64::from(shape.max_height) * 16
                - i64::from(shape.holes) * 24
                - i64::from(shape.coveredness) * 2
                - i64::from(shape.bumpiness)
                - i64::from(garbage_cells) * 6
                - i64::from(danger * danger) * 35
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ArenaOpponentForecast {
    immediate: u32,
    followup: u32,
}

#[derive(Clone)]
struct ArenaOpponentForecastState {
    board: intetrigence_engine::data::Board,
    hold: Option<intetrigence_engine::data::Piece>,
    queue: VecDeque<intetrigence_engine::data::Piece>,
    combo: u8,
    b2b: bool,
}

fn arena_opponent_attack_forecast(opponent: &crate::game::OpponentState) -> ArenaOpponentForecast {
    let mut queue = opponent
        .queue
        .into_iter()
        .flatten()
        .collect::<VecDeque<_>>();
    let mut hold = opponent.hold;
    // Tests and non-arena callers may only populate the compact immediate
    // fields. Preserve that representation while local matches provide the
    // full public ACTIVE/NEXT queue.
    if queue.is_empty() {
        if let Some(active) = opponent.active {
            queue.push_back(active);
        }
        if hold.is_none() {
            if let Some(reserve) = opponent.reserve {
                if queue.front().copied() != Some(reserve) {
                    queue.push_back(reserve);
                }
            }
        } else if hold != opponent.reserve {
            hold = opponent.reserve;
        }
    }
    let initial = ArenaOpponentForecastState {
        board: opponent.board,
        hold,
        queue,
        combo: opponent.combo,
        b2b: opponent.b2b,
    };
    let first = arena_opponent_transitions(&initial);
    let immediate = first.iter().map(|(_, attack)| *attack).max().unwrap_or(0);
    let followup = first
        .iter()
        .filter(|(_, attack)| *attack >= 4)
        .flat_map(|(state, _)| arena_opponent_transitions(state))
        .map(|(_, attack)| attack)
        .filter(|&attack| attack >= 4)
        .max()
        .unwrap_or(0);
    ArenaOpponentForecast {
        immediate,
        followup,
    }
}

fn arena_opponent_transitions(
    source: &ArenaOpponentForecastState,
) -> Vec<(ArenaOpponentForecastState, u32)> {
    let Some(active) = source.queue.front().copied() else {
        return Vec::new();
    };
    let reserve = source.hold.or_else(|| source.queue.get(1).copied());
    [Some(active), reserve]
        .into_iter()
        .flatten()
        .fold(Vec::new(), |mut pieces, piece| {
            if !pieces.contains(&piece) {
                pieces.push(piece);
            }
            pieces
        })
        .into_iter()
        .flat_map(|piece| intetrigence_engine::movegen::find_moves(&source.board, piece))
        .filter(|(placement, _)| !locks_out(*placement))
        .filter_map(|(placement, _)| {
            let mut next = source.clone();
            let current = next.queue.pop_front()?;
            if placement.location.piece != current {
                if next.hold.replace(current).is_none() {
                    next.queue.pop_front()?;
                }
            }
            next.board.place(placement.location);
            let cleared = next.board.line_clears();
            let lines = cleared.count_ones();
            next.board.remove_lines(cleared);
            let perfect_clear = lines > 0 && next.board.cols.iter().all(|&column| column == 0);
            let difficult = lines > 0
                && (lines == 4 || placement.spin != intetrigence_engine::data::Spin::None);
            let prior_b2b = next.b2b;
            next.combo = if lines > 0 {
                next.combo.saturating_add(1)
            } else {
                0
            };
            if lines > 0 {
                next.b2b = difficult;
            }
            let attack = attack(
                lines,
                placement.spin,
                difficult && prior_b2b,
                next.combo,
                perfect_clear,
            );
            Some((next, attack))
        })
        .collect()
}

fn arena_surviving_garbage_holes(game: &Game) -> usize {
    (0..10)
        .filter(|&hole| {
            let mut state = game.clone();
            state.pending.extend(std::iter::repeat_n(hole, 16));
            all_legal_arena_moves(&state).into_iter().any(|mv| {
                let mut next = state.clone();
                next.play(mv).is_ok_and(|outcome| !outcome.dead)
            })
        })
        .count()
}

fn arena_burst_pattern(game: &Game) -> [u8; 5] {
    let mut burst = [0; 5];
    for (slot, &hole) in game.pending.iter().take(5).enumerate() {
        burst[slot] = hole;
    }
    if game.pending.is_empty() {
        // No hole is observable before the opponent's next attack.  A fixed
        // representative keeps the rescue deterministic; the ordinary search
        // still controls which root is used when no emergency is detected.
        burst = [5; 5];
    } else if game.pending.len() < burst.len() {
        let fill = burst[game.pending.len().saturating_sub(1)];
        for hole in burst.iter_mut().skip(game.pending.len()) {
            *hole = fill;
        }
    }
    burst
}

fn arena_survives_burst(game: &Game, remaining: u8, delay: Option<u8>, burst: &[u8; 5]) -> bool {
    if remaining == 0 {
        return true;
    }
    let mut state = game.clone();
    let mut next_delay = delay;
    if matches!(next_delay, Some(0)) && state.pending.is_empty() {
        state.pending.extend(burst.iter().copied());
        next_delay = None;
    }
    all_legal_arena_moves(&state).into_iter().any(|mv| {
        let mut next = state.clone();
        let outcome = next.play(mv).ok();
        let Some(outcome) = outcome else { return false };
        if outcome.dead {
            return false;
        }
        let delay = next_delay.map(|value| value.saturating_sub(1));
        arena_survives_burst(&next, remaining - 1, delay, burst)
    })
}

fn all_legal_arena_moves(game: &Game) -> Vec<Placement> {
    let mut pieces = Vec::with_capacity(2);
    pieces.push(game.queue[0]);
    if let Some(piece) = game.hold.or_else(|| game.queue.get(1).copied()) {
        if !pieces.contains(&piece) {
            pieces.push(piece);
        }
    }
    pieces
        .into_iter()
        .flat_map(|piece| intetrigence_engine::movegen::find_moves(&game.board, piece))
        .map(|(placement, _)| placement)
        .filter(|placement| game.legal(*placement) && !locks_out(*placement))
        .fold(Vec::new(), |mut unique, placement| {
            if !unique.contains(&placement) {
                unique.push(placement);
            }
            unique
        })
}

fn locks_out(mv: Placement) -> bool {
    // A placement can be reachable while every locked cell is above the
    // visible field. The referee treats that as a top-out, so it is not a
    // safe continuation even though movement reachability accepts it.
    mv.location.cells().iter().all(|&(_, y)| y >= 20)
}

pub fn choose(
    game: &Game,
    budget: Budget,
    improved: bool,
    weights: Option<&serde_json::Value>,
) -> (Option<Placement>, SearchStats) {
    Searcher::new(improved, weights).choose(game, budget)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::OpponentState;
    use intetrigence_engine::data::{Board, Piece, PieceLocation, Rotation, Spin};

    #[test]
    fn attacks_high_opponent_even_from_a_low_stack() {
        let mut game = Game::new(1);
        game.board = Board { cols: [0b1111; 10] };
        game.board.cols[4] = 0;
        game.hold = Some(Piece::O);
        game.queue.clear();
        game.queue
            .extend([Piece::I, Piece::O, Piece::T, Piece::S, Piece::Z, Piece::J]);
        game.set_opponent_snapshot(OpponentState {
            board: Board {
                cols: [1 << 18; 10],
            },
            ..OpponentState::default()
        });
        let quiet = all_legal_arena_moves(&game)
            .into_iter()
            .find(|&mv| {
                let mut next = game.clone();
                next.play(mv)
                    .is_ok_and(|outcome| !outcome.dead && outcome.lines == 0)
            })
            .expect("a quiet placement must exist");

        let chosen = choose_arena_move(&game, &[quiet]).expect("a move must be chosen");
        let outcome = game.play(chosen).expect("the chosen placement is legal");
        assert!(
            outcome.attack >= 2,
            "should attack before the opponent recovers"
        );
    }

    #[test]
    fn converts_a_safe_explored_root_into_a_large_attack() {
        let mut game = Game::new(5);
        game.board = Board { cols: [0b1111; 10] };
        game.board.cols[4] = 0;
        game.hold = Some(Piece::O);
        game.queue.clear();
        game.queue
            .extend([Piece::I, Piece::O, Piece::T, Piece::S, Piece::Z, Piece::J]);
        assert!(arena_safe_offense_context(&game));

        let moves = all_legal_arena_moves(&game);
        let quiet = moves
            .iter()
            .copied()
            .find(|&mv| {
                let mut next = game.clone();
                next.play(mv)
                    .is_ok_and(|outcome| !outcome.dead && outcome.sent == 0)
            })
            .expect("a quiet placement must exist");
        let attack = moves
            .iter()
            .copied()
            .find(|&mv| {
                let mut next = game.clone();
                next.play(mv)
                    .is_ok_and(|outcome| !outcome.dead && outcome.sent >= 2)
            })
            .expect("an attacking placement must exist");

        let chosen = choose_arena_move(&game, &[quiet, attack]).expect("a move must be chosen");
        let outcome = game.play(chosen).expect("the chosen placement is legal");
        assert!(outcome.sent >= 2);
    }

    #[test]
    fn guideline_reranking_uses_all_explored_roots_for_immediate_attack() {
        let mut game = Game::new_with_rules(6, crate::game::RulesProfile::Guideline);
        game.board = Board { cols: [0b1111; 10] };
        game.board.cols[4] = 0;
        game.hold = Some(Piece::O);
        game.queue.clear();
        game.queue
            .extend([Piece::I, Piece::O, Piece::T, Piece::S, Piece::Z, Piece::J]);

        let moves = all_legal_arena_moves(&game);
        let quiet = moves
            .iter()
            .copied()
            .find(|&mv| {
                let mut next = game.clone();
                next.play(mv)
                    .is_ok_and(|outcome| !outcome.dead && outcome.sent == 0)
            })
            .expect("a quiet placement must exist");
        let attack = moves
            .iter()
            .copied()
            .find(|&mv| {
                let mut next = game.clone();
                next.play(mv)
                    .is_ok_and(|outcome| !outcome.dead && outcome.sent >= 4)
            })
            .expect("a Tetris placement must exist");

        let chosen = choose_guideline_move(&game, &[quiet, attack]).expect("a move must be chosen");
        let outcome = game.play(chosen).expect("the chosen placement is legal");
        assert!(outcome.sent >= 4);
    }

    #[test]
    fn finds_a_clear_before_inserting_a_large_queue_on_a_low_board() {
        let mut game = Game::new(2);
        game.board = Board {
            cols: [1, 0, 1, 7, 3, 7, 15, 31, 7, 1],
        };
        game.hold = Some(Piece::I);
        game.queue.clear();
        game.queue
            .extend([Piece::J, Piece::S, Piece::Z, Piece::L, Piece::O, Piece::I]);
        game.pending.extend([0, 0, 0, 0]);
        let quiet = all_legal_arena_moves(&game)
            .into_iter()
            .find(|&mv| {
                let mut next = game.clone();
                next.play(mv).is_ok_and(|outcome| {
                    !outcome.dead && outcome.lines == 0 && outcome.garbage_applied == 4
                })
            })
            .expect("a quiet placement must exist");

        let chosen = choose_arena_move(&game, &[quiet]).expect("a move must be chosen");
        let outcome = game.play(chosen).expect("the chosen placement is legal");
        assert!(outcome.lines > 0);
        assert_eq!(outcome.garbage_applied, 0);
    }

    #[test]
    fn pressure_plan_looks_past_a_zero_attack_clear() {
        let mut game = Game::new(3);
        game.board = Board {
            cols: [4095, 8191, 16383, 16382, 16383, 16383, 16383, 2047, 1, 4095],
        };
        game.hold = Some(Piece::Z);
        game.queue.clear();
        game.queue
            .extend([Piece::O, Piece::S, Piece::I, Piece::T, Piece::J, Piece::T]);
        game.pending.extend([7, 7, 7, 7, 7]);
        let delayed = Placement {
            location: PieceLocation {
                piece: Piece::O,
                rotation: Rotation::North,
                x: 7,
                y: 11,
            },
            spin: Spin::None,
        };
        let mut delayed_state = game.clone();
        let delayed_outcome = delayed_state.play(delayed).expect("recorded move is legal");
        assert_eq!(delayed_outcome.lines, 1);
        assert_eq!(delayed_outcome.attack, 0);
        assert_eq!(delayed_state.pending.len(), 5);

        let planned = arena_pressure_plan(&game, 5, 64).expect("pressure path must exist");
        let mut planned_state = game.clone();
        let planned_outcome = planned_state.play(planned).expect("planned move is legal");
        assert_ne!(planned, delayed);
        assert_eq!(planned_outcome.lines, 0);
        assert_eq!(planned_outcome.garbage_applied, 5);
        assert!(planned_state.pending.is_empty());
    }

    #[test]
    fn shields_only_when_the_visible_opponent_has_a_difficult_clear() {
        let mut game = Game::new(4);
        game.board = Board {
            cols: [415, 892, 2047, 487, 255, 255, 511, 1019, 767, 1023],
        };
        game.hold = Some(Piece::S);
        game.queue.clear();
        game.queue
            .extend([Piece::I, Piece::Z, Piece::T, Piece::J, Piece::O, Piece::Z]);
        game.set_opponent_snapshot(OpponentState {
            board: Board {
                cols: [
                    16383, 16383, 8191, 4095, 8191, 127, 4064, 16287, 16383, 32767,
                ],
            },
            active: Some(Piece::T),
            reserve: Some(Piece::I),
            b2b: true,
            ..OpponentState::default()
        });
        let exposed = Placement {
            location: PieceLocation {
                piece: Piece::S,
                rotation: Rotation::East,
                x: 3,
                y: 9,
            },
            spin: Spin::None,
        };
        assert!(arena_opponent_attack_forecast(&game.opponent.unwrap()).immediate >= 4);

        let shielded = choose_arena_move(&game, &[exposed]).expect("a shielded move must exist");
        assert_ne!(shielded, exposed);
    }

    #[test]
    fn forecasts_two_public_back_to_back_attacks_at_their_turns() {
        let mut board = Board {
            cols: [0b1111_1111; 10],
        };
        board.cols[9] = 0;
        let mut queue = [None; 6];
        queue[0] = Some(Piece::I);
        queue[1] = Some(Piece::I);
        let opponent = OpponentState {
            board,
            active: Some(Piece::I),
            reserve: Some(Piece::I),
            hold: None,
            queue,
            ..OpponentState::default()
        };

        let forecast = arena_opponent_attack_forecast(&opponent);
        assert!(forecast.immediate >= 4);
        assert!(forecast.followup >= 4);
    }
}
