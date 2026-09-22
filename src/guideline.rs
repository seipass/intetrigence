//! Independent placement-level reference for the public Guideline mechanics.
//!
//! The search engine has its own move generator. This module deliberately does
//! not call it: it checks spawn, SRS kicks, collision, gravity and hard-drop
//! reachability against the engine output in the integration tests.
use intetrigence_engine::data::{Board, Piece, PieceLocation, Placement, Rotation, Spin};
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct State {
    x: i8,
    y: i8,
    rotation: Rotation,
    last_was_rotation: bool,
}

const fn base_cells(piece: Piece) -> [(i8, i8); 4] {
    match piece {
        Piece::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
        Piece::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
        Piece::T => [(-1, 0), (0, 0), (1, 0), (0, 1)],
        Piece::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
        Piece::J => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
        Piece::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
        Piece::Z => [(-1, 1), (0, 1), (0, 0), (1, 0)],
    }
}

const fn rotate(rotation: Rotation, (x, y): (i8, i8)) -> (i8, i8) {
    match rotation {
        Rotation::North => (x, y),
        Rotation::East => (y, -x),
        Rotation::South => (-x, -y),
        Rotation::West => (-y, x),
    }
}

fn cells(piece: Piece, rotation: Rotation, x: i8, y: i8) -> [(i8, i8); 4] {
    let base = base_cells(piece);
    [
        rotate(rotation, base[0]),
        rotate(rotation, base[1]),
        rotate(rotation, base[2]),
        rotate(rotation, base[3]),
    ]
    .map(|(dx, dy)| (x + dx, y + dy))
}

fn obstructed(board: &Board, piece: Piece, rotation: Rotation, x: i8, y: i8) -> bool {
    cells(piece, rotation, x, y).into_iter().any(|(x, y)| {
        x < 0 || x >= 10 || y < 0 || y >= 40 || board.cols[x as usize] & (1 << y) != 0
    })
}

/// Return whether a location overlaps the wall, floor, ceiling, or board.
pub fn is_obstructed(board: &Board, location: PieceLocation) -> bool {
    obstructed(
        board,
        location.piece,
        location.rotation,
        location.x,
        location.y,
    )
}

/// Apply one horizontal movement when the target location is valid.
pub fn try_shift(board: &Board, location: PieceLocation, dx: i8) -> Option<PieceLocation> {
    let next = PieceLocation {
        x: location.x + dx,
        ..location
    };
    if is_obstructed(board, next) {
        None
    } else {
        Some(next)
    }
}

/// Apply one SRS clockwise or counter-clockwise rotation and retain the
/// T-spin classification produced by the successful kick.
pub fn try_rotate_placement(
    board: &Board,
    location: PieceLocation,
    clockwise: bool,
) -> Option<Placement> {
    if location.piece == Piece::O {
        return None;
    }
    let next_rotation = if clockwise {
        rotate_cw(location.rotation)
    } else {
        rotate_ccw(location.rotation)
    };
    for (kick_index, (dx, dy)) in kick(location.piece, location.rotation, next_rotation)
        .into_iter()
        .enumerate()
    {
        let next = PieceLocation {
            rotation: next_rotation,
            x: location.x + dx,
            y: location.y + dy,
            ..location
        };
        if is_obstructed(board, next) {
            continue;
        }
        let spin = if next.piece != Piece::T {
            Spin::None
        } else {
            let corners = [(-1, -1), (1, -1), (-1, 1), (1, 1)]
                .into_iter()
                .filter(|&(x, y)| board.occupied((next.x + x, next.y + y)))
                .count();
            let front_corners = [(-1, 1), (1, 1)]
                .into_iter()
                .map(|cell| next.rotation.rotate_cell(cell))
                .filter(|&(x, y)| board.occupied((next.x + x, next.y + y)))
                .count();
            if corners < 3 {
                Spin::None
            } else if front_corners == 2 || kick_index == 4 {
                Spin::Full
            } else {
                Spin::Mini
            }
        };
        return Some(Placement {
            location: next,
            spin,
        });
    }
    None
}

/// Apply one SRS clockwise or counter-clockwise rotation. The first valid
/// kick is selected, as required by SRS; O has no positional rotation.
pub fn try_rotate(
    board: &Board,
    location: PieceLocation,
    clockwise: bool,
) -> Option<PieceLocation> {
    try_rotate_placement(board, location, clockwise).map(|placement| placement.location)
}

/// Drop a valid active piece to the lowest unobstructed location.
pub fn hard_drop(board: &Board, mut location: PieceLocation) -> PieceLocation {
    while !is_obstructed(
        board,
        PieceLocation {
            y: location.y - 1,
            ..location
        },
    ) {
        location.y -= 1;
    }
    location
}

fn kick(piece: Piece, from: Rotation, to: Rotation) -> [(i8, i8); 5] {
    if piece == Piece::O {
        return [(0, 0); 5];
    }
    // SRS tables use 0=North, R=East, 2=South, L=West. The engine's
    // Rotation enum is ordered North, West, South, East, so the transitions
    // are spelled out rather than indexed by enum discriminants.
    if piece == Piece::I {
        // The I piece rotates around its half-cell pivot. These are the
        // Guideline tests expressed in the same location coordinates as the
        // piece cell tables below (the orientation origins therefore move
        // between states).
        match (from, to) {
            (Rotation::North, Rotation::East) => [(1, 0), (-1, 0), (2, 0), (-1, -1), (2, 2)],
            (Rotation::East, Rotation::North) => [(-1, 0), (1, 0), (-2, 0), (1, 1), (-2, -2)],
            (Rotation::East, Rotation::South) => [(0, -1), (-1, -1), (2, -1), (-1, 1), (2, -2)],
            (Rotation::South, Rotation::East) => [(0, 1), (1, 1), (-2, 1), (1, -1), (-2, 2)],
            (Rotation::South, Rotation::West) => [(-1, 0), (1, 0), (-2, 0), (1, 1), (-2, -2)],
            (Rotation::West, Rotation::South) => [(1, 0), (-1, 0), (2, 0), (-1, -1), (2, 2)],
            (Rotation::West, Rotation::North) => [(0, 1), (1, 1), (-2, 1), (1, -1), (-2, 2)],
            (Rotation::North, Rotation::West) => [(0, -1), (-1, -1), (2, -1), (-1, 1), (2, -2)],
            _ => [(0, 0); 5],
        }
    } else {
        match (from, to) {
            (Rotation::North, Rotation::East) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            (Rotation::East, Rotation::North) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (Rotation::East, Rotation::South) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (Rotation::South, Rotation::East) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            (Rotation::South, Rotation::West) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            (Rotation::West, Rotation::South) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (Rotation::West, Rotation::North) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (Rotation::North, Rotation::West) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            _ => [(0, 0); 5],
        }
    }
}

fn rotate_cw(rotation: Rotation) -> Rotation {
    match rotation {
        Rotation::North => Rotation::East,
        Rotation::East => Rotation::South,
        Rotation::South => Rotation::West,
        Rotation::West => Rotation::North,
    }
}

fn rotate_ccw(rotation: Rotation) -> Rotation {
    match rotation {
        Rotation::North => Rotation::West,
        Rotation::West => Rotation::South,
        Rotation::South => Rotation::East,
        Rotation::East => Rotation::North,
    }
}

fn canonical(mut location: PieceLocation) -> PieceLocation {
    match location.piece {
        Piece::T | Piece::J | Piece::L => location,
        Piece::O => {
            match location.rotation {
                Rotation::North => {}
                Rotation::East => location.y -= 1,
                Rotation::South => {
                    location.x -= 1;
                    location.y -= 1;
                }
                Rotation::West => location.x -= 1,
            }
            location.rotation = Rotation::North;
            location
        }
        Piece::S | Piece::Z => match location.rotation {
            Rotation::North | Rotation::East => location,
            Rotation::South => {
                location.rotation = Rotation::North;
                location.y -= 1;
                location
            }
            Rotation::West => {
                location.rotation = Rotation::East;
                location.x -= 1;
                location
            }
        },
        Piece::I => match location.rotation {
            Rotation::North | Rotation::East => location,
            Rotation::South => {
                location.rotation = Rotation::North;
                location.x -= 1;
                location
            }
            Rotation::West => {
                location.rotation = Rotation::East;
                location.y += 1;
                location
            }
        },
    }
}

/// Return every canonical lock location reachable under SRS from the Guideline
/// spawn point and a hard drop.
pub fn reference_locations(board: &Board, piece: Piece) -> HashSet<PieceLocation> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut result = HashSet::new();
    let starts = if obstructed(board, piece, Rotation::North, 4, 19) {
        if obstructed(board, piece, Rotation::North, 4, 20) {
            return result;
        }
        vec![State {
            x: 4,
            y: 20,
            rotation: Rotation::North,
            last_was_rotation: false,
        }]
    } else {
        vec![State {
            x: 4,
            y: 19,
            rotation: Rotation::North,
            last_was_rotation: false,
        }]
    };
    for state in starts {
        visited.insert(state);
        queue.push_back(state);
    }
    while let Some(state) = queue.pop_front() {
        let mut y = state.y;
        while !obstructed(board, piece, state.rotation, state.x, y - 1) {
            y -= 1;
        }
        result.insert(canonical(PieceLocation {
            piece,
            rotation: state.rotation,
            x: state.x,
            y,
        }));

        let mut push = |next: State| {
            if !obstructed(board, piece, next.rotation, next.x, next.y) && visited.insert(next) {
                queue.push_back(next);
            }
        };
        push(State {
            x: state.x - 1,
            y: state.y,
            rotation: state.rotation,
            last_was_rotation: false,
        });
        push(State {
            x: state.x + 1,
            y: state.y,
            rotation: state.rotation,
            last_was_rotation: false,
        });
        push(State {
            x: state.x,
            y: state.y - 1,
            rotation: state.rotation,
            last_was_rotation: false,
        });
        if piece != Piece::O {
            let cw = rotate_cw(state.rotation);
            for (dx, dy) in kick(piece, state.rotation, cw) {
                let next = State {
                    x: state.x + dx,
                    y: state.y + dy,
                    rotation: cw,
                    last_was_rotation: true,
                };
                if !obstructed(board, piece, next.rotation, next.x, next.y) {
                    if visited.insert(next) {
                        queue.push_back(next);
                    }
                    break;
                }
            }
            let ccw = rotate_ccw(state.rotation);
            for (dx, dy) in kick(piece, state.rotation, ccw) {
                let next = State {
                    x: state.x + dx,
                    y: state.y + dy,
                    rotation: ccw,
                    last_was_rotation: true,
                };
                if !obstructed(board, piece, next.rotation, next.x, next.y) {
                    if visited.insert(next) {
                        queue.push_back(next);
                    }
                    break;
                }
            }
        }
    }
    result
}

/// Validate a placement's board position independently of the search engine.
pub fn is_reachable(board: &Board, location: PieceLocation) -> bool {
    reference_locations(board, location.piece).contains(&canonical(location))
}
