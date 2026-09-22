use intetrigence::arena::{
    paired_victory_bound, paired_victory_bound_for, MatchAgent, MatchResult,
};
fn m(seed: u64, swapped: bool, winner: Option<usize>) -> MatchResult {
    MatchResult {
        seed,
        swapped,
        winner,
        turns: 1,
        pieces: [1, 1],
        attack: [0, 0],
        lines: [0, 0],
    }
}
#[test]
fn paired_evidence_counts_draws_and_rejects_duplicate_seeds() {
    let matches = vec![m(1, false, Some(0)), m(1, true, None)];
    let r = paired_victory_bound(&matches).unwrap();
    assert_eq!(r["victory_rate"], 0.5);
    assert_eq!(r["above_half"], false);
    assert!(paired_victory_bound(&matches[..1]).is_err());
    assert!(paired_victory_bound(&[
        m(1, false, None),
        m(1, true, None),
        m(1, false, None),
        m(1, true, None)
    ])
    .is_err());
    let all_wins: Vec<_> = (0..100)
        .flat_map(|s| [m(s, false, Some(0)), m(s, true, Some(1))])
        .collect();
    assert_eq!(paired_victory_bound(&all_wins).unwrap()["above_half"], true);
}

#[test]
fn matchup_agents_and_named_pair_bound_follow_the_first_agent() {
    assert_eq!(
        MatchAgent::parse("cold-clear-2"),
        Some(MatchAgent::ColdClear2)
    );
    assert_eq!(MatchAgent::parse("Hoiko"), Some(MatchAgent::Hoiko));
    assert_eq!(
        MatchAgent::parse("intelligence"),
        Some(MatchAgent::Intetrigence)
    );
    let bound = paired_victory_bound_for(&[(4, Some(true)), (4, Some(false))]).unwrap();
    assert_eq!(bound["victory_rate"], 0.5);
    assert!(!bound["above_half"].as_bool().unwrap());
}
