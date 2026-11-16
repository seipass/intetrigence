//! SRS+ kick tables for TETR.IO Season 2, isolated so the data can be replaced
//! in one place.
//!
//! # Source
//!
//! `docs/s2-spec.md` §3.1 (SRS+) and §3.2 (180°), v1. J/L/S/T/Z use the
//! standard SRS 90° table; I uses the SRS+ table (mirrored I kicks). 180° uses
//! the S2 table: six candidates for J/L/S/T/Z/O and two for I.
//!
//! The SRS+ I **test order** is marked [単一実装] (U1) in the spec. The offset
//! sets are fixed, but if U1 resolves differently, change only [`I_KICKS`].
//!
//! # Sign convention
//!
//! `(dx, dy)` is in board coordinates: `+x` right, `+y` up. This matches the
//! spec's `Δy > 0 = up` convention. `@haelp/teto` stores y downward, so do not
//! copy its tables without negating y.

use crate::piece::{PieceKind, Rotation};

/// A kick offset in board coordinates (`+x` right, `+y` up).
pub type Kick = (i8, i8);

/// Identity-only table, used for same-state queries.
pub const NO_KICK: [Kick; 1] = [(0, 0)];

/// SRS+ kicks for J, L, S, T and Z (identical to standard SRS).
///
/// Index order: `0->R`, `R->0`, `R->2`, `2->R`, `2->L`, `L->2`, `L->0`, `0->L`.
pub const JLSTZ_KICKS: [[Kick; 5]; 8] = [
    // 0 -> R
    [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
    // R -> 0
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    // R -> 2
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    // 2 -> R
    [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
    // 2 -> L
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
    // L -> 2
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // L -> 0
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    // 0 -> L
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
];

/// SRS+ kicks for I.
///
/// Index order matches [`JLSTZ_KICKS`]. The T2/T3 order is the spec's SRS+
/// order (U1); the offset set is the mirrored SRS set.
pub const I_KICKS: [[Kick; 5]; 8] = [
    // 0 -> R
    [(0, 0), (1, 0), (-2, 0), (-2, -1), (1, 2)],
    // R -> 0
    [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
    // R -> 2
    [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)],
    // 2 -> R
    [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
    // 2 -> L
    [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)],
    // L -> 2
    [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
    // L -> 0
    [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)],
    // 0 -> L
    [(0, 0), (-1, 0), (2, 0), (2, -1), (-1, 2)],
];

/// S2 180-degree kicks for J, L, S, T, Z and O.
///
/// Six candidates per transition. The 6th entry is the final TETR.IO
/// priority level (Hard Drop wiki; S7 `kicks["SRS+"].kicks["02"/"20"/"13"/"31"]`
/// has five explicit entries after the implicit `(0,0)`).
///
/// Index order: `0->2`, `2->0`, `R->L`, `L->R`. O uses this general table for
/// 180s because O has no dedicated 180 table in the spec.
pub const JLSTZ_180_KICKS: [[Kick; 6]; 4] = [
    // 0 -> 2
    [(0, 0), (0, 1), (1, 1), (-1, 1), (1, 0), (-1, 0)],
    // 2 -> 0
    [(0, 0), (0, -1), (-1, -1), (1, -1), (-1, 0), (1, 0)],
    // R -> L
    [(0, 0), (1, 0), (1, 2), (1, 1), (0, 2), (0, 1)],
    // L -> R
    [(0, 0), (-1, 0), (-1, 2), (-1, 1), (0, 2), (0, 1)],
];

/// S2 180-degree kicks for I (two candidates per transition).
///
/// Index order matches [`JLSTZ_180_KICKS`].
pub const I_180_KICKS: [[Kick; 2]; 4] = [
    // 0 -> 2
    [(0, 0), (0, 1)],
    // 2 -> 0
    [(0, 0), (0, -1)],
    // R -> L
    [(0, 0), (1, 0)],
    // L -> R
    [(0, 0), (-1, 0)],
];

/// O has no 90-degree kicks; every rotation state occupies the same cells.
pub const O_KICKS: [Kick; 1] = [(0, 0)];

/// Maps a 90-degree transition to its index in [`JLSTZ_KICKS`] / [`I_KICKS`].
pub const fn kick_index(from: Rotation, to: Rotation) -> Option<usize> {
    Some(match (from, to) {
        (Rotation::Zero, Rotation::R) => 0,
        (Rotation::R, Rotation::Zero) => 1,
        (Rotation::R, Rotation::Two) => 2,
        (Rotation::Two, Rotation::R) => 3,
        (Rotation::Two, Rotation::L) => 4,
        (Rotation::L, Rotation::Two) => 5,
        (Rotation::L, Rotation::Zero) => 6,
        (Rotation::Zero, Rotation::L) => 7,
        _ => return None,
    })
}

/// Maps a 180-degree transition to its index in [`JLSTZ_180_KICKS`] /
/// [`I_180_KICKS`].
pub const fn kick_180_index(from: Rotation, to: Rotation) -> Option<usize> {
    Some(match (from, to) {
        (Rotation::Zero, Rotation::Two) => 0,
        (Rotation::Two, Rotation::Zero) => 1,
        (Rotation::R, Rotation::L) => 2,
        (Rotation::L, Rotation::R) => 3,
        _ => return None,
    })
}

/// Kick candidates for `from -> to`, in SRS+ test order.
///
/// The first entry is always `(0, 0)` for a rotation that changes state.
pub fn kicks(kind: PieceKind, from: Rotation, to: Rotation) -> &'static [Kick] {
    if let Some(index) = kick_index(from, to) {
        return match kind {
            PieceKind::I => &I_KICKS[index],
            PieceKind::O => &O_KICKS,
            _ => &JLSTZ_KICKS[index],
        };
    }

    if let Some(index) = kick_180_index(from, to) {
        return match kind {
            PieceKind::I => &I_180_KICKS[index],
            // O uses the general 180 table (docs/s2-spec.md §3.1.3).
            _ => &JLSTZ_180_KICKS[index],
        };
    }

    &NO_KICK
}

/// True when `kick` is the 5th-kick FIN/TST candidate for this transition.
///
/// `docs/s2-spec.md` §3.3: `(+1, -2)` on `0->L`/`2->L`, and `(-1, -2)` on
/// `0->R`/`2->R`. Spin classification uses this to promote a T-spin with one
/// front corner to full.
pub fn is_fin_or_tst_kick(from: Rotation, to: Rotation, kick: Kick) -> bool {
    match (from, to) {
        (Rotation::Zero, Rotation::L) | (Rotation::Two, Rotation::L) => kick == (1, -2),
        (Rotation::Zero, Rotation::R) | (Rotation::Two, Rotation::R) => kick == (-1, -2),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::ALL_PIECES;

    #[test]
    fn first_kick_is_identity() {
        for kind in ALL_PIECES {
            for from in Rotation::ALL {
                for to in Rotation::ALL {
                    if from == to {
                        continue;
                    }
                    assert_eq!(
                        kicks(kind, from, to)[0],
                        (0, 0),
                        "{kind:?} {from:?}->{to:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn quarter_turn_kick_tables_are_symmetric() {
        for kind in ALL_PIECES {
            for from in Rotation::ALL {
                for to in Rotation::ALL {
                    if kick_index(from, to).is_none() {
                        continue;
                    }
                    let forward = kicks(kind, from, to);
                    let reverse = kicks(kind, to, from);
                    for &(dx, dy) in forward {
                        assert!(
                            reverse.contains(&(-dx, -dy)),
                            "{kind:?} {from:?}->{to:?}: ({dx}, {dy}) has no reverse"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn srs_plus_jlstz_zero_to_r_table() {
        assert_eq!(
            kicks(PieceKind::T, Rotation::Zero, Rotation::R),
            &[(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)]
        );
    }

    #[test]
    fn srs_plus_i_zero_to_r_table() {
        assert_eq!(
            kicks(PieceKind::I, Rotation::Zero, Rotation::R),
            &[(0, 0), (1, 0), (-2, 0), (-2, -1), (1, 2)]
        );
    }

    #[test]
    fn jlstz_180_table_matches_spec() {
        assert_eq!(
            kicks(PieceKind::T, Rotation::Zero, Rotation::Two),
            &[(0, 0), (0, 1), (1, 1), (-1, 1), (1, 0), (-1, 0)]
        );
        assert_eq!(
            kicks(PieceKind::T, Rotation::Two, Rotation::Zero),
            &[(0, 0), (0, -1), (-1, -1), (1, -1), (-1, 0), (1, 0)]
        );
        assert_eq!(
            kicks(PieceKind::T, Rotation::R, Rotation::L),
            &[(0, 0), (1, 0), (1, 2), (1, 1), (0, 2), (0, 1)]
        );
        assert_eq!(
            kicks(PieceKind::T, Rotation::L, Rotation::R),
            &[(0, 0), (-1, 0), (-1, 2), (-1, 1), (0, 2), (0, 1)]
        );
    }

    #[test]
    fn i_180_table_matches_spec() {
        assert_eq!(
            kicks(PieceKind::I, Rotation::Zero, Rotation::Two),
            &[(0, 0), (0, 1)]
        );
        assert_eq!(
            kicks(PieceKind::I, Rotation::Two, Rotation::Zero),
            &[(0, 0), (0, -1)]
        );
        assert_eq!(
            kicks(PieceKind::I, Rotation::R, Rotation::L),
            &[(0, 0), (1, 0)]
        );
        assert_eq!(
            kicks(PieceKind::I, Rotation::L, Rotation::R),
            &[(0, 0), (-1, 0)]
        );
    }

    #[test]
    fn o_quarter_turns_do_not_kick_but_180_uses_the_general_table() {
        for from in Rotation::ALL {
            for to in Rotation::ALL {
                if from == to {
                    continue;
                }
                if kick_index(from, to).is_some() {
                    assert_eq!(kicks(PieceKind::O, from, to), &[(0, 0)]);
                } else {
                    assert_eq!(
                        kicks(PieceKind::O, from, to),
                        kicks(PieceKind::T, from, to),
                        "{from:?}->{to:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn fin_or_tst_kick_detection() {
        assert!(is_fin_or_tst_kick(Rotation::Zero, Rotation::R, (-1, -2)));
        assert!(is_fin_or_tst_kick(Rotation::Two, Rotation::R, (-1, -2)));
        assert!(is_fin_or_tst_kick(Rotation::Zero, Rotation::L, (1, -2)));
        assert!(is_fin_or_tst_kick(Rotation::Two, Rotation::L, (1, -2)));
        assert!(!is_fin_or_tst_kick(Rotation::Zero, Rotation::R, (1, -2)));
        assert!(!is_fin_or_tst_kick(Rotation::R, Rotation::Zero, (-1, -2)));
        assert!(!is_fin_or_tst_kick(Rotation::Zero, Rotation::Two, (-1, -2)));
    }

    #[test]
    fn kick_index_covers_all_quarter_turns() {
        for from in Rotation::ALL {
            for to in Rotation::ALL {
                let quarter_turn = from.cw() == to || from.ccw() == to;
                assert_eq!(
                    kick_index(from, to).is_some(),
                    quarter_turn,
                    "{from:?}->{to:?}"
                );
            }
        }
    }

    #[test]
    fn kick_180_index_covers_all_half_turns() {
        for from in Rotation::ALL {
            for to in Rotation::ALL {
                let half_turn = from != to && from.flip() == to;
                assert_eq!(
                    kick_180_index(from, to).is_some(),
                    half_turn,
                    "{from:?}->{to:?}"
                );
            }
        }
    }
}
