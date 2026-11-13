//! Operation legality, determinism, reachability and game-over behaviour.

use std::collections::{HashMap, VecDeque};

use intetrigence_core::{
    ALL_PIECES, Action, ActivePiece, Bag, Board, Cell, Game, PieceKind, Rotation, SplitMix64,
};

#[test]
fn move_stops_at_left_wall() {
    let mut game = Game::new(0);
    let mut guard = 0;
    while game.move_left() {
        guard += 1;
        assert!(guard < 20);
    }

    let piece = game.active().unwrap();
    for (x, y) in piece.cells() {
        assert!((0..10).contains(&x));
        assert!((0..40).contains(&y));
        assert!(!game.board().occupied_at(x as usize, y as usize));
    }
    assert!(!game.move_left());
}

#[test]
fn move_stops_at_right_wall() {
    let mut game = Game::new(0);
    let mut guard = 0;
    while game.move_right() {
        guard += 1;
        assert!(guard < 20);
    }

    let piece = game.active().unwrap();
    for (x, y) in piece.cells() {
        assert!((0..10).contains(&x));
        assert!((0..40).contains(&y));
        assert!(!game.board().occupied_at(x as usize, y as usize));
    }
    assert!(!game.move_right());
}

#[test]
fn soft_drop_stops_when_grounded() {
    let mut game = Game::new(0);
    let mut guard = 0;
    while game.soft_drop() {
        guard += 1;
        assert!(guard < 40);
    }

    let piece = game.active().unwrap();
    assert!(
        !game
            .board()
            .can_place(piece.kind, piece.rotation, piece.x, piece.y - 1)
    );
}

#[test]
fn hard_drop_distance_matches_ghost() {
    let mut game = Game::new(0);
    let start_y = game.active().unwrap().y;
    let ghost = game.ghost_y().unwrap();

    let result = game.hard_drop().unwrap();
    assert_eq!(result.lock.piece.y, ghost);
    assert_eq!(result.distance, (start_y - ghost) as u32);
}

#[test]
fn hard_drop_lands_on_the_stack() {
    let mut board = Board::new();
    board.set_cell(4, 0, Cell::Block);

    let active = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::Zero,
        x: 4,
        y: 20,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    let result = game.hard_drop().unwrap();
    assert_eq!(result.lock.piece.y, 1);
    assert_eq!(result.distance, 19);
    assert_eq!(result.lock.clear.lines, 0);

    assert!(game.board().occupied_at(4, 0));
    assert!(game.board().occupied_at(4, 1));
    assert!(game.board().occupied_at(5, 1));
    assert!(game.board().occupied_at(4, 2));
    assert!(game.board().occupied_at(5, 2));
}

#[test]
fn hold_is_limited_to_once_per_lock() {
    let mut game = Game::new(0);
    let first = game.active().unwrap().kind;

    assert!(game.hold());
    assert_eq!(game.held_piece(), Some(first));
    assert!(!game.can_hold());
    assert!(!game.hold());

    game.hard_drop().unwrap();
    assert!(game.can_hold());
    assert!(game.hold());
}

#[test]
fn empty_hold_takes_the_next_preview_piece() {
    let mut game = Game::new(0);
    let first = game.active().unwrap().kind;
    let next = game.preview(1)[0];

    assert!(game.hold());
    assert_eq!(game.held_piece(), Some(first));
    assert_eq!(game.active().unwrap().kind, next);
}

#[test]
fn hold_returns_the_previously_held_piece() {
    let mut game = Game::new(0);
    let first = game.active().unwrap().kind;

    assert!(game.hold());
    game.hard_drop().unwrap();
    let third = game.active().unwrap().kind;

    assert!(game.hold());
    assert_eq!(game.active().unwrap().kind, first);
    assert_eq!(game.held_piece(), Some(third));
}

fn play(seed: u64, actions: &[Action]) -> Game {
    let mut game = Game::new(seed);
    for &action in actions {
        if game.is_over() {
            break;
        }
        game.apply(action);
    }
    game
}

#[test]
fn replay_is_deterministic_from_seed_and_actions() {
    let mut rng = SplitMix64::new(0xC0FF_EE00_1234_5678);
    let actions: Vec<Action> = (0..800)
        .map(|_| Action::ALL[rng.below(Action::ALL.len() as u64) as usize])
        .collect();

    let a = play(1234, &actions);
    let b = play(1234, &actions);
    assert_eq!(a, b);

    let c = play(1235, &actions);
    assert_ne!(a, c);
}

#[test]
fn no_illegal_placements_under_random_play() {
    for seed in 0..40u64 {
        let mut game = Game::new(seed);
        let mut rng = SplitMix64::new(seed ^ 0xA5A5_A5A5_A5A5_A5A5);

        for step in 0..300 {
            if game.is_over() {
                break;
            }
            let action = Action::ALL[rng.below(Action::ALL.len() as u64) as usize];
            let result = game.apply(action);

            game.check_invariants().unwrap_or_else(|error| {
                panic!("seed {seed} step {step} action {action:?}: {error:?}")
            });

            if let Some(piece) = game.active() {
                for (x, y) in piece.cells() {
                    assert!((0..10).contains(&x), "seed {seed} step {step}: x={x}");
                    assert!((0..40).contains(&y), "seed {seed} step {step}: y={y}");
                    assert!(
                        !game.board().occupied_at(x as usize, y as usize),
                        "seed {seed} step {step}: active cell ({x}, {y}) overlaps"
                    );
                }
            }

            if let Some(hard_drop) = result.hard_drop {
                for (x, y) in hard_drop.lock.piece.cells() {
                    assert!((0..10).contains(&x));
                    assert!((0..40).contains(&y));
                }
            }
        }
    }
}

type StateKey = (i32, i32, Rotation);

/// BFS over the same inputs the bot will use. Hard drop and hold are excluded
/// because they end the piece's movement phase.
fn bfs_from_spawn(kind: PieceKind) -> HashMap<StateKey, (Option<StateKey>, Option<Action>)> {
    let (x, y) = kind.spawn_position();
    let start = ActivePiece {
        kind,
        rotation: Rotation::Zero,
        x,
        y,
    };

    let mut parents = HashMap::new();
    let mut queue = VecDeque::new();
    parents.insert((x, y, Rotation::Zero), (None, None));
    queue.push_back(start);

    while let Some(state) = queue.pop_front() {
        let game = Game::from_position(0, Board::new(), state).expect("state must be legal");
        for action in [
            Action::MoveLeft,
            Action::MoveRight,
            Action::SoftDrop,
            Action::RotateCw,
            Action::RotateCcw,
            Action::Rotate180,
        ] {
            let mut next_game = game.clone();
            if !next_game.apply(action).accepted {
                continue;
            }
            let Some(next) = next_game.active() else {
                continue;
            };
            let key = (next.x, next.y, next.rotation);
            if let std::collections::hash_map::Entry::Vacant(entry) = parents.entry(key) {
                entry.insert((Some((state.x, state.y, state.rotation)), Some(action)));
                queue.push_back(next);
            }
        }
    }

    parents
}

#[test]
fn every_reachable_state_is_legal_and_grounded_states_hard_drop_in_place() {
    for kind in ALL_PIECES {
        let parents = bfs_from_spawn(kind);
        assert!(parents.len() > 1, "{kind:?}");

        for &(x, y, rotation) in parents.keys() {
            let state = ActivePiece {
                kind,
                rotation,
                x,
                y,
            };
            assert!(
                Board::new().can_place(kind, rotation, x, y),
                "illegal reachable state {state:?}"
            );

            let grounded = !Board::new().can_place(kind, rotation, x, y - 1);
            if grounded {
                let mut game = Game::from_position(0, Board::new(), state).unwrap();
                let result = game.hard_drop().unwrap();
                assert_eq!(result.distance, 0, "{state:?}");
                assert_eq!(result.lock.piece, state, "{state:?}");
            }
        }
    }
}

fn path_to(
    parents: &HashMap<StateKey, (Option<StateKey>, Option<Action>)>,
    target: StateKey,
) -> Vec<Action> {
    let mut path = Vec::new();
    let mut current = target;
    loop {
        let (parent, action) = parents[&current];
        match (parent, action) {
            (Some(parent), Some(action)) => {
                path.push(action);
                current = parent;
            }
            (None, None) => break,
            _ => panic!("malformed parent table"),
        }
    }
    path.reverse();
    path
}

#[test]
fn kick_only_placement_has_a_replayable_action_sequence() {
    let kind = PieceKind::J;
    let parents = bfs_from_spawn(kind);
    let target = (0, 0, Rotation::Two);
    assert!(
        parents.contains_key(&target),
        "J wall-kick target not reachable"
    );

    let path = path_to(&parents, target);
    let (x, y) = kind.spawn_position();
    let mut game = Game::from_position(
        0,
        Board::new(),
        ActivePiece {
            kind,
            rotation: Rotation::Zero,
            x,
            y,
        },
    )
    .unwrap();

    for action in &path {
        assert!(
            game.apply(*action).accepted,
            "replay action {action:?} was rejected"
        );
    }
    assert_eq!(
        game.active(),
        Some(ActivePiece {
            kind,
            rotation: Rotation::Two,
            x: 0,
            y: 0,
        })
    );
}

#[test]
fn block_out_ends_the_game() {
    let mut board = Board::new();
    for y in 20..=22 {
        for x in 3..=6 {
            board.set_cell(x, y, Cell::Block);
        }
    }

    let active = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::Zero,
        x: 4,
        y: 0,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    let result = game.lock().unwrap();
    assert!(result.topped_out);
    assert!(game.is_over());
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn locking_above_the_visible_field_is_not_a_lock_out() {
    // S2 TL has lock-out disabled. O at the far left locks entirely in the
    // buffer, but the next spawn (columns 3-5) is still free.
    let active = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::Zero,
        x: 0,
        y: 20,
    };
    let mut game = Game::from_position(0, Board::new(), active).unwrap();

    let result = game.lock().unwrap();
    assert!(!result.topped_out);
    assert!(!game.is_over());
    assert!(game.active().is_some());
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn clutch_clear_pushes_the_next_piece_up() {
    // Row 0 clears; a block shifts to row 21 and blocks the next piece's
    // normal spawn. Clutch pushes it up one row.
    let next = Bag::new(0).next_piece();
    let (spawn_x, spawn_y) = next.spawn_position();
    let (cell_x, _) = next
        .cells(Rotation::Zero)
        .iter()
        .find(|&&(_, cell_y)| spawn_y + cell_y == 21)
        .copied()
        .expect("every piece has a spawn cell on row 21");
    let block_x = (spawn_x + cell_x) as usize;

    let mut board = Board::new();
    for x in 4..10 {
        board.set_cell(x, 0, Cell::Block);
    }
    board.set_cell(block_x, 22, Cell::Block);

    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        x: 0,
        y: -2,
    };
    let mut game = Game::from_position(0, board, active).unwrap();

    let result = game.lock().unwrap();
    assert_eq!(result.clear.lines, 1);
    assert!(!result.topped_out);
    assert_eq!(game.active().unwrap().kind, next);
    assert_eq!(game.active().unwrap().y, spawn_y + 1);
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn clutch_clear_fails_when_no_push_up_position_is_free() {
    // Seed 1's next piece is T (two rows). With rows 19..38 blocked after the
    // clear, T cannot fit even at the freed top row. (Seed 0 would spawn a
    // horizontal I, which does fit on row 39.)
    let mut board = Board::new();
    for x in 4..10 {
        board.set_cell(x, 0, Cell::Block);
    }
    for y in 20..40 {
        for x in 3..=6 {
            board.set_cell(x, y, Cell::Block);
        }
    }

    let active = ActivePiece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        x: 0,
        y: -2,
    };
    let mut game = Game::from_position(1, board, active).unwrap();

    let result = game.lock().unwrap();
    assert_eq!(result.clear.lines, 1);
    assert!(result.topped_out);
    assert!(game.is_over());
    assert_eq!(game.check_invariants(), Ok(()));
}

#[test]
fn actions_after_game_over_are_rejected() {
    let active = ActivePiece {
        kind: PieceKind::O,
        rotation: Rotation::Zero,
        x: 4,
        y: 20,
    };
    let mut game = Game::from_position(0, Board::new(), active).unwrap();
    game.lock().unwrap();
    assert!(game.is_over());

    for action in Action::ALL {
        let result = game.apply(action);
        assert!(!result.accepted);
        assert!(result.hard_drop.is_none());
    }
}

#[test]
fn preview_returns_five_pieces() {
    let mut game = Game::new(0);
    assert_eq!(game.preview(5).len(), 5);
}
