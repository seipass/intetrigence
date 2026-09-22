use intetrigence::guideline::{is_reachable, reference_locations};
use intetrigence_engine::{
    data::{Board, Piece, PieceLocation, Rotation},
    movegen::find_moves,
};
use std::collections::HashSet;

const PIECES: [Piece; 7] = [
    Piece::I,
    Piece::O,
    Piece::T,
    Piece::L,
    Piece::J,
    Piece::S,
    Piece::Z,
];

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 7;
    *state ^= *state >> 9;
    *state ^= *state << 8;
    *state
}

fn board(seed: u64, max_height: u64) -> Board {
    let mut state = seed;
    let mut board = Board::default();
    for x in 0..10 {
        let height = (next(&mut state) % (max_height + 1)) as usize;
        for y in 0..height {
            if next(&mut state) % 5 != 0 {
                board.cols[x] |= 1 << y;
            }
        }
    }
    board
}

#[test]
fn independent_srs_reference_matches_engine_locations() {
    for seed in 1..=40 {
        let board = board(seed * 0x9e37, 12);
        for piece in PIECES {
            let expected = reference_locations(&board, piece);
            let actual: HashSet<_> = find_moves(&board, piece)
                .into_iter()
                .map(|(placement, _)| placement.location)
                .collect();
            assert_eq!(actual, expected, "seed={seed}, piece={piece:?}");
            for location in expected {
                assert!(is_reachable(&board, location));
                assert!(!location.obstructed(&board));
                assert_eq!(location.drop_distance(&board), 0);
            }
        }
    }
    for seed in 100..=112 {
        let board = board(seed * 0x9e37, 24);
        for piece in PIECES {
            let expected = reference_locations(&board, piece);
            let actual: HashSet<_> = find_moves(&board, piece)
                .into_iter()
                .map(|(placement, _)| placement.location)
                .collect();
            assert_eq!(actual, expected, "seed={seed}, piece={piece:?}");
            for location in expected {
                assert!(is_reachable(&board, location));
                assert!(!location.obstructed(&board));
                assert_eq!(location.drop_distance(&board), 0);
            }
        }
    }
}

#[test]
fn reference_rejects_wall_crossing_and_accepts_spawn_hard_drop() {
    let board = Board::default();
    let valid = PieceLocation {
        piece: Piece::T,
        rotation: Rotation::North,
        x: 4,
        y: 0,
    };
    let invalid = PieceLocation {
        piece: Piece::T,
        rotation: Rotation::North,
        x: -2,
        y: 0,
    };
    assert!(is_reachable(&board, valid));
    assert!(!is_reachable(&board, invalid));
}
