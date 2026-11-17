//! Embed the source fingerprint used by the evaluation preflight.

#[path = "src/build_fingerprint.rs"]
mod build_fingerprint;

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let hash = build_fingerprint::fingerprint(&manifest).expect("fingerprint arena sources");
    println!("cargo:rustc-env=ARENA_BUILD_ID={hash}");
    for path in build_fingerprint::SOURCE_ROOTS
        .into_iter()
        .chain(build_fingerprint::BUILD_INPUTS)
    {
        println!("cargo:rerun-if-changed={path}");
    }
}
