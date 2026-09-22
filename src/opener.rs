use crate::game::Game;
use intetrigence_engine::{
    data::{Board, Piece, Placement, Spin},
    movegen::find_moves,
};

const PIECE_COUNT: usize = 7;
const PLAN_CAPACITY: usize = 16;
const LOOKAHEAD: u8 = 7;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Target {
    occupied: Board,
    pieces: [[u64; 10]; PIECE_COUNT],
}

impl Target {
    const fn from_rows(rows: &[&str]) -> Self {
        let mut target = Self {
            occupied: Board { cols: [0; 10] },
            pieces: [[0; 10]; PIECE_COUNT],
        };
        let mut y = 0;
        while y < rows.len() {
            let row = rows[y].as_bytes();
            let mut x = 0;
            while x < row.len() {
                let cell = row[x];
                if cell != b'_' && cell != b'.' {
                    target.occupied.cols[x] |= 1 << y;
                    if let Some(piece) = piece_index(cell) {
                        target.pieces[piece][x] |= 1 << y;
                    }
                }
                x += 1;
            }
            y += 1;
        }
        target
    }

    const fn mirrored(self) -> Self {
        let mut mirrored = Self {
            occupied: Board { cols: [0; 10] },
            pieces: [[0; 10]; PIECE_COUNT],
        };
        let mut x = 0;
        while x < 10 {
            mirrored.occupied.cols[9 - x] = self.occupied.cols[x];
            let mut piece = 0;
            while piece < PIECE_COUNT {
                mirrored.pieces[mirror_piece_index(piece)][9 - x] = self.pieces[piece][x];
                piece += 1;
            }
            x += 1;
        }
        mirrored
    }
}

const fn piece_index(cell: u8) -> Option<usize> {
    match cell {
        b'I' => Some(Piece::I as usize),
        b'O' => Some(Piece::O as usize),
        b'T' => Some(Piece::T as usize),
        b'L' => Some(Piece::L as usize),
        b'J' => Some(Piece::J as usize),
        b'S' => Some(Piece::S as usize),
        b'Z' => Some(Piece::Z as usize),
        _ => None,
    }
}

const fn mirror_piece_index(piece: usize) -> usize {
    match piece {
        x if x == Piece::L as usize => Piece::J as usize,
        x if x == Piece::J as usize => Piece::L as usize,
        x if x == Piece::S as usize => Piece::Z as usize,
        x if x == Piece::Z as usize => Piece::S as usize,
        _ => piece,
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    target: Target,
    preference: u16,
}

const fn candidate(target: Target, preference: u16) -> Candidate {
    Candidate { target, preference }
}

const MS2_FIRST: Target =
    Target::from_rows(&["IJJJ_ZZTOO", "IJS_ZZTTOO", "ISS____T__", "IS________"]);
const HONEY_FIRST: Target = Target::from_rows(&[
    "IIII_ZZTOO",
    "LLL_ZZTTOO",
    "SSL____TJJ",
    "_SS______J",
    "_________J",
]);
const MEISO_FIRST: Target =
    Target::from_rows(&["OOLL_TZJJI", "OOL_TTZZJI", "__L__T_ZJI", "_________I"]);

// Pick the family whose foundation piece arrives first, then fall back to the
// other published TD openers if that graph cannot be completed.
const FIRST_TARGETS: &[Candidate] = &[
    candidate(HONEY_FIRST, 300),
    candidate(HONEY_FIRST.mirrored(), 299),
    candidate(MEISO_FIRST, 200),
    candidate(MEISO_FIRST.mirrored(), 199),
    candidate(MS2_FIRST, 100),
    candidate(MS2_FIRST.mirrored(), 99),
];

macro_rules! target {
    ($preference:expr; $($row:literal),+ $(,)?) => {
        candidate(Target::from_rows(&[$($row),+]), $preference)
    };
}

// The continuations are ordered by their published 8-line-PC quality. Each
// field is the setup immediately before the second-bag TST; X cells are the
// already-built first bag. The mirrored forms are generated below.
const SECOND_BASE: &[Candidate] = &[
    // Honey Cup: S1, S2, S3, A1, A2, A3, EX1, EX2.
    target!(390; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "IXX_ZZLLSX", "IOO___LSSX", "IOOJ__LS__", "IJJJ______"),
    target!(389; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "IXX_ZZJOOX", "ISS___JOOX", "ILSS__JJ__", "ILLL______"),
    target!(370; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "JXX_ZZLLLX", "JSS___OOLX", "JJSS__OO__", "IIII______"),
    target!(350; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "IXX_ZZLLLX", "ISS___OOLX", "I_SS__OOJJ", "I________J", "_________J"),
    target!(330; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "IXX_ZZLLJX", "ISS___L_JX", "I_SS__L_JJ", "I_______OO", "________OO"),
    target!(320; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "JXX_ZZIOOX", "JSS___IOOX", "JJSS__ILLL", "______I__L"),
    target!(310; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "LXX_ZZJJJX", "LLL___JSSX", "IIII__OOSS", "______OO__"),
    target!(300; "XXXX_XXXXX", "XXX_XXXXXX", "XXX__ZZXXX", "JXX_ZZIOOX", "JSS___IOOX", "JJSS__I___", "_LLL__I___", "___L______"),
    // Meiso: ideal, semi-ideal, compromise.
    target!(290; "XXXX_XXXXX", "XXXTXXXXXX", "SSXTTXJXXX", "ISSTJJJSZX", "ILL___SSZZ", "ILOO__S__Z", "ILOO______"),
    target!(280; "XXXX_XXXXX", "XXXTXXXXXX", "IZXTTXJXXX", "IZZTJJJSSX", "ILZ___OOSS", "ILLL__OOS_", "_______SS_", "_______S__"),
    target!(270; "XXXX_XXXXX", "XXXTXXXXXX", "SSXTTXJXXX", "ISSTJJJOOX", "ISS___ZOO_", "I_SS__ZZL_", "I______ZL_", "_______LL_"),
    // MS2 continuation fields, descending by published PC rate.
    target!(260; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXJTZZSSOO", "LLJ____SSI", "LLJJ_____I", "LL_______I", "LL_______I"),
    target!(259; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXJL", "XXSTZZJJJL", "ISS_____LL", "ISOO______", "ILOO______", "ILLL______"),
    target!(258; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXJTZZSSOO", "LLJ____SS_", "LLJJ______", "LLLL______", "IIII______"),
    target!(257; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXLI", "XXSTZZLLLI", "JSS___LLLI", "JSOO__L__I", "JJOO______"),
    target!(256; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXLI", "XXJTZZLLLI", "OOJ___LLLI", "OOJJ__LSSI", "________SS"),
    target!(255; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXJTZZLLOO", "JJJ___LLLL", "IIII__LSSL", "________SS"),
    target!(254; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXJTZZSSOO", "LLJ____SSI", "LLJJ_____I", "LLLL_____I", "_________I"),
    target!(253; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXJTZZSSOO", "LLJ___LSSI", "L_JJ__LLLI", "L________I", "_________I"),
    target!(252; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXJTZZLLOO", "I_J___LLLS", "I_JJ__LLSS", "I______LS_", "I_________"),
    target!(251; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXLL", "XXJTZZOOLL", "I_J___OOLL", "I_JJ__SSLL", "I______SS_", "I_________"),
    target!(250; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXOO", "XXSTZZLLOO", "ISS___LLLL", "ISJJ__L__L", "I__J______", "I__J______"),
    target!(249; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXLL", "XXSTZZOOLI", "_SS___OOLI", "_SJJ__LLLI", "___J____LI", "___J______"),
    target!(248; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXLI", "XXSTZZLLLI", "_SS___LLLI", "_SJJ__LOOI", "___J___OO_", "___J______"),
    target!(247; "XXXX_XXXXX", "XXXTXXXXXX", "XXXTTZZXJJ", "XXITZZLLSJ", "OOI___LSSJ", "OOIL__LS__", "__IL______", "__LL______"),
];

#[derive(Clone, Copy)]
struct PlanState {
    board: Board,
    hold: Option<Piece>,
    queue: [Piece; PLAN_CAPACITY],
    len: usize,
}

impl PlanState {
    fn from_game(game: &Game) -> Self {
        let mut state = Self {
            board: game.board,
            hold: game.hold,
            queue: [Piece::I; PLAN_CAPACITY],
            len: 0,
        };
        for &piece in game.queue.iter().take(PLAN_CAPACITY) {
            state.queue[state.len] = piece;
            state.len += 1;
        }
        if game.speculate {
            for &piece in game.bag.iter().rev() {
                if state.len == PLAN_CAPACITY {
                    break;
                }
                state.queue[state.len] = piece;
                state.len += 1;
            }
        }
        state
    }

    fn active(self) -> Option<Piece> {
        (self.len > 0).then_some(self.queue[0])
    }

    fn held_or_next(self) -> Option<Piece> {
        self.hold
            .or_else(|| (self.len > 1).then_some(self.queue[1]))
    }

    fn play(mut self, placement: Placement) -> Option<(Self, u32)> {
        let current = self.active()?;
        if placement.location.piece == current {
            self.remove_front(1);
        } else if Some(placement.location.piece) == self.held_or_next() {
            let had_hold = self.hold.is_some();
            self.hold = Some(current);
            self.remove_front(if had_hold { 1 } else { 2 });
        } else {
            return None;
        }

        self.board.place(placement.location);
        let cleared = self.board.line_clears();
        if cleared != 0 {
            self.board.remove_lines(cleared);
        }
        Some((self, cleared.count_ones()))
    }

    fn remove_front(&mut self, count: usize) {
        let count = count.min(self.len);
        let mut index = 0;
        while index + count < self.len {
            self.queue[index] = self.queue[index + count];
            index += 1;
        }
        self.len -= count;
    }
}

#[derive(Clone, Copy)]
enum Stage {
    First(Option<Target>),
    Second(Option<Target>),
    Done,
}

pub struct OpenerBook {
    stage: Stage,
}

impl Default for OpenerBook {
    fn default() -> Self {
        Self {
            stage: Stage::First(None),
        }
    }
}

impl OpenerBook {
    pub fn choose(&mut self, game: &Game) -> Option<Placement> {
        if game.dead
            || game.queue.is_empty()
            || game.lines != 0
            || !game.pending.is_empty()
            || game.garbage_board().cols.iter().any(|&column| column != 0)
        {
            self.stage = Stage::Done;
            return None;
        }

        loop {
            match self.stage {
                Stage::First(Some(target)) if first_target_complete(game, target) => {
                    self.stage = Stage::Second(None);
                }
                Stage::First(Some(target)) => {
                    let selected = first_completion_move(PlanState::from_game(game), target);
                    if selected.is_none() {
                        self.stage = Stage::Done;
                    }
                    return selected;
                }
                Stage::First(None) => {
                    let selected = select_first(game);
                    if let Some((placement, target)) = selected {
                        self.stage = Stage::First(Some(target));
                        return Some(placement);
                    }
                    self.stage = Stage::Done;
                    return None;
                }
                Stage::Second(Some(target)) => {
                    let selected = select_committed(game, target);
                    if selected.is_none() {
                        self.stage = Stage::Done;
                    }
                    return selected;
                }
                Stage::Second(None) => {
                    let selected = select_second(game);
                    if let Some((placement, target)) = selected {
                        self.stage = Stage::Second(Some(target));
                        return Some(placement);
                    }
                    self.stage = Stage::Done;
                    return None;
                }
                Stage::Done => return None,
            }
        }
    }
}

fn select_first(game: &Game) -> Option<(Placement, Target)> {
    let state = PlanState::from_game(game);
    let order = match first_family(game) {
        0 => [0, 2, 1],
        1 => [1, 0, 2],
        _ => [2, 0, 1],
    };
    for family in order {
        for candidate in &FIRST_TARGETS[family * 2..family * 2 + 2] {
            if let Some(placement) = first_completion_move(state, candidate.target) {
                return Some((placement, candidate.target));
            }
        }
    }
    None
}

fn first_family(game: &Game) -> usize {
    let position = |piece| {
        game.queue
            .iter()
            .position(|&queued| queued == piece)
            .unwrap_or(usize::MAX)
    };
    let i = position(Piece::I);
    let j = position(Piece::J);
    let l = position(Piece::L);
    if i < j && i < l {
        0
    } else if j < l {
        1
    } else {
        2
    }
}

fn first_completion_move(state: PlanState, target: Target) -> Option<Placement> {
    if !board_is_subset(state.board, target.occupied) {
        return None;
    }
    let mut selected = None;
    visit_matching_moves(state, target, |placement| {
        if state
            .play(placement)
            .is_some_and(|(next, lines)| lines == 0 && can_complete_first(next, target))
        {
            selected = Some(placement);
            true
        } else {
            false
        }
    });
    selected
}

fn can_complete_first(state: PlanState, target: Target) -> bool {
    if state.board == target.occupied {
        return true;
    }
    visit_matching_moves(state, target, |placement| {
        state
            .play(placement)
            .is_some_and(|(next, lines)| lines == 0 && can_complete_first(next, target))
    })
}

fn select_second(game: &Game) -> Option<(Placement, Target)> {
    let state = PlanState::from_game(game);
    let mut best = None;
    for candidate in SECOND_BASE {
        consider(candidate, state, &mut best);
        let mirrored = Candidate {
            target: candidate.target.mirrored(),
            preference: candidate.preference.saturating_sub(1),
        };
        consider(&mirrored, state, &mut best);
    }
    best.map(|(_, _, placement, target)| (placement, target))
}

fn select_committed(game: &Game, target: Target) -> Option<Placement> {
    let state = PlanState::from_game(game);
    let mut best = None;
    consider(
        &Candidate {
            target,
            preference: 0,
        },
        state,
        &mut best,
    );
    best.map(|(_, _, placement, _)| placement)
}

fn first_target_complete(game: &Game, target: Target) -> bool {
    if !board_is_subset(target.occupied, game.board) {
        return false;
    }
    if target.occupied != game.board {
        return true;
    }
    let mut missing = None;
    let mut piece = 0;
    while piece < PIECE_COUNT {
        if target.pieces[piece].iter().all(|&column| column == 0) {
            if missing.is_some() {
                return false;
            }
            missing = Some(piece);
        }
        piece += 1;
    }
    missing.is_none_or(|piece| {
        game.hold.map(|hold| hold as usize) == Some(piece)
            || game.queue.front().map(|active| *active as usize) == Some(piece)
    })
}

fn consider(
    candidate: &Candidate,
    state: PlanState,
    best: &mut Option<(u8, u16, Placement, Target)>,
) {
    if !board_is_subset(state.board, candidate.target.occupied) {
        return;
    }
    visit_matching_moves(state, candidate.target, |placement| {
        let Some((next, lines)) = state.play(placement) else {
            return false;
        };
        let depth = if lines == 3 && matches!(placement.spin, Spin::Full) {
            u8::MAX
        } else if lines == 0 {
            1 + continuation_depth(next, candidate.target, LOOKAHEAD - 1)
        } else {
            return false;
        };
        let ranked = (depth, candidate.preference, placement, candidate.target);
        if best
            .as_ref()
            .is_none_or(|&(best_depth, best_preference, _, _)| {
                (depth, candidate.preference) > (best_depth, best_preference)
            })
        {
            *best = Some(ranked);
        }
        false
    });
}

fn continuation_depth(state: PlanState, target: Target, remaining: u8) -> u8 {
    if remaining == 0 || state.len == 0 || !board_is_subset(state.board, target.occupied) {
        return 0;
    }
    let mut best = 0;
    visit_matching_moves(state, target, |placement| {
        let Some((next, lines)) = state.play(placement) else {
            return false;
        };
        if lines == 3 && matches!(placement.spin, Spin::Full) {
            best = remaining;
            return true;
        }
        if lines == 0 {
            best = best.max(1 + continuation_depth(next, target, remaining - 1));
        }
        best == remaining
    });
    best
}

fn visit_matching_moves(
    state: PlanState,
    target: Target,
    mut visit: impl FnMut(Placement) -> bool,
) -> bool {
    let active = state.active();
    let held = state.held_or_next();
    let pieces = [active, (held != active).then_some(held).flatten()];
    for piece in pieces.into_iter().flatten() {
        let unfilled_tst = piece == Piece::T
            && target.pieces[Piece::T as usize]
                .iter()
                .all(|&column| column == 0)
            && state.board == target.occupied;
        if let Some(placement) = find_moves(&state.board, piece)
            .into_iter()
            .map(|(placement, _)| placement)
            .find(|&placement| {
                if unfilled_tst {
                    executes_tst(state.board, placement)
                } else {
                    placement_matches(placement, target)
                }
            })
        {
            if visit(placement) {
                return true;
            }
        }
    }
    false
}
fn placement_matches(placement: Placement, target: Target) -> bool {
    let mut cells = [0u64; 10];
    for (x, y) in placement.location.cells() {
        if !(0..10).contains(&x) || !(0..40).contains(&y) {
            return false;
        }
        cells[x as usize] |= 1 << y;
    }
    cells
        .iter()
        .zip(target.pieces[placement.location.piece as usize])
        .all(|(&cells, target)| cells & !target == 0)
}

fn executes_tst(mut board: Board, placement: Placement) -> bool {
    if !matches!(placement.spin, Spin::Full) {
        return false;
    }
    board.place(placement.location);
    board.line_clears().count_ones() == 3
}

fn board_is_subset(board: Board, target: Board) -> bool {
    board
        .cols
        .iter()
        .zip(target.cols)
        .all(|(&board, target)| board & !target == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{Game, PIECES},
        search::{Budget, Searcher},
    };
    use std::collections::{HashSet, VecDeque};

    #[test]
    fn all_first_bags_are_covered_by_the_decision_graphs() {
        let mut bag = PIECES;
        let mut covered_families = HashSet::new();
        let mut permutations = 0;
        permute(&mut bag, 0, &mut |bag| {
            let mut game = Game::new(0);
            game.queue = bag.iter().copied().chain(PIECES).collect::<VecDeque<_>>();
            game.bag.clear();
            let (_, target) = select_first(&game)
                .unwrap_or_else(|| panic!("no opener graph covers first bag {bag:?}"));
            let family = FIRST_TARGETS
                .iter()
                .position(|candidate| candidate.target == target)
                .expect("selected an unknown first-bag target")
                / 2;
            covered_families.insert(family);
            permutations += 1;
        });
        assert_eq!(permutations, 5040);
        assert_eq!(covered_families, HashSet::from([0, 1, 2]));
    }

    #[test]
    fn completes_ms2_regression_bag() {
        let bag = [
            Piece::O,
            Piece::T,
            Piece::L,
            Piece::J,
            Piece::S,
            Piece::Z,
            Piece::I,
        ];
        let mut game = Game::new(0);
        game.queue = bag.into_iter().chain(PIECES).collect::<VecDeque<_>>();
        game.bag.clear();
        let mut book = OpenerBook::default();
        let first = book
            .choose(&game)
            .expect("MS2 regression has no first move");
        let Stage::First(Some(target)) = book.stage else {
            panic!("MS2 regression did not commit a first-bag target");
        };
        let family = FIRST_TARGETS
            .iter()
            .position(|candidate| candidate.target == target)
            .expect("unknown first-bag target")
            / 2;
        assert_eq!(family, 2);
        game.play(first).unwrap();
        for turn in 1..7 {
            let placement = book
                .choose(&game)
                .unwrap_or_else(|| panic!("no opener move at turn {turn}: {:?}", game.board));
            game.play(placement).unwrap();
            if FIRST_TARGETS
                .iter()
                .any(|candidate| first_target_complete(&game, candidate.target))
            {
                return;
            }
        }
        panic!("regression bag did not complete a first-bag target");
    }

    #[test]
    fn reaches_each_family_and_executes_its_tst() {
        for (seed, expected_family) in [(12, 0), (0, 1), (1, 2)] {
            let mut game = Game::new_guideline(seed);
            let mut book = OpenerBook::default();
            let mut family = None;
            let mut cleared = false;
            for turn in 0..14 {
                let placement = book.choose(&game).unwrap_or_else(|| {
                    panic!("seed {seed} abandoned family {expected_family} at turn {turn}")
                });
                if family.is_none() {
                    let Stage::First(Some(target)) = book.stage else {
                        panic!("seed {seed} did not commit a first-bag target");
                    };
                    family = FIRST_TARGETS
                        .iter()
                        .position(|candidate| candidate.target == target)
                        .map(|index| index / 2);
                    assert_eq!(family, Some(expected_family));
                }
                assert!(game.legal(placement));
                let outcome = game.play(placement).unwrap();
                if outcome.lines != 0 {
                    assert_eq!(outcome.lines, 3, "seed {seed} did not execute a TST");
                    assert_eq!(
                        placement.spin,
                        Spin::Full,
                        "seed {seed} cleared three lines without a full T-spin"
                    );
                    cleared = true;
                    break;
                }
            }
            assert!(cleared, "seed {seed} did not complete its TST");
        }
    }

    #[test]
    fn unsafe_openers_return_control_to_search() {
        let mut game = Game::new_guideline(1);
        game.lines = 1;
        assert!(OpenerBook::default().choose(&game).is_none());

        game.lines = 0;
        game.pending.push_back(4);
        assert!(OpenerBook::default().choose(&game).is_none());

        let mut searcher = Searcher::new(true, None);
        let (placement, _) = searcher.choose(
            &game,
            Budget {
                milliseconds: 0,
                iterations: 32,
            },
        );
        let placement = placement.expect("normal search did not take over");
        assert!(game.legal(placement));
    }

    fn permute<F: FnMut(&[Piece; 7])>(pieces: &mut [Piece; 7], index: usize, visit: &mut F) {
        if index == pieces.len() {
            visit(pieces);
            return;
        }
        for swap in index..pieces.len() {
            pieces.swap(index, swap);
            permute(pieces, index + 1, visit);
            pieces.swap(index, swap);
        }
    }
}
