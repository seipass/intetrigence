use intetrigence::game::{attack, guideline_attack, Game, RulesProfile, PIECES};
use intetrigence_engine::{data::*, movegen::find_moves};
use rand::SeedableRng;
fn mv(piece: Piece, rotation: Rotation, x: i8, y: i8) -> Placement {
    Placement {
        location: PieceLocation {
            piece,
            rotation,
            x,
            y,
        },
        spin: Spin::None,
    }
}
fn well(g: &mut Game, height: u32) {
    g.board.cols = [(1 << height) - 1; 10];
    g.board.cols[4] = 0;
    g.queue[0] = Piece::I;
}
#[test]
fn seven_bag_has_exactly_seven_types() {
    let mut g = Game::new(29);
    let mut all = Vec::new();
    for _ in 0..70 {
        all.push(g.queue.pop_front().unwrap());
        g.refill();
    }
    for bag in all.chunks(7) {
        for p in PIECES {
            assert_eq!(bag.iter().filter(|&&v| v == p).count(), 1);
        }
    }
}
#[test]
fn tetris_pc_and_garbage_cancellation() {
    let mut g = Game::new(0);
    well(&mut g, 4);
    g.pending.extend([3; 12]);
    let o = g.play(mv(Piece::I, Rotation::East, 4, 2)).unwrap();
    assert_eq!((o.lines, o.attack, o.sent, o.cancelled), (4, 10, 0, 10));
    assert!(o.perfect_clear);
    assert_eq!(g.pending.len(), 2);
    assert_eq!(g.combo, 1);
    assert!(g.b2b);
}
#[test]
fn combo_increments_resets_b2b_survives_no_clear() {
    let mut g = Game::new(0);
    well(&mut g, 4);
    g.play(mv(Piece::I, Rotation::East, 4, 2)).unwrap();
    well(&mut g, 4);
    g.play(mv(Piece::I, Rotation::East, 4, 2)).unwrap();
    assert_eq!(g.combo, 2);
    g.queue[0] = Piece::O;
    g.play(mv(Piece::O, Rotation::North, 0, 0)).unwrap();
    assert_eq!(g.combo, 0);
    assert!(g.b2b);
}
#[test]
fn empty_hold_consumes_two_and_used_hold_one() {
    let mut g = Game::new(0);
    g.queue[0] = Piece::T;
    g.queue[1] = Piece::O;
    let third = g.queue[2];
    g.play(mv(Piece::O, Rotation::North, 0, 0)).unwrap();
    assert_eq!(g.hold, Some(Piece::T));
    assert_eq!(g.queue[0], third);
    g.queue[0] = Piece::I;
    let next = g.queue[1];
    g.play(mv(Piece::T, Rotation::North, 5, 0)).unwrap();
    assert_eq!(g.hold, Some(Piece::I));
    assert_eq!(g.queue[0], next);
}
#[test]
fn placement_must_be_reachable_and_piece_available() {
    let g = Game::new(0);
    assert!(!g.legal(mv(g.queue[0], Rotation::North, 4, 15)));
    assert!(!g.legal(mv(g.queue[0], Rotation::North, -5, 0)));
}
#[test]
fn moves_stay_in_bounds_and_rest_on_stack() {
    for seed in 0..20 {
        let mut g = Game::new(seed);
        for turn in 0..50 {
            let moves = find_moves(&g.board, g.queue[0]);
            if moves.is_empty() {
                break;
            }
            for (m, _) in &moves {
                assert!(!m.location.obstructed(&g.board));
                assert_eq!(m.location.drop_distance(&g.board), 0);
            }
            let m = moves[(turn * 17) % moves.len()].0;
            g.play(m).unwrap();
            if g.dead {
                break;
            }
        }
    }
}
#[test]
fn attack_table_and_b2b_combo() {
    assert_eq!(attack(2, Spin::Full, true, 3, false), 6);
    assert_eq!(attack(0, Spin::Full, true, 10, false), 0);
    assert_eq!(attack(4, Spin::None, false, 1, false), 4);
}
#[test]
fn search_combo_counter_fixed() {
    let mut g = Game::new(0);
    well(&mut g, 4);
    let mut s = GameState {
        board: g.board,
        bag: enumset::EnumSet::all(),
        reserve: Piece::O,
        back_to_back: false,
        combo: 2,
    };
    let info = s.advance(Piece::I, mv(Piece::I, Rotation::East, 4, 2));
    assert_eq!(info.combo, 3);
    assert_eq!(s.combo, 3);
}
#[test]
fn garbage_rises_at_most_eight_and_has_holes() {
    let mut g = Game::new(0);
    g.queue[0] = Piece::O;
    g.pending.extend([7; 12]);
    g.play(mv(Piece::O, Rotation::North, 0, 0)).unwrap();
    assert_eq!(g.pending.len(), 4);
    assert_eq!(g.board.cols[7] & 255, 0);
    assert_eq!(g.board.cols[6] & 255, 255);
}

#[test]
fn guideline_profile_applies_remaining_garbage_after_a_line_clear() {
    let mut g = Game::new_with_rules(0, RulesProfile::Guideline);
    well(&mut g, 4);
    g.pending.extend([3; 12]);
    let o = g.play(mv(Piece::I, Rotation::East, 4, 2)).unwrap();
    assert_eq!((o.lines, o.attack, o.cancelled, o.sent), (4, 4, 4, 0));
    assert_eq!(o.garbage_applied, 8);
    assert!(g.pending.is_empty());
}

#[test]
fn replay_board_labels_garbage_separately_from_blocks() {
    let mut g = Game::new_with_rules(0, RulesProfile::Guideline);
    g.queue[0] = Piece::O;
    let mut incoming = rand_chacha::ChaCha8Rng::seed_from_u64(123);
    g.receive(1, &mut incoming);
    g.play(mv(Piece::O, Rotation::North, 0, 0)).unwrap();

    let cells = g.board_cells_json();
    let bottom = cells[0].as_array().unwrap();
    let above = cells[1].as_array().unwrap();
    assert_eq!(bottom.iter().filter(|cell| *cell == "garbage").count(), 9);
    assert_eq!(above.iter().filter(|cell| *cell == "block").count(), 2);
}

#[test]
fn guideline_attack_uses_the_public_line_table_without_pc_or_combo_bonus() {
    assert_eq!(guideline_attack(2, Spin::None, false, true), 1);
    assert_eq!(guideline_attack(4, Spin::None, false, true), 4);
    assert_eq!(guideline_attack(4, Spin::None, true, false), 5);
}

#[test]
fn guideline_garbage_hole_stays_for_eight_appearing_rows() {
    let mut g = Game::new_with_rules(0, RulesProfile::Guideline);
    let mut incoming = rand_chacha::ChaCha8Rng::seed_from_u64(77);
    g.queue[0] = Piece::O;
    g.receive(8, &mut incoming);
    g.play(mv(Piece::O, Rotation::North, 0, 0)).unwrap();
    let first_hole = (0..10)
        .find(|&x| g.board.cols[x] & 1 == 0)
        .expect("first garbage row has one hole");

    g.queue[0] = Piece::O;
    g.receive(8, &mut incoming);
    g.play(mv(Piece::O, Rotation::North, 5, 8)).unwrap();
    let first_block_hole = (0..10)
        .find(|&x| g.board.cols[x] & (1 << 8) == 0)
        .expect("first garbage block remains in row eight");
    let second_block_hole = (0..10)
        .find(|&x| g.board.cols[x] & 1 == 0)
        .expect("second garbage block has one hole");
    assert_eq!(first_block_hole, first_hole);
    assert_ne!(second_block_hole, first_hole);
}

#[test]
fn guideline_cancelled_rows_do_not_advance_the_garbage_hole_block() {
    let mut g = Game::new_with_rules(0, RulesProfile::Guideline);
    let mut incoming = rand_chacha::ChaCha8Rng::seed_from_u64(91);
    g.queue[0] = Piece::O;
    g.receive(8, &mut incoming);
    g.pending.drain(..4);
    g.play(mv(Piece::O, Rotation::North, 0, 0)).unwrap();
    let first_hole = (0..10)
        .find(|&x| g.board.cols[x] & 1 == 0)
        .expect("first garbage row has one hole");

    g.queue[0] = Piece::O;
    g.receive(4, &mut incoming);
    g.play(mv(Piece::O, Rotation::North, 5, 4)).unwrap();
    for y in 0..8 {
        let hole = (0..10)
            .find(|&x| g.board.cols[x] & (1 << y) == 0)
            .expect("garbage row has one hole");
        assert_eq!(hole, first_hole, "appearing row {y} changed the block hole");
    }
}
#[test]
fn noncontiguous_line_compaction() {
    let mut b = Board {
        cols: [0b10101; 10],
    };
    b.cols[0] |= 0b100010;
    let lines = b.line_clears();
    b.remove_lines(lines);
    assert_eq!(b.cols[0], 0b101);
    assert_eq!(b.cols[1], 0);
}
