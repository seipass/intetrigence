//! Measurement harness for the M2 acceptance numbers.
//!
//! Ignored by default because it takes ~20 s. Run it with
//!
//! ```text
//! cargo test -p intetrigence-bot --release --test timing_report -- --ignored --nocapture
//! ```

use std::time::Duration;

use intetrigence_bot::{Movegen, SearchConfig, play_game};
use intetrigence_core::{ALL_PIECES, Board};

fn high_board() -> Board {
    let mut board = Board::new();
    for y in 0..19 {
        let mut row = Board::FULL_ROW & !(1u16 << 9);
        if y % 3 == 0 {
            row &= !(1u16 << ((y * 7 + 3) % 9));
        }
        board.set_row(y, row);
    }
    board.set_row(19, 0b0000_0001_1100_0000);
    board
}

#[test]
#[ignore = "measurement harness, run with --release --ignored --nocapture"]
fn m2_acceptance_numbers() {
    let mut movegen = Movegen::new();
    for (label, board) in [("empty", Board::new()), ("high-19-rows", high_board())] {
        for kind in ALL_PIECES {
            movegen.generate(&board, kind);
        }
        let iterations = 5000;
        let start = std::time::Instant::now();
        let mut sink = 0usize;
        for _ in 0..iterations {
            for kind in ALL_PIECES {
                sink += movegen.generate(&board, kind).len();
            }
        }
        println!(
            "movegen {label}: {:?}/piece (sink {sink})",
            start.elapsed() / (iterations * 7)
        );
    }

    for (label, threads) in [("100ms-8t", 8usize), ("100ms-1t", 1)] {
        let config = SearchConfig {
            depth: 8,
            width: 96,
            max_width: 768,
            threads,
            budget: Duration::from_millis(100),
            ..SearchConfig::default()
        };
        let report = play_game(0x5EED, &config, 60);
        println!(
            "{label}: turns={} lines={} attack={} median={:?} p95={:?} max={:?}",
            report.turns,
            report.lines,
            report.attack,
            report.median_think(),
            report.p95_think(),
            report.max_think()
        );
    }
}
