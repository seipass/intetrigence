//! Diagnostic: the per-leaf cost, split into its parts.
//!
//! `search_timing.rs` showed the budget goes almost entirely into the
//! per-child work (~12k leaves x ~6 us). This splits that 6 us so the
//! optimisation targets the part that actually costs.
//!
//! ```text
//! cargo test --release -p intetrigence-bot --test leaf_timing -- --ignored --nocapture
//! ```

use intetrigence_bot::{EvalContext, Weights, analyse, apply_placement, leaf_value};
use intetrigence_core::{Board, PieceKind};
use std::time::Instant;

fn mid_game_board() -> Board {
    let mut board = Board::new();
    for row in 0..6u16 {
        let hole = (row * 3 + 1) % 10;
        board.set_row(row as usize, Board::FULL_ROW & !(1u16 << hole));
    }
    board
}

#[test]
#[ignore = "diagnostic"]
fn per_leaf_cost_breakdown() {
    if cfg!(debug_assertions) {
        panic!("run with --release");
    }
    let board = mid_game_board();
    let weights = Weights::default();
    let context = EvalContext::default();
    let placements = intetrigence_bot::generate(&board, PieceKind::T);
    let n = placements.len() as u32;
    println!("{} placements", n);

    let reps = 200u32;
    let time = |label: &str, mut f: Box<dyn FnMut() + '_>| {
        let start = Instant::now();
        for _ in 0..reps {
            f();
        }
        println!("{label:34} {:?}/call", start.elapsed() / reps);
    };

    time(
        "board.clone()",
        Box::new(|| {
            std::hint::black_box(board.clone());
        }),
    );
    time(
        "analyse(board)",
        Box::new(|| {
            std::hint::black_box(analyse(&board));
        }),
    );
    time(
        "apply_placement (one)",
        Box::new(|| {
            std::hint::black_box(apply_placement(&board, &placements[0], &context, &weights));
        }),
    );
    time(
        "leaf_value (one)",
        Box::new(|| {
            std::hint::black_box(leaf_value(&board, &context, &weights, 1));
        }),
    );
    time(
        "t_slot_cutout (one)",
        Box::new(|| {
            std::hint::black_box(intetrigence_bot::t_slot_cutout(&board, 1, &weights));
        }),
    );
    time(
        "apply_placement + leaf (one child)",
        Box::new(|| {
            let (next, _, _) = apply_placement(&board, &placements[0], &context, &weights);
            std::hint::black_box(leaf_value(&next, &context, &weights, 1));
        }),
    );
    time(
        "all children (apply + leaf)",
        Box::new(|| {
            for placement in &placements {
                let (next, _, _) = apply_placement(&board, placement, &context, &weights);
                std::hint::black_box(leaf_value(&next, &context, &weights, 1));
            }
        }),
    );
}
