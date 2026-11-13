//! S2 scoring unit/integration tests (independent of the JSON fixtures).

use intetrigence_core::{
    ClearInput, GarbageMultiplier, S2Scoring, ScoreState, ScoringEngine, SpinType, SurgeState,
    base_attack, compute_attack, split_surge, surge_charge_from_streak,
};

fn input(lines: u8, spin: SpinType) -> ClearInput {
    ClearInput::new(lines, spin)
}

#[test]
fn base_attack_table_matches_spec() {
    let expected = [
        (SpinType::None, [0, 0, 1, 2, 4]),
        (SpinType::Mini, [0, 0, 1, 2, 10]),
        (SpinType::Full, [0, 2, 4, 6, 10]),
    ];
    for (spin, row) in expected {
        for lines in 0..=4 {
            assert_eq!(
                base_attack(spin, lines),
                row[lines as usize],
                "{spin:?} {lines}"
            );
        }
    }
}

#[test]
fn combo_multiplier_and_ln_term() {
    // Double with combo 1: 1 * 1.25 = 1.25 -> floor 1.
    let mut clear = input(2, SpinType::None);
    clear.combo = 1;
    let attack = compute_attack(&clear);
    assert_eq!(attack.base, 1.25);
    assert_eq!(attack.events, vec![1]);

    // TSD with combo 2: 4 * 1.5 = 6.
    let mut clear = input(2, SpinType::Full);
    clear.combo = 2;
    let attack = compute_attack(&clear);
    assert_eq!(attack.base, 6.0);
    assert_eq!(attack.events, vec![6]);

    // Single with combo 5: base 0 -> ln(1 + 6.25) = 1.981...
    let mut clear = input(1, SpinType::None);
    clear.combo = 5;
    let attack = compute_attack(&clear);
    assert!((attack.base - 7.25_f64.ln()).abs() < 1e-15);
    assert_eq!(attack.events, vec![1]);
}

#[test]
fn b2b_bonus_uses_the_streak_after_the_clear() {
    // First difficult clear: N_before 0 -> N_after 1, no +1.
    let attack = compute_attack(&input(4, SpinType::None));
    assert_eq!(attack.base, 4.0);
    assert_eq!(attack.events, vec![4]);
    assert_eq!(attack.b2b_streak_after, 1);

    // Second difficult clear: N_before 1 -> N_after 2, +1.
    let mut clear = input(4, SpinType::None);
    clear.b2b_streak = 1;
    let attack = compute_attack(&clear);
    assert_eq!(attack.base, 5.0);
    assert_eq!(attack.events, vec![5]);
    assert_eq!(attack.b2b_streak_after, 2);

    // Breaking a B2B with a Single: N_after 0, no +1 and no surge at N=2.
    let mut clear = input(1, SpinType::None);
    clear.b2b_streak = 2;
    let attack = compute_attack(&clear);
    assert_eq!(attack.base, 0.0);
    assert!(attack.events.is_empty());
    assert_eq!(attack.b2b_streak_after, 0);
}

#[test]
fn surge_release_and_three_way_split() {
    // N_before 5 -> display B2B x4 -> S = 4 -> [1,1,2].
    let mut clear = input(1, SpinType::None);
    clear.b2b_streak = 5;
    let attack = compute_attack(&clear);
    assert_eq!(attack.surge_amount, 4);
    assert_eq!(attack.events, vec![1, 1, 2]);

    // N_before 9 -> S = 8 -> [3,3,2].
    let mut clear = input(1, SpinType::None);
    clear.b2b_streak = 9;
    let attack = compute_attack(&clear);
    assert_eq!(attack.surge_amount, 8);
    assert_eq!(attack.events, vec![3, 3, 2]);

    // N_before 6 with a Double: surge [2,2,1] before the main attack 1.
    let mut clear = input(2, SpinType::None);
    clear.b2b_streak = 6;
    let attack = compute_attack(&clear);
    assert_eq!(attack.surge_amount, 5);
    assert_eq!(attack.events, vec![2, 2, 1, 1]);

    // N_before 4 (display B2B x3) does not release.
    let mut clear = input(1, SpinType::None);
    clear.b2b_streak = 4;
    let attack = compute_attack(&clear);
    assert_eq!(attack.surge_amount, 0);
    assert!(attack.events.is_empty());
}

#[test]
fn surge_charge_is_derived_from_the_streak() {
    for streak in 0..=4 {
        assert_eq!(surge_charge_from_streak(streak), 0, "N={streak}");
    }
    assert_eq!(surge_charge_from_streak(5), 4);
    assert_eq!(surge_charge_from_streak(6), 5);
    assert_eq!(surge_charge_from_streak(9), 8);

    let state = ScoreState {
        combo: 0,
        b2b_streak: 5,
    };
    assert_eq!(state.b2b_display(), 4);
    assert_eq!(state.surge_charge(), 4);

    let surge = SurgeState::from_b2b_streak(5);
    assert_eq!(surge.charge, 4);
    assert_eq!(surge.chunks(), [1, 1, 2]);
}

#[test]
fn perfect_clear_adds_five_and_one_b2b() {
    // PC Double with no B2B: main 1 + PC 5; N_after 1.
    let mut clear = input(2, SpinType::None);
    clear.perfect_clear = true;
    let attack = compute_attack(&clear);
    assert_eq!(attack.events, vec![1, 5]);
    assert_eq!(attack.b2b_streak_after, 1);
    assert_eq!(attack.base, 1.0);

    // PC TSD with N_before 3: difficult +1 (not +2); main 5 + PC 5.
    let mut clear = input(2, SpinType::Full);
    clear.b2b_streak = 3;
    clear.perfect_clear = true;
    let attack = compute_attack(&clear);
    assert_eq!(attack.events, vec![5, 5]);
    assert_eq!(attack.b2b_streak_after, 4);
    assert_eq!(attack.base, 5.0);

    // PC Single: base 0, only the PC bonus.
    let mut clear = input(1, SpinType::None);
    clear.perfect_clear = true;
    let attack = compute_attack(&clear);
    assert_eq!(attack.events, vec![5]);
    assert_eq!(attack.base, 0.0);
}

#[test]
fn garbage_special_bonus_conditions() {
    // TSD with garbage: +1.
    let mut clear = input(2, SpinType::Full);
    clear.garbage_cleared = 1;
    let attack = compute_attack(&clear);
    assert_eq!(attack.gsb, 1);
    assert_eq!(attack.events, vec![5]);

    // Quad with garbage: +1.
    let mut clear = input(4, SpinType::None);
    clear.garbage_cleared = 1;
    let attack = compute_attack(&clear);
    assert_eq!(attack.gsb, 1);
    assert_eq!(attack.events, vec![5]);

    // Mini double with garbage: +1 (spin counts).
    let mut clear = input(2, SpinType::Mini);
    clear.garbage_cleared = 1;
    let attack = compute_attack(&clear);
    assert_eq!(attack.gsb, 1);
    assert_eq!(attack.events, vec![2]);

    // Single with garbage: no GSB.
    let mut clear = input(1, SpinType::None);
    clear.garbage_cleared = 1;
    let attack = compute_attack(&clear);
    assert_eq!(attack.gsb, 0);
    assert!(attack.events.is_empty());

    // Quad without garbage: no GSB.
    let attack = compute_attack(&input(4, SpinType::None));
    assert_eq!(attack.gsb, 0);
    assert_eq!(attack.events, vec![4]);
}

#[test]
fn no_clear_resets_combo_but_keeps_b2b() {
    // A no-clear placement normally arrives with combo 0 (the previous
    // placement's no-clear reset it). The §6.8 formula uses the pre-value, so
    // a synthetic combo > 0 would still apply the ln term to a 0-line clear.
    let mut clear = input(0, SpinType::Full);
    clear.combo = 0;
    clear.b2b_streak = 3;
    let attack = compute_attack(&clear);
    assert!(attack.events.is_empty());
    assert_eq!(attack.combo_after, 0);
    assert_eq!(attack.b2b_streak_after, 3);
    assert_eq!(attack.base, 0.0);
}

#[test]
fn stateful_engine_advances_combo_and_b2b() {
    let mut engine = S2Scoring::new();

    let first = engine.score(ClearInput::new(4, SpinType::None));
    assert_eq!(first.events, vec![4]);
    assert_eq!(first.b2b_streak_after, 1);
    assert_eq!(first.combo_after, 1);

    // combo 1: base (4 + 1 B2B) * 1.25 = 6.25 -> 6.
    let second = engine.score(ClearInput::new(4, SpinType::None));
    assert_eq!(second.events, vec![6]);
    assert_eq!(second.base, 6.25);
    assert_eq!(second.b2b_streak_after, 2);
    assert_eq!(second.combo_after, 2);

    let third = engine.score(ClearInput::new(1, SpinType::None));
    assert_eq!(third.b2b_streak_after, 0);
    assert_eq!(third.combo_after, 3);
    assert_eq!(third.events, vec![1]); // combo 2 ln term

    // The trait and the inherent method agree.
    let mut trait_engine = S2Scoring::new();
    let trait_result = ScoringEngine::score(&mut trait_engine, ClearInput::new(4, SpinType::None));
    assert_eq!(trait_result.events, vec![4]);
    assert_eq!(trait_engine.state.b2b_streak, 1);
}

#[test]
fn margin_multiplier_accumulates_frame_by_frame() {
    assert_eq!(GarbageMultiplier::at_frame(10_800).value(), 1.0);

    let accumulated = GarbageMultiplier::at_frame(12_300);
    assert!(
        (accumulated.value() - 1.199_999_999_999_978).abs() < 1e-15,
        "{}",
        accumulated.value()
    );

    let mut clear = input(4, SpinType::None);
    clear.b2b_streak = 2;
    clear.multiplier = accumulated.value();
    assert_eq!(compute_attack(&clear).events, vec![5]);

    clear.multiplier = 1.2;
    assert_eq!(compute_attack(&clear).events, vec![6]);
}

#[test]
fn split_surge_matches_the_spec_examples() {
    assert_eq!(split_surge(0), [0, 0, 0]);
    assert_eq!(split_surge(1), [0, 0, 1]);
    assert_eq!(split_surge(2), [1, 1, 0]);
    assert_eq!(split_surge(3), [1, 1, 1]);
    assert_eq!(split_surge(4), [1, 1, 2]);
    assert_eq!(split_surge(5), [2, 2, 1]);
    assert_eq!(split_surge(8), [3, 3, 2]);
}
