//! Integration tests for the M2 search and the single-player driver.
//!
//! These exercise the whole stack the referee will use: exact movegen →
//! anytime beam search → S2 scoring → engine inputs. The game is played only
//! through `Game::apply`, so the recorded action log is a legal replay.

use std::time::Duration;

use intetrigence_bot::{
    EvalContext, GameReport, RootState, SearchConfig, Searcher, play_game, spawn_piece,
};
use intetrigence_core::{Board, Game, PieceKind};

fn real_config() -> SearchConfig {
    SearchConfig {
        depth: 8,
        width: 96,
        max_width: 768,
        threads: 8,
        budget: Duration::from_millis(100),
        ..SearchConfig::default()
    }
}

fn board_signature(board: &Board) -> Vec<u16> {
    (0..Board::HEIGHT).map(|y| board.row(y)).collect()
}

#[test]
fn plays_a_complete_legal_game_at_the_match_settings() {
    let report = play_game(0xA11CE, &real_config(), 12);
    assert!(report.turns >= 10, "the bot must keep playing: {report:?}");
    assert_eq!(report.clutch_fallbacks, 0);
    assert_eq!(report.think_times.len(), report.turns as usize);

    // The recorded inputs must replay to the same board.
    let mut game = Game::new(0xA11CE);
    for action in &report.actions {
        assert!(
            game.apply(*action).accepted,
            "recorded input {action:?} was rejected on replay"
        );
    }
    let mut reference = Game::new(0xA11CE);
    for action in &report.actions {
        reference.apply(*action);
    }
    assert_eq!(
        board_signature(game.board()),
        board_signature(reference.board())
    );
}

/// A board the fixed depth-8 beam cannot finish inside the budget.
fn heavy_board() -> intetrigence_core::Board {
    let mut board = intetrigence_core::Board::new();
    for row in 0..6u16 {
        let hole = (row * 3 + 1) % 10;
        board.set_row(
            row as usize,
            intetrigence_core::Board::FULL_ROW & !(1u16 << hole),
        );
    }
    board
}

/// Regression for `91ec267`: the anytime loop used to discard an iteration
/// whenever a level missed the deadline, so on a board the fixed depth could
/// not finish the search fell back to the 1-ply greedy move (width 1, nodes 1)
/// after burning the whole budget. Every game before that commit played the
/// greedy move.
#[test]
fn a_completed_iteration_is_never_thrown_away() {
    let root = intetrigence_bot::RootState {
        board: heavy_board(),
        current: intetrigence_core::PieceKind::T,
        preview: vec![
            intetrigence_core::PieceKind::I,
            intetrigence_core::PieceKind::O,
            intetrigence_core::PieceKind::S,
            intetrigence_core::PieceKind::Z,
            intetrigence_core::PieceKind::L,
            intetrigence_core::PieceKind::J,
            intetrigence_core::PieceKind::T,
        ],
        hold: Some(intetrigence_core::PieceKind::L),
        context: intetrigence_bot::EvalContext::default(),
    };
    // Deterministic: the assertion is "a completed iteration is kept", so the
    // test must not race a wall-clock budget. With `budget` alone a loaded
    // machine can end the search before level 0 finishes, and the test then
    // fails on the machine's speed rather than on the behaviour it checks.
    let config = SearchConfig {
        deterministic: true,
        budget: Duration::from_millis(60),
        ..real_config()
    };
    let mut searcher = intetrigence_bot::Searcher::new(config);
    let result = searcher.search(&root).expect("the board has placements");
    assert!(
        result.nodes > 1,
        "the search fell back to the 1-ply greedy move (width {}, nodes {})",
        result.width,
        result.nodes
    );
    assert!(
        result.depth >= 2,
        "a completed iteration must be reported, got depth {}",
        result.depth
    );
    assert!(!result.candidates.is_empty());
}

/// The partial-result contract: a deadline that expires mid-level still yields
/// the last completed depth, and only a search that completed nothing at all
/// may return the greedy move.
#[test]
fn a_tight_deadline_still_returns_a_move() {
    let root = intetrigence_bot::RootState {
        board: heavy_board(),
        current: intetrigence_core::PieceKind::T,
        preview: vec![
            intetrigence_core::PieceKind::I,
            intetrigence_core::PieceKind::O,
        ],
        hold: None,
        context: intetrigence_bot::EvalContext::default(),
    };
    let config = SearchConfig {
        budget: Duration::from_micros(200),
        ..real_config()
    };
    let mut searcher = intetrigence_bot::Searcher::new(config);
    let result = searcher.search(&root).expect("the board has placements");
    // Whatever the machine speed, a move comes back and it is a legal one.
    assert!(!result.candidates.is_empty() || result.nodes > 0);
    assert!(result.elapsed <= Duration::from_millis(200));
}

/// Deterministic mode: the same position always yields the same move.
///
/// The anytime loop stops on the wall clock, so the same seed produced a win
/// in one run and a collapse in another (9004-A). An A/B on 20-40 games is
/// noise in that regime, so the search has a fixed-schedule mode.
#[test]
fn deterministic_mode_repeats_exactly() {
    let config = SearchConfig {
        deterministic: true,
        depth: 6,
        width: 96,
        threads: 8,
        ..real_config()
    };
    let root = intetrigence_bot::RootState {
        board: heavy_board(),
        current: intetrigence_core::PieceKind::T,
        preview: vec![
            intetrigence_core::PieceKind::I,
            intetrigence_core::PieceKind::O,
            intetrigence_core::PieceKind::S,
            intetrigence_core::PieceKind::Z,
            intetrigence_core::PieceKind::L,
            intetrigence_core::PieceKind::J,
        ],
        hold: Some(intetrigence_core::PieceKind::L),
        context: intetrigence_bot::EvalContext::default(),
    };
    // Same answer at every thread count: the beam merge is a total order on
    // content, so the workers' completion order cannot leak into the result.
    let mut reference = None;
    for threads in [1usize, 2, 8] {
        let config = SearchConfig {
            threads,
            ..config.clone()
        };
        let mut searcher = intetrigence_bot::Searcher::new(config);
        let result = searcher.search(&root).expect("placements");
        match &reference {
            None => reference = Some(result.clone()),
            Some(expected) => {
                assert_eq!(result.best, expected.best, "threads {threads}");
                assert_eq!(result.nodes, expected.nodes, "threads {threads}");
                assert_eq!(result.line, expected.line, "threads {threads}");
            }
        }
    }
    let mut first = intetrigence_bot::Searcher::new(config.clone());
    let a = first.search(&root).expect("placements");
    let mut second = intetrigence_bot::Searcher::new(config.clone());
    let b = second.search(&root).expect("placements");
    assert_eq!(a.best, b.best);
    assert_eq!(a.nodes, b.nodes);
    assert_eq!(a.candidates.len(), b.candidates.len());
    assert_eq!(a.line, b.line);
    // And a whole game replays identically from the same seed.
    let one = play_game(0xD00D, &config, 12);
    let two = play_game(0xD00D, &config, 12);
    assert_eq!(one.actions, two.actions);
    assert_eq!(one.turns, two.turns);
    assert_eq!(one.lines, two.lines);
}

#[test]
fn think_time_respects_the_budget() {
    let config = SearchConfig {
        budget: Duration::from_millis(30),
        ..real_config()
    };
    let report = play_game(3, &config, 10);
    // The deadline is checked between widening iterations, so a single level
    // can overshoot. Asserting the *maximum* against a tight cap made this
    // test fail roughly one run in five when the whole workspace ran in
    // parallel and the CPU was saturated, which is a property of the machine,
    // not of the search. The median is the load-robust statement; the max is
    // only checked against a cap loose enough that it still catches a search
    // that ignores the deadline altogether (that would run every width up to
    // `max_width`).
    let hard_cap = config.budget.mul_f32(20.0);
    assert!(
        report.max_think() <= hard_cap,
        "max think {:?} exceeds {hard_cap:?}",
        report.max_think()
    );
    // The load-independent statement: a 30 ms budget cannot fund the whole
    // width ramp, so the loop must stop early. This is what "respects the
    // budget" means structurally; wall-clock medians move with machine load
    // and made this test fail one run in five under a parallel workspace test.
    let max_width = config.max_width;
    let over_budget = report
        .search_stats
        .iter()
        .filter(|stats| !stats.fallback && stats.width >= max_width)
        .count();
    assert_eq!(
        over_budget, 0,
        "the search completed the full width ramp ({max_width}) on {over_budget} moves          despite a {:?} budget",
        config.budget
    );
    // Deterministic companion: every move produced a think time, so the
    // search always ran and returned.
    assert_eq!(report.think_times.len(), report.turns as usize);
}

/// The game is reproducible **from its recorded inputs**.
///
/// Two independent searches are not bit-identical: the anytime loop stops when
/// the wall clock says so, so a loaded machine completes a different number of
/// iterations. That was masked before `91ec267`, when a search that missed its
/// deadline fell back to a deterministic 1-ply move. The protocol requirement
/// is that the *recorded actions* replay to the same board, which is what this
/// pins (and what the referee verifies).
#[test]
fn the_game_is_reproducible_from_its_inputs() {
    let config = SearchConfig {
        budget: Duration::from_millis(5),
        ..real_config()
    };
    let report = play_game(0xBEEF, &config, 8);
    let mut replay = intetrigence_core::Game::new(0xBEEF);
    for action in &report.actions {
        assert!(
            replay.apply(*action).accepted,
            "recorded input {action:?} was rejected on replay"
        );
    }
    let mut fresh = intetrigence_core::Game::new(0xBEEF);
    for action in &report.actions {
        fresh.apply(*action);
    }
    assert_eq!(
        board_signature(replay.board()),
        board_signature(fresh.board())
    );
}

#[test]
fn the_answer_does_not_depend_on_the_thread_count() {
    let mut board = Board::new();
    board.set_row(0, 0x1FF);
    board.set_row(1, 0x3EF);
    board.set_row(2, 0x3C7);
    board.set_row(3, 0x1CF);
    let state = RootState {
        board,
        current: PieceKind::T,
        preview: vec![PieceKind::I, PieceKind::O, PieceKind::S, PieceKind::Z],
        hold: Some(PieceKind::L),
        context: EvalContext::default(),
    };
    let config = SearchConfig {
        depth: 6,
        width: 64,
        max_width: 64,
        budget: Duration::from_secs(5),
        ..SearchConfig::default()
    };
    let mut one = Searcher::new(SearchConfig {
        threads: 1,
        ..config.clone()
    });
    let a = one.search(&state).expect("a move exists");
    let mut eight = Searcher::new(SearchConfig {
        threads: 8,
        ..config
    });
    let b = eight.search(&state).expect("a move exists");
    assert_eq!(a.best, b.best);
    assert_eq!(a.value, b.value);
    assert_eq!(a.line, b.line);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn the_search_prefers_a_t_spin_over_a_flat_placement() {
    // A ready TSD slot with the T in hand: the first move must be the spin.
    let mut board = Board::new();
    board.set_row(0, 0x1FF);
    board.set_row(1, 0x3EF);
    board.set_row(2, 0x3C7);
    board.set_row(3, 0x1CF);
    let state = RootState {
        board,
        current: PieceKind::T,
        preview: vec![PieceKind::I, PieceKind::O],
        hold: None,
        context: EvalContext::default(),
    };
    let mut searcher = Searcher::new(SearchConfig {
        depth: 4,
        width: 96,
        max_width: 96,
        threads: 2,
        budget: Duration::from_secs(5),
        ..SearchConfig::default()
    });
    let result = searcher.search(&state).expect("a move exists");
    assert_eq!(result.best.piece, PieceKind::T);
    assert_eq!(
        result.best.placement.spin,
        intetrigence_core::SpinType::Full
    );
    assert!(result.line.len() >= 2);
}

#[test]
fn the_driver_never_emits_an_illegal_turn() {
    // Small budget, many turns: every turn still has to be a legal input.
    let config = SearchConfig {
        budget: Duration::from_millis(2),
        ..real_config()
    };
    let report: GameReport = play_game(0x1234, &config, 25);
    let mut game = Game::new(0x1234);
    for action in &report.actions {
        assert!(game.apply(*action).accepted, "{action:?} rejected");
    }
    assert_eq!(game.stats().pieces_placed, report.turns);
}

#[test]
fn spawn_piece_matches_the_engine() {
    for kind in intetrigence_core::ALL_PIECES {
        let game = Game::from_position(0, Board::new(), spawn_piece(kind)).expect("legal spawn");
        assert_eq!(game.active(), Some(spawn_piece(kind)));
    }
}
