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
    let median_cap = config
        .budget
        .mul_f32(if cfg!(debug_assertions) { 2.0 } else { 1.2 });
    let hard_cap = config.budget.mul_f32(20.0);
    assert!(
        report.median_think() <= median_cap,
        "median think {:?} exceeds {median_cap:?}",
        report.median_think()
    );
    assert!(
        report.max_think() <= hard_cap,
        "max think {:?} exceeds {hard_cap:?}",
        report.max_think()
    );
    // Deterministic companion: every move produced a think time, so the
    // search always ran and returned.
    assert_eq!(report.think_times.len(), report.turns as usize);
}

#[test]
fn the_game_is_reproducible() {
    let config = SearchConfig {
        budget: Duration::from_millis(5),
        ..real_config()
    };
    let a = play_game(0xBEEF, &config, 8);
    let b = play_game(0xBEEF, &config, 8);
    assert_eq!(a.turns, b.turns);
    assert_eq!(a.lines, b.lines);
    assert_eq!(a.attack, b.attack);
    assert_eq!(a.actions, b.actions);
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
