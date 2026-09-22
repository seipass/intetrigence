use bumpalo_herd::Herd;
use enum_map::EnumMap;
use once_cell::sync::OnceCell;
use ouroboros::self_referencing;

use crate::data::Placement;
use crate::data::{GameState, Piece};

mod known;
mod speculated;

pub trait Evaluation:
    Ord + Copy + Default + std::ops::Add<Self::Reward, Output = Self> + 'static
{
    type Reward: Copy;

    fn average(of: impl Iterator<Item = Option<Self>>) -> Self;
}

pub struct Dag<E: Evaluation> {
    root: GameState,
    top_layer: Box<LayerCommon<E>>,
}

pub struct Selection<'a, E: Evaluation> {
    layers: Vec<&'a LayerCommon<E>>,
    game_state: GameState,
}

pub struct ChildData<E: Evaluation> {
    pub resulting_state: GameState,
    pub mv: Placement,
    pub eval: E,
    pub reward: E::Reward,
}

#[derive(Default)]
struct LayerCommon<E: Evaluation> {
    next_layer: OnceCell<Box<LayerCommon<E>>>,
    kind: WithBump<E>,
}

#[self_referencing]
struct WithBump<E: Evaluation> {
    bump: Herd,
    #[borrows(bump)]
    #[not_covariant]
    data: LayerKind<'this, E>,
}

enum LayerKind<'bump, E: Evaluation> {
    Known(known::Layer<'bump, E>),
    Speculated(speculated::Layer<'bump, E>),
}

#[derive(Clone, Copy, Debug)]
struct Child<E: Evaluation> {
    mv: Placement,
    reward: E::Reward,
    cached_eval: E,
}

enum SelectResult {
    Failed,
    Done,
    Advance(Piece, Placement),
}

struct BackpropUpdate {
    parent: u64,
    speculation_piece: Piece,
    mv: Placement,
    child: u64,
}

impl<E: Evaluation> Dag<E> {
    pub fn new(root: GameState, queue: &[Piece]) -> Self {
        let mut top_layer = LayerCommon::default();
        top_layer.kind.initialize_root(&root);

        let mut layer = &mut top_layer;
        for &piece in queue {
            layer.kind.despeculate(piece);
            layer.next_layer.get_or_init(Default::default);
            layer = layer.next_layer.get_mut().unwrap();
        }

        Dag {
            root,
            top_layer: Box::new(top_layer),
        }
    }

    pub fn advance(&mut self, mv: Placement) {
        puffin::profile_function!();
        let top_layer = std::mem::take(&mut *self.top_layer);
        self.root.advance(
            top_layer
                .kind
                .piece()
                .expect("cannot advance without next piece"),
            mv,
        );
        self.top_layer = top_layer.next_layer.into_inner().unwrap_or_default();
        self.top_layer.kind.initialize_root(&self.root);
    }

    pub fn add_piece(&mut self, piece: Piece) {
        puffin::profile_function!();
        self.top_layer
            .despeculate_and_backprop(piece)
            .expect("DAG must end in a speculative layer");
    }

    pub fn suggest(&self) -> Vec<Placement> {
        puffin::profile_function!();
        self.top_layer.kind.suggest(&self.root)
    }

    pub fn suggest_all(&self) -> Vec<Placement> {
        puffin::profile_function!();
        self.top_layer.kind.suggest_all(&self.root)
    }

    pub fn select(&self, speculate: bool, exploration: f64) -> Option<Selection<E>> {
        puffin::profile_function!();
        let mut layers = vec![&*self.top_layer];
        let mut game_state = self.root;
        loop {
            let &layer = layers.last().unwrap();

            match layer.kind.select(&game_state, speculate, exploration) {
                SelectResult::Failed => return None,
                SelectResult::Done => return Some(Selection { layers, game_state }),
                SelectResult::Advance(next, placement) => {
                    game_state.advance(next, placement);
                    layers.push(layer.next_layer.get_or_init(Default::default));
                }
            }
        }
    }
}

impl<E: Evaluation> Selection<'_, E> {
    pub fn state(&self) -> (GameState, Option<Piece>) {
        (self.game_state, self.layers.last().unwrap().kind.piece())
    }

    pub fn expand(self, children: EnumMap<Piece, Vec<ChildData<E>>>) {
        puffin::profile_function!();
        let mut layers = self.layers;
        let start_layer = layers.pop().unwrap();
        let mut next = start_layer.kind.expand(
            start_layer.next_layer.get_or_init(Default::default),
            self.game_state,
            children,
        );

        puffin::profile_scope!("backprop");
        let mut next_layer = start_layer;
        while let Some(layer) = layers.pop() {
            next = layer.kind.backprop(next, next_layer);
            next_layer = layer;

            if next.is_empty() {
                break;
            }
        }
    }
}

fn update_child<E: Evaluation>(list: &mut [Child<E>], placement: Placement, child_eval: E) -> bool {
    let mut index = list
        .iter()
        .enumerate()
        .find_map(|(i, c)| (c.mv == placement).then(|| i))
        .unwrap();

    let was_best = index == 0;
    list[index].cached_eval = child_eval + list[index].reward;

    if index > 0 && list[index - 1].cached_eval < list[index].cached_eval {
        // Shift up until the list is in order
        let hole = list[index];
        while index > 0 && list[index - 1].cached_eval < hole.cached_eval {
            list[index] = list[index - 1];
            index -= 1;
        }
        list[index] = hole;
    } else if index < list.len() - 1 && list[index + 1].cached_eval > list[index].cached_eval {
        // Shift down until the list is in order
        let hole = list[index];
        while index < list.len() - 1 && list[index + 1].cached_eval > hole.cached_eval {
            list[index] = list[index + 1];
            index += 1;
        }
        list[index] = hole;
    }

    was_best || index == 0
}

impl<E: Evaluation> LayerCommon<E> {
    fn despeculate_and_backprop(&mut self, piece: Piece) -> Option<Vec<BackpropUpdate>> {
        if let Some(updates) = self.kind.despeculate(piece) {
            return Some(updates);
        }
        self.next_layer.get_or_init(Default::default);
        let next_layer = self.next_layer.get_mut().unwrap();
        let updates = next_layer.despeculate_and_backprop(piece)?;
        Some(self.kind.backprop(updates, next_layer))
    }
}

impl<E: Evaluation> WithBump<E> {
    fn initialize_root(&self, root: &GameState) {
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.initialize_root(root),
            LayerKind::Speculated(l) => l.initialize_root(root),
        });
    }

    fn backprop(
        &self,
        to_update: Vec<BackpropUpdate>,
        next_layer: &LayerCommon<E>,
    ) -> Vec<BackpropUpdate> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.backprop(to_update, next_layer),
            LayerKind::Speculated(l) => l.backprop(to_update, next_layer),
        })
    }

    fn piece(&self) -> Option<Piece> {
        self.with(|this| match this.data {
            LayerKind::Known(l) => Some(l.piece),
            LayerKind::Speculated(_) => None,
        })
    }

    fn expand(
        &self,
        next_layer: &LayerCommon<E>,
        parent_state: GameState,
        children: EnumMap<Piece, Vec<ChildData<E>>>,
    ) -> Vec<BackpropUpdate> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.expand(this.bump, next_layer, parent_state, children),
            LayerKind::Speculated(l) => l.expand(this.bump, next_layer, parent_state, children),
        })
    }

    fn select(&self, game_state: &GameState, speculate: bool, exploration: f64) -> SelectResult {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.select(game_state, exploration),
            LayerKind::Speculated(l) if speculate => l.select(game_state, exploration),
            LayerKind::Speculated(_) => SelectResult::Failed,
        })
    }

    fn suggest(&self, state: &GameState) -> Vec<Placement> {
        puffin::profile_function!();
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.suggest(state),
            LayerKind::Speculated(l) => l.suggest(state),
        })
    }

    fn suggest_all(&self, state: &GameState) -> Vec<Placement> {
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.suggest_all(state),
            LayerKind::Speculated(l) => l.suggest_all(state),
        })
    }

    fn despeculate(&mut self, piece: Piece) -> Option<Vec<BackpropUpdate>> {
        puffin::profile_function!();
        self.with_mut(|this| {
            let old = match this.data {
                LayerKind::Known(_) => return None,
                LayerKind::Speculated(layer) => std::mem::take(layer),
            };
            let mut updates = Vec::new();
            let states = old.states.map_values_with_key(|child, node| {
                let children = node.children.map(|children| children.into_children(piece));
                let eval = match children.as_ref() {
                    Some(children) => E::average(std::iter::once(
                        children.first().map(|entry| entry.cached_eval),
                    )),
                    None => node.eval,
                };
                if eval != node.eval {
                    updates.extend(node.parents.iter().map(|&(parent, mv, speculation_piece)| {
                        BackpropUpdate {
                            parent,
                            mv,
                            speculation_piece,
                            child,
                        }
                    }));
                }
                known::Node {
                    parents: node.parents,
                    eval,
                    children,
                    expanding: node.expanding,
                }
            });
            *this.data = LayerKind::Known(known::Layer { states, piece });
            Some(updates)
        })
    }

    fn get_eval(&self, raw: u64) -> E {
        self.with(|this| match this.data {
            LayerKind::Known(l) => l.get_eval(raw),
            LayerKind::Speculated(l) => l.get_eval(raw),
        })
    }

    fn create_nodes(
        &self,
        children: &[ChildData<E>],
        parent: u64,
        speculation_piece: Piece,
    ) -> Vec<E> {
        self.with(|this| match this.data {
            LayerKind::Known(l) => {
                let bump = this.bump.get();
                children
                    .iter()
                    .map(|child| l.create_node(&bump, child, parent, speculation_piece))
                    .collect()
            }
            LayerKind::Speculated(l) => {
                let bump = this.bump.get();
                children
                    .iter()
                    .map(|child| l.create_node(&bump, child, parent, speculation_piece))
                    .collect()
            }
        })
    }
}

impl<E: Evaluation> Default for WithBump<E> {
    fn default() -> Self {
        WithBump::new(Herd::new(), |_| LayerKind::Speculated(Default::default()))
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use crate::data::{PieceLocation, Rotation, Spin};
    #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
    struct TestEval(i32);
    impl std::ops::Add<i32> for TestEval {
        type Output = Self;
        fn add(self, r: i32) -> Self {
            Self(self.0 + r)
        }
    }
    impl Evaluation for TestEval {
        type Reward = i32;
        fn average(v: impl Iterator<Item = Option<Self>>) -> Self {
            v.flatten().next().unwrap_or_default()
        }
    }
    fn child(x: i8, eval: i32) -> Child<TestEval> {
        Child {
            mv: Placement {
                location: PieceLocation {
                    piece: Piece::O,
                    rotation: Rotation::North,
                    x,
                    y: 0,
                },
                spin: Spin::None,
            },
            reward: 0,
            cached_eval: TestEval(eval),
        }
    }
    #[test]
    fn demoted_best_child_triggers_parent_recalculation() {
        let mut children = [child(0, 10), child(2, 5), child(4, 0)];
        let first = children[0].mv;
        assert!(update_child(&mut children, first, TestEval(-10)));
        assert_eq!(children[0].cached_eval.0, 5);
        assert_eq!(children[2].mv, first);
        let last = children[2].mv;
        assert!(!update_child(&mut children, last, TestEval(-20)));
        assert!(update_child(&mut children, last, TestEval(20)));
    }

    fn test_state(marker: u32) -> GameState {
        let mut board = crate::data::Board::default();
        if marker != 0 {
            board.cols[0] = 1 << marker;
        }
        GameState {
            board,
            bag: enumset::EnumSet::all(),
            reserve: Piece::O,
            back_to_back: false,
            combo: 0,
        }
    }

    fn test_placement(piece: Piece, x: i8) -> Placement {
        Placement {
            location: PieceLocation {
                piece,
                rotation: Rotation::North,
                x,
                y: 0,
            },
            spin: Spin::None,
        }
    }

    fn speculative_children(branch: u32) -> EnumMap<Piece, Vec<ChildData<TestEval>>> {
        let mut children: EnumMap<Piece, Vec<ChildData<TestEval>>> = EnumMap::default();
        for piece in enumset::EnumSet::<Piece>::all() {
            let eval = if branch == 1 && piece == Piece::T {
                -100
            } else if branch == 1 {
                100
            } else {
                50
            };
            children[piece].push(ChildData {
                resulting_state: test_state(3 + branch * 8 + piece as u32),
                mv: test_placement(piece, 0),
                eval: TestEval(eval),
                reward: 0,
            });
        }
        children
    }

    #[test]
    fn revealing_speculated_piece_reorders_ancestor_without_more_search() {
        let root = test_state(0);
        let state_a = test_state(1);
        let state_b = test_state(2);
        let move_a = test_placement(Piece::I, 0);
        let move_b = test_placement(Piece::I, 1);
        let mut dag = Dag::<TestEval>::new(root, &[Piece::I]);

        let mut roots = EnumMap::default();
        roots[Piece::I] = vec![
            ChildData {
                resulting_state: state_a,
                mv: move_a,
                eval: TestEval::default(),
                reward: 0,
            },
            ChildData {
                resulting_state: state_b,
                mv: move_b,
                eval: TestEval::default(),
                reward: 0,
            },
        ];
        dag.select(false, 1.0)
            .expect("root is expandable")
            .expand(roots);

        {
            let first = &*dag.top_layer;
            let second = first.next_layer.get_or_init(Default::default);
            Selection {
                layers: vec![first, second],
                game_state: state_a,
            }
            .expand(speculative_children(1));
            Selection {
                layers: vec![first, second],
                game_state: state_b,
            }
            .expand(speculative_children(2));
        }
        assert_eq!(dag.suggest().first(), Some(&move_a));

        dag.add_piece(Piece::T);

        assert_eq!(dag.suggest().first(), Some(&move_b));
    }
}
