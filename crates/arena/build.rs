//! Embed a content hash of the sources that decide a game.
//!
//! `git HEAD` says what the checkout contains, not what the running binary was
//! built from. A stale `arena.exe` records the current HEAD while playing with
//! old code, so a result can be filed under a commit it never ran. The hash
//! below is taken from the actual sources at build time (FNV-1a over the
//! sorted relative paths and bytes), which ties a replay to the code that
//! produced it.
//!
//! Covered: `crates/bot/src`, `crates/core/src` and `crates/arena/src`. The
//! bot decides our moves, core decides the rules and arena decides the referee,
//! so all three can change a game.

use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let roots = ["../bot/src", "../core/src", "src"];

    let mut files = Vec::new();
    for root in roots {
        collect(&manifest.join(root), &mut files);
    }
    files.sort();

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for path in &files {
        let relative = path.strip_prefix(&manifest).unwrap_or(path);
        for byte in relative.to_string_lossy().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        for byte in std::fs::read(path).unwrap_or_default() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    println!("cargo:rustc-env=ARENA_BUILD_ID={hash:016x}");
    for root in roots {
        println!("cargo:rerun-if-changed={root}");
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}
