//! U7 seed derivation vectors and optional `docs/s2-rng-derivation.json` check.
//!
//! `docs/` is owned by the spec lead, so the four official vectors are also
//! hardcoded here. The test reads the JSON fixture when it is present and
//! always verifies the hardcoded vectors, keeping this commit self-contained.

use std::fs;
use std::path::Path;

use intetrigence_core::derive_seeds;
use serde_json::Value;

/// `(seed, piece_seed, garbage_seed_0, garbage_seed_1)`.
const EXPECTED: [(u64, u64, u64, u64); 4] = [
    (1, 1, 16_807, 282_475_249),
    (12_345, 12_345, 207_482_415, 1_790_989_824),
    (123_456_789, 123_456_789, 469_049_721, 2_053_676_357),
    (2_147_483_646, 2_147_483_646, 2_147_466_840, 1_865_008_398),
];

#[test]
fn rng_derivation_known_vectors() {
    for (seed, piece_seed, garbage_seed_0, garbage_seed_1) in EXPECTED {
        let derived = derive_seeds(seed);
        assert_eq!(derived.piece_seed, piece_seed, "seed {seed}: piece seed");
        assert_eq!(
            derived.garbage_seeds[0], garbage_seed_0,
            "seed {seed}: garbage seed 0"
        );
        assert_eq!(
            derived.garbage_seeds[1], garbage_seed_1,
            "seed {seed}: garbage seed 1"
        );
    }
}

#[test]
fn rng_derivation_fixture_matches_when_present() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/s2-rng-derivation.json");
    if !path.exists() {
        return;
    }

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
    for (seed, ..) in EXPECTED {
        let derived = derive_seeds(seed);
        assert_ne!(derived.piece_seed, 0);
        assert_ne!(derived.garbage_seeds[0], 0);
        assert_ne!(derived.garbage_seeds[1], 0);
        assert_ne!(derived.piece_seed, derived.garbage_seeds[0]);
        assert_ne!(derived.piece_seed, derived.garbage_seeds[1]);
        assert_ne!(derived.garbage_seeds[0], derived.garbage_seeds[1]);
    }
}
