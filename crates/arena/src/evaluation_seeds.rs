//! Durable seed claims shared with the Lab runner under the measurement lock.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct SeedClaim {
    pub seeds: Vec<u64>,
    pub kind: String,
    pub identity: Value,
}

pub(crate) fn claim_round(
    claim: &SeedClaim,
    resume: bool,
    legacy_resume: bool,
) -> Result<(), String> {
    let lock = crate::measurement_lock::lock_path()?;
    let ledger = lock
        .parent()
        .ok_or("measurement lock has no parent")?
        .join("evaluation-seeds");
    register(&ledger, claim, resume, legacy_resume)
}

fn register(
    ledger: &Path,
    claim: &SeedClaim,
    resume: bool,
    legacy_resume: bool,
) -> Result<(), String> {
    fs::create_dir_all(ledger).map_err(|error| format!("create shared seed ledger: {error}"))?;
    let first = claim
        .seeds
        .iter()
        .min()
        .ok_or("cannot claim an empty seed allocation")?;
    let path = ledger.join(format!("{}-{first}.json", claim.kind));
    let mut matched = false;
    for entry in
        fs::read_dir(ledger).map_err(|error| format!("read shared seed ledger: {error}"))?
    {
        let other = entry.map_err(|error| error.to_string())?.path();
        if other
            .extension()
            .is_none_or(|extension| extension != "json")
        {
            continue;
        }
        let body =
            fs::read(&other).map_err(|error| format!("read {}: {error}", other.display()))?;
        let existing: SeedClaim = serde_json::from_slice(&body)
            .map_err(|error| format!("parse {}: {error}", other.display()))?;
        if resume && other == path && existing == *claim {
            matched = true;
            continue;
        }
        if let Some(seed) = claim
            .seeds
            .iter()
            .find(|seed| existing.seeds.contains(seed))
        {
            return Err(format!(
                "evaluation seed {seed} has a claim at {}; resume the original run or register fresh seeds",
                other.display()
            ));
        }
    }
    if matched {
        return Ok(());
    }
    if resume && !legacy_resume {
        return Err("the evaluation round has no existing seed claim to resume".into());
    }
    let body = serde_json::to_vec(claim).map_err(|error| error.to_string())?;
    // Publish only a fully written claim. If a process stops before publish,
    // only the .tmp file remains; if it stops after publish, the seeds are used.
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("create temporary seed claim {}: {error}", temp.display()))?;
    let write = (|| -> std::io::Result<()> {
        file.write_all(&body)?;
        file.sync_all()?;
        drop(file);
        fs::hard_link(&temp, &path)
    })();
    let _ = fs::remove_file(&temp);
    write.map_err(|error| format!("publish shared seed claim {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_ledger_detects_overlaps_and_only_resumes_the_original_output() {
        let ledger = std::env::temp_dir().join(format!(
            "intetrigence-seeds-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let round = SeedClaim {
            seeds: vec![62001, 62002],
            kind: "round".into(),
            identity: serde_json::json!({"output": "original"}),
        };
        register(&ledger, &round, false, false).unwrap();
        assert!(
            register(&ledger, &round, false, false)
                .unwrap_err()
                .contains("62001")
        );
        register(&ledger, &round, true, false).unwrap();
        let changed = SeedClaim {
            identity: serde_json::json!({"output": "another clone"}),
            ..round
        };
        assert!(register(&ledger, &changed, true, false).is_err());
        let lab = SeedClaim {
            seeds: vec![62002],
            kind: "lab".into(),
            identity: serde_json::json!({"output": "Lab"}),
        };
        assert!(
            register(&ledger, &lab, false, false)
                .unwrap_err()
                .contains("62002")
        );
        fs::remove_file(ledger.join("round-62001.json")).unwrap();
        fs::remove_dir(ledger).unwrap();
    }
}
