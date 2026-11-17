use std::io;
use std::path::{Path, PathBuf};

pub const SOURCE_ROOTS: [&str; 4] = ["../bot/src", "../core/src", "../tbp/src", "src"];
pub const BUILD_INPUTS: [&str; 8] = [
    "../../Cargo.toml",
    "../../Cargo.lock",
    "../../docs/evaluation-seed-exclusions.txt",
    "../bot/Cargo.toml",
    "../core/Cargo.toml",
    "../tbp/Cargo.toml",
    "Cargo.toml",
    "build.rs",
];

pub fn fingerprint(manifest: &Path) -> io::Result<String> {
    let mut files: Vec<_> = BUILD_INPUTS
        .iter()
        .map(|path| manifest.join(path))
        .collect();
    for root in SOURCE_ROOTS {
        collect(&manifest.join(root), &mut files)?;
    }
    files.sort();

    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for path in &files {
        let relative = path.strip_prefix(manifest).unwrap_or(path);
        let name = relative.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(path).map_err(|error| {
            io::Error::new(error.kind(), format!("{}: {error}", path.display()))
        })?;
        for byte in name.bytes().chain([0]).chain(bytes).chain([0]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Ok(format!("{hash:016x}"))
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
    Ok(())
}
