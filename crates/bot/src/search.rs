//! Anytime beam search over the exact movegen.
//!
//! # Shape
//!
//! - **Beam**: each ply keeps the best `width` nodes. A node is a board plus
//!   the scoring state (B2B streak, combo, pieces placed) and the position in
//!   the piece sequence.
//! - **Hold**: every node expands both "place the current piece" and "use
//!   hold". With an empty hold the swap consumes two sequence entries, which
//!   is exactly the game rule.
//! - **Bag prediction**: the known pieces are the current piece plus the five
//!   preview pieces. Beyond them the 7-bag is inferred (the pieces of the
//!   current bag that have not appeared yet, then whole bags in canonical
//!   order), so the search can reach the configured depth of eight plies.
//!   Predicted plies are discounted, so they only break ties among lines that
//!   are already good on the known pieces.
//! - **Transposition graph**: children are keyed by `(board, sequence
//!   position, hold, b2b, combo, pieces placed)`. A child that is no better
//!   than an already-seen state is dropped, so different piece orders that
//!   reach the same position share one node.
//! - **T-slot cutout**: the leaf value credits T pieces that are still in the
//!   queue with the lines they would clear in the best slot (see
//!   [`crate::eval::t_slot_cutout`]).
//! - **Discounted reward**: `value = Σ discount^i · reward_i + discount^d ·
//!   leaf(board_d)`, the same shape as the public cold-clear-2 idea.
//! - **Anytime**: iterative widening. Each completed iteration replaces the
//!   answer; the search stops at the deadline and keeps the last complete one.
//!   A one-ply greedy result is always available as a floor.
//! - **Threads**: the beam is expanded level-synchronously by a worker pool.
//!   Workers only expand and score; the coordinator merges, applies the
//!   transposition table and truncates, so the answer does not depend on the
//!   thread count.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use intetrigence_core::{ALL_PIECES, Board, PieceKind};

use crate::eval::{EvalContext, Weights, apply_placement, leaf_value, t_slot_lines};
use crate::movegen::{Movegen, Placement};

/// Maximum worker threads (PLAN §7: default 8, the physical core count).
pub const MAX_THREADS: usize = 8;

/// Hard cap for one deterministic search, leaving 5 ms for the referee's own
/// bookkeeping under the 100 ms budget.
pub const DETERMINISTIC_DEADLINE: Duration = Duration::from_millis(95);

/// Depth the first anytime iteration uses, so that a completed result exists
/// almost immediately. The iteration then deepens and widens.
pub const START_DEPTH: usize = 2;

/// Search parameters.
#[derive(Clone, Debug)]
pub struct SearchConfig {
    /// Plies to search. The piece sequence is extended to this length.
    pub depth: usize,
    /// Beam width of the first iteration.
    pub width: usize,
    /// Widest beam the anytime loop will try.
    pub max_width: usize,
    /// Worker threads, clamped to `1..=MAX_THREADS`.
    pub threads: usize,
    /// Wall-clock budget for one move.
    pub budget: Duration,
    pub weights: Weights,
    /// Run one fixed iteration at `(depth, width)` with no deadline instead of
    /// the anytime loop.
    ///
    /// The anytime loop stops on the wall clock, so the number of completed
    /// iterations depends on machine load and thread timing. Measured: the
    /// same seed produced a win in one run and a collapse in another (9004-A),
    /// which makes an A/B on 20-40 games noise. In this mode the search is a
    /// pure function of the position.
    pub deterministic: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            depth: 12,
            width: 96,
            max_width: 768,
            threads: MAX_THREADS,
            budget: Duration::from_millis(100),
            weights: Weights::default(),
            deterministic: false,
        }
    }
}

/// The position the search starts from.
#[derive(Clone, Debug)]
pub struct RootState {
    pub board: Board,
    pub current: PieceKind,
    pub preview: Vec<PieceKind>,
    pub hold: Option<PieceKind>,
    pub context: EvalContext,
}

/// One chosen placement, with the inputs that execute it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    /// Whether the turn starts with a hold.
    pub hold: bool,
    /// The piece that is placed (after the hold swap, if any).
    pub piece: PieceKind,
    pub placement: Placement,
}

/// A root move with the value the search assigned to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub first: Move,
    pub value: f64,
}

/// What the search produced.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best: Move,
    pub value: f64,
    /// Plies actually searched by the winning iteration.
    pub depth: usize,
    /// Beam width of the winning iteration.
    pub width: usize,
    /// Nodes expanded by the winning iteration.
    pub nodes: u64,
    /// Anytime iterations that completed. Zero means the search never got past
    /// the root and the move came from the 1-ply `greedy` path -- the audit
    /// signal for the `91ec267` bug class, where a bot burned its whole budget
    /// and still played a one-ply move.
    pub iterations: usize,
    /// True when `best` came from `greedy` rather than from the beam.
    pub fallback: bool,
    /// Longest single level of the winning iteration.
    ///
    /// The level boundary is the only place a deterministic search can stop,
    /// so the real worst case is `abort_time + max_level_time`, not
    /// `abort_time`.
    pub max_level_time: Duration,
    /// True when a deterministic search hit `DETERMINISTIC_DEADLINE` before
    /// finishing its fixed schedule and returned the last completed level.
    ///
    /// Distinct from `fallback`: an aborted search still completed at least one
    /// level, so it is a normal, time-respecting result.
    pub aborted: bool,
    pub elapsed: Duration,
    /// Every root move that survived, best first.
    pub candidates: Vec<Candidate>,
    /// The full line behind `best`, root move first.
    pub line: Vec<Move>,
}

/// The piece sequence: known pieces plus bag prediction.
#[derive(Clone, Debug)]
pub struct PieceSequence {
    pieces: Vec<PieceKind>,
    known: usize,
}

impl PieceSequence {
    /// `current` followed by `preview`, extended by bag prediction to at least
    /// `min_len` pieces.
    pub fn new(current: PieceKind, preview: &[PieceKind], min_len: usize) -> Self {
        let mut pieces = Vec::with_capacity(min_len.max(preview.len() + 1));
        pieces.push(current);
        pieces.extend_from_slice(preview);
        let known = pieces.len();

        // Infer the rest of the current bag: walk the known pieces and remove
        // them from the bag; a repeat means a new bag started.
        let mut bag: Vec<PieceKind> = ALL_PIECES.to_vec();
        for &piece in &pieces {
            match bag.iter().position(|&candidate| candidate == piece) {
                Some(index) => {
                    bag.remove(index);
                }
                None => {
                    bag = ALL_PIECES.to_vec();
                    bag.retain(|&candidate| candidate != piece);
                }
            }
        }
        while pieces.len() < min_len {
            if bag.is_empty() {
                bag = ALL_PIECES.to_vec();
            }
            pieces.push(bag.remove(0));
        }
        Self { pieces, known }
    }

    pub fn get(&self, index: usize) -> Option<PieceKind> {
        self.pieces.get(index).copied()
    }

    pub fn len(&self) -> usize {
        self.pieces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// How many pieces came from the preview rather than prediction.
    pub fn known(&self) -> usize {
        self.known
    }

    /// T pieces from `next` onwards, plus the hold slot.
    fn t_available(&self, next: usize, hold: Option<PieceKind>) -> usize {
        let held = usize::from(hold == Some(PieceKind::T));
        held + self.pieces[next.min(self.pieces.len())..]
            .iter()
            .filter(|&&piece| piece == PieceKind::T)
            .count()
    }
}

/// One node of the search graph.
#[derive(Clone, Debug)]
struct NodeRecord {
    parent: u32,
    /// The move that produced this node (`None` for the root).
    mv: Option<Move>,
    /// The root move that leads to this node (`None` for the root).
    first: Option<Move>,
    board: Board,
    next: usize,
    hold: Option<PieceKind>,
    context: EvalContext,
    /// Discounted reward accumulated up to this node.
    reward: f64,
    value: f64,
    depth: usize,
}

/// A child produced by a worker, before the coordinator assigns it a node id.
#[derive(Clone, Debug)]
struct Child {
    parent: u32,
    piece: PieceKind,
    hold_used: bool,
    placement: Placement,
    board: Board,
    next: usize,
    hold: Option<PieceKind>,
    context: EvalContext,
    reward: f64,
    value: f64,
    key: u64,
}

/// A chunk of beam nodes handed to one worker.
struct Job {
    chunk: usize,
    level: usize,
    deadline: Instant,
    nodes: Vec<(u32, NodeRecord)>,
}

/// The anytime searcher. Reusable across moves; it owns the sequential
/// movegen scratch.
#[derive(Clone, Debug)]
pub struct Searcher {
    config: SearchConfig,
    movegen: Movegen,
}

impl Searcher {
    pub fn new(config: SearchConfig) -> Self {
        Self {
            config,
            movegen: Movegen::new(),
        }
    }

    pub fn config(&self) -> &SearchConfig {
        &self.config
    }

    /// Runs the anytime loop and returns the best move found.
    ///
    /// Returns `None` when the current piece has no legal placement at all
    /// (the spawn is blocked, i.e. the game is already over). A referee should
    /// check for top-out before asking, but the search never panics on it.
    pub fn search(&mut self, root: &RootState) -> Option<SearchResult> {
        if self.config.deterministic {
            return self.search_fixed(root);
        }
        let start = Instant::now();
        // Leave a small margin for the final merge, which happens after the
        // last deadline check, so the wall-clock time stays inside the budget.
        let deadline = start + self.config.budget.mul_f32(0.95);
        let max_depth = self.config.depth.max(1);
        // Iterative deepening: start shallow enough that the first iteration
        // almost always completes, then grow the depth, then widen the beam,
        // then grow the depth again. The previous version only widened a fixed
        // depth, and if that depth did not finish it fell back to a 1-ply
        // greedy move after burning the whole budget.
        let mut depth = max_depth.clamp(1, START_DEPTH);
        let mut best: Option<SearchResult> = None;
        let mut width = self.config.width.max(1);
        let mut iterations = 0usize;
        loop {
            let sequence = PieceSequence::new(root.current, &root.preview, depth);
            let outcome = if self.config.threads <= 1 {
                self.run_sequential(root, &sequence, depth, width, deadline)
            } else {
                self.run_parallel(root, &sequence, depth, width, deadline)
            };
            let Some((nodes, candidates, line, completed)) = outcome else {
                break;
            };
            if candidates.is_empty() {
                break;
            }
            let Candidate { first, value } = candidates[0];
            best = Some(SearchResult {
                best: first,
                value,
                depth: completed,
                width,
                nodes,
                iterations: iterations + 1,
                fallback: false,
                aborted: false,
                max_level_time: start.elapsed(),
                elapsed: start.elapsed(),
                candidates,
                line,
            });
            iterations += 1;
            if Instant::now() >= deadline {
                break;
            }
            // Deepen before widening. Measured on a real garbage-pressure game
            // (`search_distribution.rs`), widening first spent the whole budget
            // on cheap depth-2 iterations and the completed depth never rose
            // above 2.2, so the bot was effectively playing two plies.
            if depth < max_depth {
                depth = (depth + 2).min(max_depth);
            } else if width < self.config.max_width {
                width = (width * 3 / 2).min(self.config.max_width);
            } else {
                break;
            }
        }

        best.or_else(|| {
            self.greedy(root, start).map(|mut result| {
                result.fallback = true;
                result.iterations = 0;
                result
            })
        })
    }

    /// One iteration at `(depth, width)` with no deadline.
    ///
    /// The beam is deterministic (children are merged in chunk order and
    /// `commit` sorts by value), so the same position always yields the same
    /// move regardless of thread timing.
    fn search_fixed(&mut self, root: &RootState) -> Option<SearchResult> {
        let start = Instant::now();
        let depth = self.config.depth.max(1);
        let width = self.config.width.max(1);
        let sequence = PieceSequence::new(root.current, &root.preview, depth);
        // A fixed schedule has no natural stop, and 400 moves of headroom does
        // not bound 10,000. The cap keeps the worst case under the 100 ms
        // budget; a move that hits it returns the last completed level and is
        // recorded as `aborted`, which is a normal time-respecting result.
        let deadline = start + DETERMINISTIC_DEADLINE;
        // Parallel, but with the workers' clock check disabled: that check is
        // the only time-dependent branch in the beam, and removing it is what
        // makes the merge thread-invariant. The real cap stays at the level
        // boundary, so a level that does not fit the budget is detected there.
        let level_start = Instant::now();
        let outcome = if self.config.threads <= 1 {
            self.run_sequential(root, &sequence, depth, width, deadline)
        } else {
            self.run_parallel_with(
                root,
                &sequence,
                depth,
                width,
                deadline,
                start + Duration::from_secs(3600),
            )
        };
        // One iteration, so its duration divided by the levels it finished is
        // the per-level cost; `run_parallel_with` cannot report per-level
        // timing without threading a clock through the job loop.
        let max_level_time = level_start.elapsed();
        let Some((nodes, candidates, line, completed)) = outcome else {
            return self.greedy(root, start).map(|mut result| {
                result.fallback = true;
                result.iterations = 0;
                result
            });
        };
        if candidates.is_empty() {
            return None;
        }
        let aborted = completed < depth;
        let Candidate { first, value } = candidates[0];
        Some(SearchResult {
            best: first,
            value,
            depth: completed,
            width,
            nodes,
            iterations: 1,
            fallback: false,
            aborted,
            max_level_time,
            elapsed: start.elapsed(),
            candidates,
            line,
        })
    }

    /// One-ply fallback that always returns a move unless the piece cannot be
    /// placed anywhere.
    fn greedy(&mut self, root: &RootState, start: Instant) -> Option<SearchResult> {
        let sequence = PieceSequence::new(root.current, &root.preview, 2);
        let node = root_record(root);
        let mut children = Vec::new();
        let mut movegen = std::mem::take(&mut self.movegen);
        expand_node(
            &mut movegen,
            0,
            &node,
            0,
            &sequence,
            &self.config.weights,
            &mut children,
        );
        self.movegen = movegen;
        children
            .sort_unstable_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.key.cmp(&b.key)));
        let child = children.first()?;
        let first = Move {
            hold: child.hold_used,
            piece: child.piece,
            placement: child.placement,
        };
        Some(SearchResult {
            best: first,
            value: child.value,
            depth: 1,
            width: 1,
            nodes: 1,
            iterations: 0,
            fallback: true,
            aborted: false,
            max_level_time: Duration::ZERO,
            elapsed: start.elapsed(),
            candidates: vec![Candidate {
                first,
                value: child.value,
            }],
            line: vec![first],
        })
    }

    fn run_sequential(
        &mut self,
        root: &RootState,
        sequence: &PieceSequence,
        depth: usize,
        width: usize,
        deadline: Instant,
    ) -> Option<(u64, Vec<Candidate>, Vec<Move>, usize)> {
        let mut arena = vec![root_record(root)];
        let mut beam = vec![0u32];
        let mut transposition = HashMap::new();
        let mut nodes = 0u64;

        // A beam at level k is a complete search to depth k, so a deadline hit
        // keeps the last completed depth instead of throwing the iteration
        // away. `None` is reserved for "not even one level finished".
        let mut completed = 0usize;
        for level in 0..depth {
            if Instant::now() >= deadline {
                break;
            }
            let mut children = Vec::with_capacity(beam.len() * 32);
            for (index, &node_id) in beam.iter().enumerate() {
                // A level can be long, so the deadline is checked inside it as
                // well; the completed levels are kept.
                if index % 4 == 0 && Instant::now() >= deadline {
                    break;
                }
                let node = arena[node_id as usize].clone();
                expand_node(
                    &mut self.movegen,
                    node_id,
                    &node,
                    level,
                    sequence,
                    &self.config.weights,
                    &mut children,
                );
                nodes += 1;
            }
            // Do not start a merge that would run past the deadline: the
            // previous level is complete and only slightly narrower.
            if Instant::now() >= deadline {
                break;
            }
            let next = commit(&mut arena, children, width, &mut transposition);
            if next.is_empty() {
                break;
            }
            beam = next;
            completed = level + 1;
        }
        if completed == 0 {
            return None;
        }
        let (nodes, candidates, line) = finish(&arena, &beam, nodes);
        Some((nodes, candidates, line, completed))
    }

    fn run_parallel(
        &mut self,
        root: &RootState,
        sequence: &PieceSequence,
        depth: usize,
        width: usize,
        deadline: Instant,
    ) -> Option<(u64, Vec<Candidate>, Vec<Move>, usize)> {
        self.run_parallel_with(root, sequence, depth, width, deadline, deadline)
    }

    /// [`Searcher::run_parallel`] with a separate deadline for the workers.
    ///
    /// The workers' `Instant::now()` check is the only time-dependent branch in
    /// the beam, so a deterministic search passes a far-future worker deadline
    /// and keeps the real cap at the level boundary. That costs the ability to
    /// stop mid-level, which is why the per-level time has to fit the budget.
    fn run_parallel_with(
        &mut self,
        root: &RootState,
        sequence: &PieceSequence,
        depth: usize,
        width: usize,
        deadline: Instant,
        worker_deadline: Instant,
    ) -> Option<(u64, Vec<Candidate>, Vec<Move>, usize)> {
        let threads = self.config.threads.clamp(1, MAX_THREADS);
        let weights = &self.config.weights;
        let mut arena = vec![root_record(root)];
        let mut beam = vec![0u32];
        let mut transposition = HashMap::new();
        let mut nodes = 0u64;

        std::thread::scope(|scope| {
            let (result_tx, result_rx) = mpsc::channel::<(usize, Vec<Child>)>();
            let mut job_txs = Vec::with_capacity(threads);
            for _ in 0..threads {
                let (job_tx, job_rx) = mpsc::channel::<Job>();
                job_txs.push(job_tx);
                let result_tx = result_tx.clone();
                scope.spawn(move || {
                    let mut movegen = Movegen::new();
                    while let Ok(job) = job_rx.recv() {
                        let mut children = Vec::with_capacity(job.nodes.len() * 32);
                        for (index, (node_id, node)) in job.nodes.iter().enumerate() {
                            if index % 4 == 0 && Instant::now() >= job.deadline {
                                break;
                            }
                            expand_node(
                                &mut movegen,
                                *node_id,
                                node,
                                job.level,
                                sequence,
                                weights,
                                &mut children,
                            );
                        }
                        if result_tx.send((job.chunk, children)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(result_tx);

            // A beam at level k is a complete search to depth k, so a deadline
            // hit still yields a usable answer: the frontier is the last
            // completed depth. Returning `None` here used to throw the whole
            // iteration away and drop the bot to the 1-ply `greedy` move,
            // which is what every game up to `91ec267` actually played.
            let mut completed = 0usize;
            for level in 0..depth {
                if Instant::now() >= deadline {
                    break;
                }
                let chunk_size = beam.len().div_ceil(threads).max(1);
                let mut jobs = 0usize;
                for (chunk, ids) in beam.chunks(chunk_size).enumerate() {
                    let nodes = ids
                        .iter()
                        .map(|&id| (id, arena[id as usize].clone()))
                        .collect::<Vec<_>>();
                    let job = Job {
                        chunk,
                        level,
                        deadline: worker_deadline,
                        nodes,
                    };
                    if job_txs[chunk].send(job).is_err() {
                        break;
                    }
                    jobs += 1;
                }
                let mut collected: Vec<Option<Vec<Child>>> = vec![None; jobs];
                for _ in 0..jobs {
                    let Some((chunk, children)) = result_rx.recv().ok() else {
                        break;
                    };
                    collected[chunk] = Some(children);
                }
                let mut children = Vec::new();
                for chunk in collected.into_iter().flatten() {
                    children.extend(chunk);
                }
                nodes += beam.len() as u64;
                let next = commit(&mut arena, children, width, &mut transposition);
                if next.is_empty() {
                    break;
                }
                beam = next;
                completed = level + 1;
                if Instant::now() >= deadline {
                    break;
                }
            }
            if completed == 0 {
                return None;
            }
            let (nodes, candidates, line) = finish(&arena, &beam, nodes);
            Some((nodes, candidates, line, completed))
        })
    }
}

fn root_record(root: &RootState) -> NodeRecord {
    NodeRecord {
        parent: u32::MAX,
        mv: None,
        first: None,
        board: root.board.clone(),
        next: 0,
        hold: root.hold,
        context: root.context,
        reward: 0.0,
        value: 0.0,
        depth: 0,
    }
}

/// Expands one node into every legal child (no hold, then hold).
fn expand_node(
    movegen: &mut Movegen,
    node_id: u32,
    node: &NodeRecord,
    level: usize,
    sequence: &PieceSequence,
    weights: &Weights,
    out: &mut Vec<Child>,
) {
    let Some(current) = sequence.get(node.next) else {
        return;
    };
    let mut context = node.context;
    context.t_slot = t_slot_lines(&node.board);
    let scale = weights.discount.powi(level as i32);

    // No hold: place the current piece.
    expand_piece(
        movegen, node_id, node, sequence, weights, scale, context, current, false, out,
    );

    // Hold: place the held piece, or the piece after the current one.
    match node.hold {
        Some(held) => expand_piece(
            movegen, node_id, node, sequence, weights, scale, context, held, true, out,
        ),
        None => {
            if let Some(next) = sequence.get(node.next + 1) {
                expand_piece(
                    movegen, node_id, node, sequence, weights, scale, context, next, true, out,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn expand_piece(
    movegen: &mut Movegen,
    node_id: u32,
    node: &NodeRecord,
    sequence: &PieceSequence,
    weights: &Weights,
    scale: f64,
    context: EvalContext,
    piece: PieceKind,
    hold_used: bool,
    out: &mut Vec<Child>,
) {
    // The hold swap advances the sequence by one, or by two when the slot was
    // empty (the current piece goes into hold and the next one becomes active).
    let next = if hold_used && node.hold.is_none() {
        node.next + 2
    } else {
        node.next + 1
    };
    let hold = if hold_used {
        Some(sequence.get(node.next).unwrap_or(piece))
    } else {
        node.hold
    };
    let t_available = sequence.t_available(next, hold);

    let placements = movegen.generate(&node.board, piece);
    for placement in placements {
        let (board, reward, next_context) =
            apply_placement(&node.board, placement, &context, weights);
        let accumulated = node.reward + scale * reward.value;
        let leaf = leaf_value(&board, &next_context, weights, t_available);
        let value = accumulated + scale * weights.discount * leaf;
        let key = state_key(&board, next, hold, &next_context);
        out.push(Child {
            parent: node_id,
            piece,
            hold_used,
            placement: *placement,
            board,
            next,
            hold,
            context: next_context,
            reward: accumulated,
            value,
            key,
        });
    }
}

/// Sorts children, drops transpositions that are not an improvement, and
/// returns the new beam.
fn commit(
    arena: &mut Vec<NodeRecord>,
    children: Vec<Child>,
    width: usize,
    transposition: &mut HashMap<u64, f64>,
) -> Vec<u32> {
    // Sort small keys rather than the ~300-byte children: the merge is the
    // only serial part of a level, and moving structs dominated it.
    let mut order: Vec<(f64, u64, u32)> = children
        .iter()
        .enumerate()
        .map(|(index, child)| (child.value, child.key, index as u32))
        .collect();
    order.sort_unstable_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let mut beam = Vec::with_capacity(width);
    for (_, _, index) in order {
        let child = &children[index as usize];
        if let Some(&best) = transposition.get(&child.key) {
            if best >= child.value {
                continue;
            }
        }
        transposition.insert(child.key, child.value);
        let id = arena.len() as u32;
        let parent = &arena[child.parent as usize];
        let mv = Move {
            hold: child.hold_used,
            piece: child.piece,
            placement: child.placement,
        };
        let first = parent.first.unwrap_or(mv);
        let depth = parent.depth + 1;
        arena.push(NodeRecord {
            parent: child.parent,
            mv: Some(mv),
            first: Some(first),
            board: child.board.clone(),
            next: child.next,
            hold: child.hold,
            context: child.context,
            reward: child.reward,
            value: child.value,
            depth,
        });
        beam.push(id);
        if beam.len() == width {
            break;
        }
    }
    beam
}

/// Collects the root candidates and the best line.
fn finish(arena: &[NodeRecord], beam: &[u32], nodes: u64) -> (u64, Vec<Candidate>, Vec<Move>) {
    assert!(!beam.is_empty(), "the beam always keeps at least the root");
    let mut best_id = beam[0];
    for &id in beam {
        if arena[id as usize].value > arena[best_id as usize].value {
            best_id = id;
        }
    }

    // One candidate per root move: the best node that starts with it.
    let mut candidates: Vec<Candidate> = Vec::new();
    for &id in beam {
        let node = &arena[id as usize];
        let Some(first) = node.first else {
            continue;
        };
        match candidates
            .iter_mut()
            .find(|candidate| candidate.first == first)
        {
            Some(existing) => {
                if node.value > existing.value {
                    existing.value = node.value;
                }
            }
            None => candidates.push(Candidate {
                first,
                value: node.value,
            }),
        }
    }
    candidates.sort_unstable_by(|a, b| {
        b.value
            .total_cmp(&a.value)
            .then_with(|| move_key(&a.first).cmp(&move_key(&b.first)))
    });

    let mut line = Vec::new();
    let mut cursor = best_id;
    while let Some(mv) = arena[cursor as usize].mv {
        line.push(mv);
        let parent = arena[cursor as usize].parent;
        if parent == u32::MAX {
            break;
        }
        cursor = parent;
    }
    line.reverse();

    (nodes, candidates, line)
}

fn move_key(mv: &Move) -> (u8, u8, u8, i32, i32) {
    (
        u8::from(mv.hold),
        mv.piece as u8,
        mv.placement.piece.rotation as u8,
        mv.placement.piece.x,
        mv.placement.piece.y,
    )
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[inline]
fn fnv(mut hash: u64, value: u64) -> u64 {
    hash ^= value;
    hash.wrapping_mul(FNV_PRIME)
}

/// Transposition key: board plus the search state that affects future value.
fn state_key(board: &Board, next: usize, hold: Option<PieceKind>, context: &EvalContext) -> u64 {
    let mut hash = FNV_OFFSET;
    for y in 0..Board::HEIGHT {
        hash = fnv(hash, u64::from(board.row(y)));
    }
    hash = fnv(hash, next as u64);
    hash = fnv(hash, hold.map_or(7, |piece| piece as u64));
    hash = fnv(hash, u64::from(context.b2b_streak));
    hash = fnv(hash, u64::from(context.combo));
    hash = fnv(hash, u64::from(context.pieces_placed));
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use intetrigence_core::{ActivePiece, Rotation, SpinType};

    fn root(board: Board, current: PieceKind, preview: &[PieceKind]) -> RootState {
        RootState {
            board,
            current,
            preview: preview.to_vec(),
            hold: None,
            context: EvalContext::default(),
        }
    }

    fn fast_config() -> SearchConfig {
        SearchConfig {
            depth: 4,
            width: 16,
            max_width: 16,
            threads: 1,
            budget: Duration::from_secs(5),
            weights: Weights::default(),
            deterministic: false,
        }
    }

    fn tsd_board() -> Board {
        let mut board = Board::new();
        board.set_row(0, 0x1FF);
        board.set_row(1, 0x3EF);
        board.set_row(2, 0x3C7);
        board.set_row(3, 0x1CF);
        board
    }

    #[test]
    fn bag_prediction_completes_the_current_bag() {
        let sequence = PieceSequence::new(
            PieceKind::T,
            &[
                PieceKind::I,
                PieceKind::O,
                PieceKind::S,
                PieceKind::Z,
                PieceKind::J,
            ],
            8,
        );
        assert_eq!(sequence.known(), 6);
        assert_eq!(sequence.len(), 8);
        // The missing piece of the first bag is L, then a fresh bag starts.
        assert_eq!(sequence.get(6), Some(PieceKind::L));
        assert_eq!(sequence.get(7), Some(PieceKind::I));
    }

    #[test]
    fn bag_prediction_handles_a_repeat() {
        // T appears twice, so a new bag must have started.
        let sequence = PieceSequence::new(
            PieceKind::T,
            &[
                PieceKind::I,
                PieceKind::O,
                PieceKind::S,
                PieceKind::Z,
                PieceKind::T,
            ],
            8,
        );
        assert_eq!(sequence.len(), 8);
        assert!(sequence.get(6).is_some());
    }

    #[test]
    fn search_returns_a_legal_move_on_an_empty_board() {
        let mut searcher = Searcher::new(fast_config());
        let state = root(Board::new(), PieceKind::T, &[PieceKind::I, PieceKind::O]);
        let result = searcher.search(&state).expect("a move exists");
        assert!(
            result
                .best
                .placement
                .cells()
                .iter()
                .all(|&(x, y)| (0..10).contains(&x) && (0..40).contains(&y))
        );
        assert!(!result.line.is_empty());
        assert_eq!(result.line[0], result.best);
        assert!(result.candidates.iter().any(|c| c.first == result.best));
    }

    #[test]
    fn search_finds_the_t_spin_when_the_t_is_in_hand() {
        let mut searcher = Searcher::new(SearchConfig {
            depth: 2,
            width: 32,
            max_width: 32,
            threads: 1,
            budget: Duration::from_secs(5),
            weights: Weights::default(),
            deterministic: false,
        });
        let state = root(tsd_board(), PieceKind::T, &[PieceKind::I]);
        let result = searcher.search(&state).expect("a move exists");
        assert_eq!(result.best.piece, PieceKind::T);
        assert_eq!(result.best.placement.spin, SpinType::Full);
    }

    #[test]
    fn hold_is_used_when_it_helps() {
        // Current piece S, hold I, and a well that only the I can fill.
        let mut board = Board::new();
        for y in 0..6 {
            board.set_row(y, Board::FULL_ROW & !(1u16 << 9));
        }
        let mut searcher = Searcher::new(SearchConfig {
            depth: 2,
            width: 48,
            max_width: 48,
            threads: 1,
            budget: Duration::from_secs(5),
            weights: Weights::default(),
            deterministic: false,
        });
        let mut state = root(board, PieceKind::S, &[PieceKind::O]);
        state.hold = Some(PieceKind::I);
        let result = searcher.search(&state).expect("a move exists");
        assert!(result.best.hold, "the I in hold should be used");
        assert_eq!(result.best.piece, PieceKind::I);
    }

    #[test]
    fn thread_count_does_not_change_the_answer() {
        let state = root(
            tsd_board(),
            PieceKind::T,
            &[PieceKind::I, PieceKind::O, PieceKind::S],
        );
        let mut sequential = Searcher::new(SearchConfig {
            threads: 1,
            ..fast_config()
        });
        let a = sequential.search(&state).expect("a move exists");
        let mut parallel = Searcher::new(SearchConfig {
            threads: 4,
            ..fast_config()
        });
        let b = parallel.search(&state).expect("a move exists");
        assert_eq!(a.best, b.best);
        assert_eq!(a.value, b.value);
        assert_eq!(a.line, b.line);
    }

    #[test]
    fn deadline_falls_back_to_a_greedy_move() {
        let mut searcher = Searcher::new(SearchConfig {
            depth: 12,
            width: 4096,
            max_width: 4096,
            threads: 1,
            budget: Duration::from_micros(1),
            weights: Weights::default(),
            deterministic: false,
        });
        let state = root(Board::new(), PieceKind::I, &[PieceKind::O]);
        let result = searcher.search(&state).expect("a move exists");
        assert!(!result.line.is_empty());
        assert!(result.elapsed < Duration::from_secs(1));
    }

    #[test]
    fn a_blocked_spawn_returns_no_move_instead_of_panicking() {
        let mut board = Board::new();
        board.set_row(21, Board::FULL_ROW);
        board.set_row(22, Board::FULL_ROW);
        let state = root(board, PieceKind::T, &[PieceKind::I]);
        let mut searcher = Searcher::new(fast_config());
        assert!(searcher.search(&state).is_none());
    }

    #[test]
    fn transposition_keeps_one_node_per_state() {
        let mut arena = vec![root_record(&root(Board::new(), PieceKind::T, &[]))];
        let board = Board::new();
        let context = EvalContext::default();
        let placement = Placement {
            piece: ActivePiece {
                kind: PieceKind::O,
                rotation: Rotation::Zero,
                x: 4,
                y: 0,
            },
            spin_entry: false,
            fin_or_tst: false,
            immobile: false,
            spin: SpinType::None,
            actions: crate::movegen::ActionSeq::new(),
        };
        let make = |value: f64| Child {
            parent: 0,
            piece: PieceKind::O,
            hold_used: false,
            placement,
            board: board.clone(),
            next: 1,
            hold: None,
            context,
            reward: 0.0,
            value,
            key: state_key(&board, 1, None, &context),
        };
        let mut transposition = HashMap::new();
        let beam = commit(
            &mut arena,
            vec![make(5.0), make(3.0)],
            8,
            &mut transposition,
        );
        assert_eq!(beam.len(), 1);
        assert_eq!(arena[beam[0] as usize].value, 5.0);
    }
}
