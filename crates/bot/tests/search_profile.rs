//! Diagnostic: what the 100 ms budget actually buys.
//!
//! The widening loop only grows the beam (`width * 3/2` up to `max_width`); the
//! depth is fixed at `config.depth`. If the loop reaches `max_width` before the
//! deadline, the rest of the budget is discarded. This prints the iteration
//! count, the final width, the nodes and the elapsed time so that "raise the
//! width" and "raise the depth" can be compared on evidence.
//!
//! ```text
//! cargo test -p intetrigence-bot --test search_profile -- --ignored --nocapture
//! ```

use intetrigence_bot::{EvalContext, RootState, SearchConfig, Searcher};
use intetrigence_core::{Board, PieceKind};
use std::time::Duration;

/// A mid-game board: 6 rows of garbage with a hole per row.
fn mid_game_board() -> Board {
    let mut board = Board::new();
    for row in 0..6u16 {
        let hole = (row * 3 + 1) % 10;
        board.set_row(row as usize, Board::FULL_ROW & !(1u16 << hole));
    }
    board
}

fn profile(label: &str, config: SearchConfig, board: &Board) {
    let root = RootState {
        board: board.clone(),
        current: PieceKind::T,
        preview: vec![
            PieceKind::I,
            PieceKind::O,
            PieceKind::S,
            PieceKind::Z,
            PieceKind::L,
            PieceKind::J,
            PieceKind::T,
        ],
        hold: Some(PieceKind::L),
        context: EvalContext::default(),
    };
    let mut searcher = Searcher::new(config.clone());
    let mut elapsed = Vec::new();
    let mut widths = Vec::new();
    let mut nodes = Vec::new();
    for _ in 0..5 {
        let Some(result) = searcher.search(&root) else {
            println!("{label}: no placement");
            return;
        };
        // `run_parallel` returns None when a level does not finish before the
        // deadline, which discards the whole iteration; the search then plays
        // the 1-ply greedy move (width 1, nodes 1). Reporting it instead of
        // asserting keeps the diagnostic usable while the fix is pending.
        if result.candidates.len() == 1 && result.nodes == 1 {
            println!("  !! greedy fallback: the anytime loop produced no candidate");
        }
        elapsed.push(result.elapsed);
        widths.push(result.width);
        nodes.push(result.nodes);
    }
    elapsed.sort();
    let median_elapsed = elapsed[elapsed.len() / 2];
    println!(
        "{label:28} depth {:2} width {:3}..{:3} nodes {:5}..{:5} elapsed median {:?} max {:?} budget {:?}",
        config.depth,
        widths.iter().min().copied().unwrap_or(0),
        widths.iter().max().copied().unwrap_or(0),
        nodes.iter().min().copied().unwrap_or(0),
        nodes.iter().max().copied().unwrap_or(0),
        median_elapsed,
        elapsed.last().copied().unwrap_or_default(),
        config.budget
    );
}

#[test]
#[ignore = "diagnostic: minutes of search"]
fn what_the_budget_buys() {
    let board = mid_game_board();
    let base = SearchConfig {
        depth: 8,
        width: 96,
        max_width: 768,
        threads: 8,
        budget: Duration::from_millis(100),
        ..SearchConfig::default()
    };
    profile("baseline d8 w96-768", base.clone(), &board);
    profile(
        "narrow d8 w96-192",
        SearchConfig {
            max_width: 192,
            ..base
        },
        &board,
    );
    profile(
        "deep d12 w96-768",
        SearchConfig { depth: 12, ..base },
        &board,
    );
    profile(
        "deep d16 w96-768",
        SearchConfig { depth: 16, ..base },
        &board,
    );
    profile(
        "wide d8 w96-2048",
        SearchConfig {
            max_width: 2048,
            ..base
        },
        &board,
    );
    profile(
        "single thread d8",
        SearchConfig { threads: 1, ..base },
        &board,
    );
}
