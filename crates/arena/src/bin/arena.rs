//! `arena` CLI: run an M0/M1 match, verify replays, rebuild the summary.
//!
//! ```text
//! arena match  [--out DIR] [--seeds 1001,1002,...] [--sheets A,B]
//!              [--turns 1000] [--frames 12] [--timeout 30]
//!              [--attax DIR] [--bun bun] [--no-attax]
//! arena verify <replay.jsonl | match-dir>
//! arena report <match-dir>
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use intetrigence_arena::engine::Sheet;
use intetrigence_arena::harness::{
    AttaxConfig, MatchConfig, load_audits, now_stamp, run_match, summarize, verify_all,
};
use intetrigence_arena::replay::verify_replay;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        std::process::exit(2);
    }
    let result = match args[0].as_str() {
        "match" => cmd_match(&args[1..]),
        "verify" => cmd_verify(&args[1..]),
        "report" => cmd_report(&args[1..]),
        "-h" | "--help" | "help" => {
            usage();
            Ok(())
        }
        other => Err(format!("unknown command `{other}`")),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn usage() {
    eprintln!(
        "usage:\n  arena match  [--out DIR] [--seeds 1001,1002,...] [--sheets A,B]\n               [--turns 1000] [--frames 12] [--timeout 30]\n               [--attax DIR] [--bun bun] [--no-attax]\n  arena verify <replay.jsonl | match-dir>\n  arena report <match-dir>"
    );
}

fn cmd_match(args: &[String]) -> Result<(), String> {
    let mut config = MatchConfig::new("out/m0");
    let mut attax: Option<PathBuf> = None;
    let mut bun = "bun".to_string();
    let mut timeout = 30u64;
    let mut no_attax = false;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].clone();
        let value = |offset: usize| -> Result<String, String> {
            args.get(index + offset)
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--out" => {
                config.out_dir = PathBuf::from(value(1)?);
                index += 2;
            }
            "--seeds" => {
                config.seeds = parse_seeds(&value(1)?)?;
                index += 2;
            }
            "--sheets" => {
                config.sheets = parse_sheets(&value(1)?)?;
                index += 2;
            }
            "--turns" => {
                config.turn_cap = value(1)?
                    .parse()
                    .map_err(|error| format!("--turns: {error}"))?;
                index += 2;
            }
            "--frames" => {
                config.frames_per_turn = value(1)?
                    .parse()
                    .map_err(|error| format!("--frames: {error}"))?;
                index += 2;
            }
            "--timeout" => {
                timeout = value(1)?
                    .parse()
                    .map_err(|error| format!("--timeout: {error}"))?;
                index += 2;
            }
            "--attax" => {
                attax = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--bun" => {
                bun = value(1)?;
                index += 2;
            }
            "--no-attax" => {
                no_attax = true;
                index += 1;
            }
            other => return Err(format!("unknown flag `{other}`")),
        }
    }

    if !no_attax {
        let dir = attax.ok_or_else(|| "match needs --attax DIR or --no-attax".to_string())?;
        let entry = dir.join("tbp").join("attax-tbp.ts");
        if !entry.exists() {
            return Err(format!("attax entry point not found: {}", entry.display()));
        }
        let commit = git_commit(&dir);
        let bun_version = command_version(&bun);
        println!(
            "attax: {} commit {} bun {}",
            dir.display(),
            commit.as_deref().unwrap_or("unknown"),
            bun_version.as_deref().unwrap_or("unknown")
        );
        let mut attax_config = AttaxConfig::new(dir);
        attax_config.bun = bun;
        attax_config.commit = commit;
        attax_config.bun_version = bun_version;
        attax_config.timeout = Duration::from_secs(timeout);
        config.attax = Some(attax_config);
    }
    config.started_at = now_stamp();

    println!(
        "match: seeds {:?} sheets {:?} turns {} -> {}",
        config.seeds,
        config
            .sheets
            .iter()
            .map(|sheet| sheet.as_str())
            .collect::<Vec<_>>(),
        config.turn_cap,
        config.out_dir.display()
    );
    let summary = run_match(&config)?;
    println!(
        "summary: {} games, wins {} losses {} draws {} forfeits {}, verified {}/{}",
        summary.games.len(),
        summary.wins,
        summary.losses,
        summary.draws,
        summary.forfeits,
        summary.verified,
        summary.games.len()
    );
    println!(
        "wrote {}/summary.json and {}/summary.md",
        config.out_dir.display(),
        config.out_dir.display()
    );
    if summary.verify_failures > 0 {
        return Err(format!(
            "{} replay(s) failed verification",
            summary.verify_failures
        ));
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> Result<(), String> {
    let path = args
        .first()
        .ok_or_else(|| "verify needs a replay file or match directory".to_string())?;
    let path = Path::new(path);
    let mut failures = 0;
    if path.is_dir() {
        for (file, report) in verify_all(path)? {
            println!(
                "{file}: ok={} turns={} placements={} mismatches={}",
                report.ok,
                report.turns,
                report.placements,
                report.mismatches.len()
            );
            for mismatch in report.mismatches.iter().take(10) {
                println!("  - {mismatch}");
            }
            if !report.ok {
                failures += 1;
            }
        }
    } else {
        let report = verify_replay(path).map_err(|error| error.to_string())?;
        println!(
            "{}: ok={} seed={} sheet={} turns={} placements={} mismatches={}",
            path.display(),
            report.ok,
            report.seed,
            report.sheet,
            report.turns,
            report.placements,
            report.mismatches.len()
        );
        for mismatch in report.mismatches.iter().take(20) {
            println!("  - {mismatch}");
        }
        if !report.ok {
            failures += 1;
        }
    }
    if failures > 0 {
        return Err(format!("{failures} replay(s) failed verification"));
    }
    Ok(())
}

fn cmd_report(args: &[String]) -> Result<(), String> {
    let dir = args
        .first()
        .ok_or_else(|| "report needs a match directory".to_string())?;
    let dir = Path::new(dir);
    let audits = load_audits(dir)?;
    if audits.is_empty() {
        return Err(format!("no audits under {}", dir.join("audit").display()));
    }
    let summary = summarize(&audits);
    summary
        .write(dir)
        .map_err(|error| format!("write summary: {error}"))?;
    println!(
        "{} games: wins {} losses {} draws {} forfeits {}, verified {}/{}",
        summary.games.len(),
        summary.wins,
        summary.losses,
        summary.draws,
        summary.forfeits,
        summary.verified,
        summary.games.len()
    );
    println!(
        "win rate {:.1}% (Wilson 95% CI {:.1}%..{:.1}%)",
        summary.win_rate * 100.0,
        summary.wilson_low * 100.0,
        summary.wilson_high * 100.0
    );
    Ok(())
}

fn parse_seeds(text: &str) -> Result<Vec<u64>, String> {
    text.split(',')
        .map(|part| {
            part.trim()
                .parse::<u64>()
                .map_err(|error| format!("bad seed `{part}`: {error}"))
        })
        .collect()
}

fn parse_sheets(text: &str) -> Result<Vec<Sheet>, String> {
    let mut sheets = Vec::new();
    for part in text.split(',') {
        let sheet = Sheet::parse(part).ok_or_else(|| format!("bad sheet `{part}`"))?;
        if !sheets.contains(&sheet) {
            sheets.push(sheet);
        }
    }
    Ok(sheets)
}

fn git_commit(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_version(program: &str) -> Option<String> {
    let output = Command::new(program).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
