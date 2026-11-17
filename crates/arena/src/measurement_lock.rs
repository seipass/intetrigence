//! Process-wide exclusion for the benchmark entry points. The owner file is
//! shared with the Lab runner and Bash wrappers.

use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const OWNER_TOKEN: &str = "INTETRIGENCE_MEASUREMENT_OWNER";

pub(crate) fn lock_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("MEASUREMENT_LOCK_DIR").filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err("MEASUREMENT_LOCK_DIR must be an absolute path".into());
        }
        return Ok(path);
    }
    let base = if cfg!(windows) {
        env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData/Local"))
            })
    } else {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache"))
    }
    .filter(|base| base.is_absolute())
    .ok_or("cannot locate the measurement lock directory; set MEASUREMENT_LOCK_DIR")?;
    Ok(base.join("intetrigence/measurement.lock"))
}

pub struct MeasurementLock {
    owned: Option<(PathBuf, String)>,
}

impl MeasurementLock {
    pub fn acquire(label: &str) -> Result<Self, String> {
        let path = lock_path()?;
        let owner_file = path.join("owner");
        let parent = path
            .parent()
            .ok_or("measurement lock has no parent directory")?;
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create measurement lock parent {}: {error}",
                parent.display()
            )
        })?;
        match fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                let owner =
                    fs::read_to_string(owner_file).unwrap_or_else(|_| "unknown owner".into());
                if env::var(OWNER_TOKEN)
                    .is_ok_and(|token| !token.is_empty() && token == owner.trim_end())
                {
                    return Ok(Self { owned: None });
                }
                return Err(format!(
                    "another measurement is running ({}) at {}",
                    owner.trim_end(),
                    path.display()
                ));
            }
            Err(error) => {
                return Err(format!(
                    "acquire measurement lock {}: {error}",
                    path.display()
                ));
            }
        }
        let owner = format!(
            "{label} pid={} started={}",
            std::process::id(),
            crate::harness::now_stamp()
        );
        if let Err(error) = fs::write(&owner_file, &owner) {
            let _ = fs::remove_file(owner_file);
            let _ = fs::remove_dir(&path);
            return Err(format!(
                "record measurement owner {}: {error}",
                path.display()
            ));
        }
        Ok(Self {
            owned: Some((path, owner)),
        })
    }
}

impl Drop for MeasurementLock {
    fn drop(&mut self) {
        if let Some((path, owner)) = &self.owned {
            release(path, owner);
        }
    }
}

fn release(path: &Path, owner: &str) {
    let owner_file = path.join("owner");
    if fs::read_to_string(&owner_file).is_ok_and(|current| current == owner) {
        let _ = fs::remove_file(owner_file);
        let _ = fs::remove_dir(path);
    }
}
