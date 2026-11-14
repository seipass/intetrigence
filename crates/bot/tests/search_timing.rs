//! Diagnostic: where the 100 ms goes.
//!
//! `search_distribution.rs` showed a completed depth of ~2 with `nodes` around
//! 43, which looks impossible next to a beam width of 352. The accounting
//! explains it -- `nodes` counts *expanded beam nodes*, and at depth 2 the beam
//! is only as wide as the root's branching factor (the T has ~42 placements) --
//! but 43 expansions in 100 ms is still 2.3 ms per expansion, which is far too
//! slow for a bitboard evaluation. This times the pieces separately.
//!
//! ```text
//! cargo test -p intetrigence-bot --test search_timing -- --ignored --nocapture
//! ```

use intetrigence_bot::{EvalContext, RootState, SearchConfig, Searcher};
use intetrigence_core::{Board, PieceKind};
use std::time::{Duration, Instant};

fn mid_game_board() -> Board {
    let mut board = Board::new();
    for row in 0..6u16 {
        let hole = (row * 3 + 1) % 10;
        board.set_row(row as usize, Board::FULL_ROW & !(1u16 << hole));
    }
    board
}

fn root() -> RootState {
    RootState {
        board: mid_game_board(),
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
    }
}

#[test]
#[ignore = "diagnostic"]
fn where_the_budget_goes() {
    let root = root();

    // 1. Movegen alone: how many placements does one piece have, and how long?
    let mut movegen = intetrigence_bot::Movegen::new();
    let start = Instant::now();
    let mut count = 0;
    for _ in 0..200 {
        count = movegen.generate(&root.board, PieceKind::T).len();
    }
    println!(
        "movegen: {count} placements, {:?}/call",
        start.elapsed() / 200
    );

    // 2. One full search per thread count, at several widths.
    for threads in [1usize, 8] {
        for width in [96usize, 768] {
            let config = SearchConfig {
                threads,
                width,
                max_width: width,
                depth: 2,
                budget: Duration::from_millis(100),
                ..SearchConfig::default()
            };
            let mut searcher = Searcher::new(config);
            let mut elapsed = Vec::new();
            let mut nodes = Vec::new();
            for _ in 0..5 {
                let result = searcher.search(&root).expect("placements");
                elapsed.push(result.elapsed);
                nodes.push(result.nodes);
            }
            elapsed.sort();
            println!(
                "threads {threads} width {width:3} depth 2: elapsed median {:?} nodes {:?}",
                elapsed[elapsed.len() / 2],
                nodes
            );
        }
    }

    // 3. How much of the budget is thread setup? One search with a huge budget
    //    shows the fixed cost of a single iteration.
    let config = SearchConfig {
        threads: 8,
        width: 96,
        max_width: 96,
        depth: 2,
        budget: Duration::from_secs(5),
        ..SearchConfig::default()
    };
    let mut searcher = Searcher::new(config);
    let start = Instant::now();
    let result = searcher.search(&root).expect("placements");
    println!(
        "single depth-2 iteration with a 5 s budget: {:?} (nodes {}, iterations {})",
        start.elapsed(),
        result.nodes,
        result.iterations
    );
}
