//! Mechanical verification against `docs/s2-rng-derivation.json` (U7).

use std::fs;
use std::path::Path;

use intetrigence_core::derive_seeds;
use serde_json::Value;

#[test]
fn rng_derivation_fixtures_match() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/s2-rng-derivation.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let root: Value = serde_json::from_str(&text).expect("valid fixture JSON");

    assert_eq!(
        root["schema"].as_str(),
        Some("intetrigence/s2-rng-derivation/1")
    );
    let cases = root["cases"].as_array().expect("cases array");
    assert_eq!(cases.len(), 4);

    for case in cases {
        let seed = case["seed"].as_u64().unwrap();
        let derived = derive_seeds(seed);
        assert_eq!(
            derived.piece_seed,
            case["pieceSeed"].as_u64().unwrap(),
            "seed {seed}: piece seed"
        );
        assert_eq!(
            derived.garbage_seeds[0],
            case["garbageSeed0"].as_u64().unwrap(),
            "seed {seed}: garbage seed 0"
        );
        assert_eq!(
            derived.garbage_seeds[1],
            case["garbageSeed1"].as_u64().unwrap(),
            "seed {seed}: garbage seed 1"
        );
    }
}

#[test]
fn derivation_seeds_are_distinct_and_nonzero() {
    for seed in [1, 12_345, 123_456_789, 2_147_483_646] {
        let derived = derive_seeds(seed);
        assert_ne!(derived.piece_seed, 0);
        assert_ne!(derived.garbage_seeds[0], 0);
        assert_ne!(derived.garbage_seeds[1], 0);
        assert_ne!(derived.piece_seed, derived.garbage_seeds[0]);
        assert_ne!(derived.piece_seed, derived.garbage_seeds[1]);
        assert_ne!(derived.garbage_seeds[0], derived.garbage_seeds[1]);
    }
}
