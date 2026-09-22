use intetrigence::{
    game::Game,
    search::{Budget, Searcher},
};
#[test]
fn both_searches_reuse_tree_and_rebuild_after_external_board_change() {
    for improved in [false, true] {
        let mut game = Game::new(76);
        // This test isolates the MCTS cache; a prior clear bypasses the
        // match-start opener book without changing the engine state.
        game.lines = 1;
        let mut searcher = Searcher::new(improved, None);
        let budget = Budget {
            milliseconds: 0,
            iterations: 5,
        };
        let (mv, stats) = searcher.choose(&game, budget);
        assert!(!stats.reused_tree);
        game.play(mv.unwrap()).unwrap();
        let (mv, stats) = searcher.choose(&game, budget);
        assert!(stats.reused_tree, "backend {improved}");
        game.play(mv.unwrap()).unwrap();
        game.board.cols[9] |= 1 << 10;
        let (_, stats) = searcher.choose(&game, budget);
        assert!(!stats.reused_tree);
    }
}

#[test]
fn one_piece_queue_still_produces_a_move() {
    let mut game = Game::new(77);
    game.queue.truncate(1);
    let (mv, stats) = Searcher::new(true, None).choose(
        &game,
        Budget {
            milliseconds: 0,
            iterations: 5,
        },
    );
    assert!(stats.iterations > 0);
    assert!(mv.is_some());
}

#[test]
fn unknown_randomizer_still_searches_known_queue_without_speculation() {
    let game = Game::new(78);
    let mut searcher = Searcher::new(true, None);
    let mut game = game;
    game.speculate = false;
    game.queue.truncate(2);
    let (mv, stats) = searcher.choose(
        &game,
        Budget {
            milliseconds: 0,
            iterations: 5,
        },
    );
    assert!(stats.iterations > 0);
    assert!(mv.is_some());
}
