//! Parser for attax's `ATTAX_TBP_LOG` decision log
//! (`docs/attax-integration.md` §7).
//!
//! One JSON object per `suggest`. The log is the independent witness for
//! attax's own work (nodes, ms, hold decision) and for the 20-row vision
//! deviation (`cellsAboveRow20`, checklist F4).

use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AttaxDecision {
    #[serde(default)]
    pub ms: f64,
    #[serde(default)]
    pub nodes: u64,
    #[serde(default)]
    pub depth: u32,
    #[serde(default)]
    pub primary: Option<AttaxPrimary>,
    #[serde(default)]
    pub first: Option<serde_json::Value>,
    #[serde(default, rename = "cellsAboveRow20")]
    pub cells_above_row20: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AttaxPrimary {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub rotation: u32,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub tspin: String,
    #[serde(default)]
    pub hold: bool,
}

/// Reads every decision line. Malformed lines are skipped and counted.
#[derive(Clone, Debug, Default)]
pub struct AttaxLog {
    pub decisions: Vec<AttaxDecision>,
    pub malformed: usize,
}

impl AttaxLog {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut log = Self::default();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AttaxDecision>(&line) {
                Ok(decision) => log.decisions.push(decision),
                Err(_) => log.malformed += 1,
            }
        }
        Ok(log)
    }

    pub fn nodes_total(&self) -> u64 {
        self.decisions.iter().map(|decision| decision.nodes).sum()
    }

    pub fn ms_total(&self) -> f64 {
        self.decisions.iter().map(|decision| decision.ms).sum()
    }

    pub fn any_cells_above_row20(&self) -> bool {
        self.decisions
            .iter()
            .any(|decision| decision.cells_above_row20)
    }

    /// attax's own hold decision per suggestion (`primary.hold`).
    pub fn holds(&self) -> Vec<Option<bool>> {
        self.decisions
            .iter()
            .map(|decision| decision.primary.as_ref().map(|primary| primary.hold))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_log_line() {
        let line = r#"{"ms":38.43,"nodes":8653,"depth":5,"primary":{"kind":"I","rotation":0,"x":6,"y":18,"tspin":"none","hold":false},"first":{"location":{"type":"I","orientation":"north","x":7,"y":0},"spin":"none"},"cellsAboveRow20":false}"#;
        let decision: AttaxDecision = serde_json::from_str(line).unwrap();
        assert_eq!(decision.ms, 38.43);
        assert_eq!(decision.nodes, 8653);
        assert_eq!(decision.depth, 5);
        assert!(!decision.cells_above_row20);
        assert!(!decision.primary.unwrap().hold);
    }

    #[test]
    fn missing_fields_default() {
        let decision: AttaxDecision = serde_json::from_str("{}").unwrap();
        assert_eq!(decision.nodes, 0);
        assert!(decision.primary.is_none());
        assert!(!decision.cells_above_row20);
    }
}
