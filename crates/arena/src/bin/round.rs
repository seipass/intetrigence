//! `round` CLI: staged round runner (protocol v1.13 §1.4/§1.5).
//!
//! ```text
//! round run --round 1 --seeds-file docs/seeds-round-1.md --out scripts/evidence \
//!           --attax-dir C:/Users/Hasilan/Projects/attax \
//!           --cc2-program <program> --cc2-args <a,b> --cc2-commit <sha> \
//!           --clys-program <program> --clys-args <a,b> --clys-commit <sha> \
//!           --ms 100 --threads 8
//! ```
//!
//! A dry run uses already-burned seeds and stops after stage 1:
//! `round run --round 0 --seeds 9001 --out scripts/evidence --attax-dir ... --dry-run`

use std::path::PathBuf;
use std::time::Duration;

use intetrigence_arena::harness::BotChoice;
use intetrigence_arena::harness::TbpOpponentConfig;
use intetrigence_arena::round::{
    OpponentKind, OpponentSpec, RoundSpec, check_seed_discipline, current_build,
    default_report_path, head_commit, read_seeds_file, run_round,
};
use intetrigence_bot::SearchConfig;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        std::process::exit(2);
    }
    let result = match args[0].as_str() {
        "run" => cmd_run(&args[1..]),
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
        "usage:\n  round run --round N (--seeds-file PATH | --seeds a,b,c) --out DIR\n\
         \x20   [--attax-dir DIR] [--attax-bun bun] [--attax-commit SHA]\n\
         \x20   [--cc2-program P] [--cc2-args a,b] [--cc2-commit SHA] [--cc2-visible-rows 40]\n\
         \x20   [--clys-program P] [--clys-args a,b] [--clys-commit SHA] [--clys-visible-rows 40]\n\
         \x20   [--gate-attax 50] [--gate-cc2 48] [--gate-clys 48]\n\
         \x20   [--ms 100] [--threads 8] [--turns 1000] [--max-stages 3] [--dry-run]"
    );
}

fn parse_env(text: &str) -> Vec<(String, String)> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            part.split_once('=')
                .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn parse_args(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    let mut round = 0u32;
    let mut seeds_file: Option<PathBuf> = None;
    let mut seeds_override: Option<Vec<u64>> = None;
    let mut out_root = PathBuf::from("scripts/evidence");
    let mut attax_dir: Option<PathBuf> = None;
    let mut attax_bun = "bun".to_string();
    let mut attax_commit: Option<String> = None;
    let mut attax_patch: Option<String> = None;
    let mut cc2_program: Option<String> = None;
    let mut cc2_args: Vec<String> = Vec::new();
    let mut cc2_commit: Option<String> = None;
    let mut cc2_patch: Option<String> = None;
    let mut cc2_visible_rows = 40u32;
    let mut cc2_suggest_delay_ms = 100u64;
    let mut cc2_retry_empty_ms = 2000u64;
    let mut cc2_cwd: Option<PathBuf> = None;
    let mut cc2_env: Vec<(String, String)> = Vec::new();
    let mut clys_program: Option<String> = None;
    let mut clys_args: Vec<String> = Vec::new();
    let mut clys_commit: Option<String> = None;
    let mut clys_patch: Option<String> = None;
    let mut clys_visible_rows = 40u32;
    let mut clys_suggest_delay_ms = 0u64;
    let mut clys_retry_empty_ms = 0u64;
    let mut clys_cwd: Option<PathBuf> = None;
    let mut clys_env: Vec<(String, String)> = Vec::new();
    let mut gates = [50u32, 48, 48];
    let mut ms = 100u64;
    let mut threads = 8usize;
    let mut turn_cap = 1000u32;
    let mut max_stages: Option<usize> = None;
    let mut report_path: Option<PathBuf> = None;
    let mut dry_run = false;
    let mut check_only = false;
    let mut resume = false;
    let mut resume_reason: Option<String> = None;

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].clone();
        let value = |offset: usize| -> Result<String, String> {
            args.get(index + offset)
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--round" => {
                round = value(1)?
                    .parse()
                    .map_err(|error| format!("--round: {error}"))?;
                index += 2;
            }
            "--seeds-file" => {
                seeds_file = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--seeds" => {
                let seeds = value(1)?
                    .split(',')
                    .map(|part| {
                        part.trim()
                            .parse::<u64>()
                            .map_err(|error| format!("bad seed `{part}`: {error}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                seeds_override = Some(seeds);
                index += 2;
            }
            "--out" => {
                out_root = PathBuf::from(value(1)?);
                index += 2;
            }
            "--attax-dir" => {
                attax_dir = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--attax-bun" => {
                attax_bun = value(1)?;
                index += 2;
            }
            "--attax-commit" => {
                attax_commit = Some(value(1)?);
                index += 2;
            }
            "--attax-patch-note" => {
                attax_patch = Some(value(1)?);
                index += 2;
            }
            "--cc2-program" => {
                cc2_program = Some(value(1)?);
                index += 2;
            }
            "--cc2-args" => {
                cc2_args = parse_args(&value(1)?);
                index += 2;
            }
            "--cc2-commit" => {
                cc2_commit = Some(value(1)?);
                index += 2;
            }
            "--cc2-patch-note" => {
                cc2_patch = Some(value(1)?);
                index += 2;
            }
            "--cc2-visible-rows" => {
                cc2_visible_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--cc2-visible-rows: {error}"))?;
                index += 2;
            }
            "--cc2-suggest-delay-ms" => {
                cc2_suggest_delay_ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--cc2-suggest-delay-ms: {error}"))?;
                index += 2;
            }
            "--cc2-retry-empty-ms" => {
                cc2_retry_empty_ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--cc2-retry-empty-ms: {error}"))?;
                index += 2;
            }
            "--cc2-cwd" => {
                cc2_cwd = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--cc2-env" => {
                cc2_env = parse_env(&value(1)?);
                index += 2;
            }
            "--clys-program" => {
                clys_program = Some(value(1)?);
                index += 2;
            }
            "--clys-args" => {
                clys_args = parse_args(&value(1)?);
                index += 2;
            }
            "--clys-commit" => {
                clys_commit = Some(value(1)?);
                index += 2;
            }
            "--clys-patch-note" => {
                clys_patch = Some(value(1)?);
                index += 2;
            }
            "--clys-visible-rows" => {
                clys_visible_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--clys-visible-rows: {error}"))?;
                index += 2;
            }
            "--clys-suggest-delay-ms" => {
                clys_suggest_delay_ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--clys-suggest-delay-ms: {error}"))?;
                index += 2;
            }
            "--clys-retry-empty-ms" => {
                clys_retry_empty_ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--clys-retry-empty-ms: {error}"))?;
                index += 2;
            }
            "--clys-cwd" => {
                clys_cwd = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--clys-env" => {
                clys_env = parse_env(&value(1)?);
                index += 2;
            }
            "--gate-attax" => {
                gates[0] = value(1)?
                    .parse()
                    .map_err(|error| format!("--gate-attax: {error}"))?;
                index += 2;
            }
            "--gate-cc2" => {
                gates[1] = value(1)?
                    .parse()
                    .map_err(|error| format!("--gate-cc2: {error}"))?;
                index += 2;
            }
            "--gate-clys" => {
                gates[2] = value(1)?
                    .parse()
                    .map_err(|error| format!("--gate-clys: {error}"))?;
                index += 2;
            }
            "--ms" => {
                ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--ms: {error}"))?;
                index += 2;
            }
            "--threads" => {
                threads = value(1)?
                    .parse()
                    .map_err(|error| format!("--threads: {error}"))?;
                index += 2;
            }
            "--turns" => {
                turn_cap = value(1)?
                    .parse()
                    .map_err(|error| format!("--turns: {error}"))?;
                index += 2;
            }
            "--max-stages" => {
                max_stages = Some(
                    value(1)?
                        .parse()
                        .map_err(|error| format!("--max-stages: {error}"))?,
                );
                index += 2;
            }
            "--report" => {
                report_path = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            "--check-only" => {
                check_only = true;
                index += 1;
            }
            "--resume" => {
                resume = true;
                index += 1;
            }
            "--resume-reason" => {
                resume_reason = Some(value(1)?);
                index += 2;
            }
            other => return Err(format!("unknown round flag `{other}`")),
        }
    }

    let max_stages = max_stages.unwrap_or(if dry_run { 1 } else { 3 });
    let build_commit = head_commit();
    let seeds_commit =
        check_seed_discipline(seeds_file.as_deref(), build_commit.as_deref(), !dry_run)?;
    let (seeds, seeds_source) = match (seeds_override, &seeds_file) {
        (Some(seeds), _) => (seeds, "<--seeds override>".to_string()),
        (None, Some(path)) => {
            let file = read_seeds_file(path)?;
            (file.seeds, path.display().to_string())
        }
        (None, None) => {
            return Err("run needs --seeds-file or --seeds".into());
        }
    };
    if !dry_run && seeds.len() != 25 {
        return Err(format!(
            "a round needs 25 evaluation seeds, found {}",
            seeds.len()
        ));
    }

    if check_only {
        println!(
            "seeds: {} values ({} .. {}) from {}",
            seeds.len(),
            seeds.first().copied().unwrap_or(0),
            seeds.last().copied().unwrap_or(0),
            seeds_source
        );
        println!(
            "seeds commit: {}",
            seeds_commit.as_deref().unwrap_or("uncommitted")
        );
        println!(
            "build commit: {}",
            build_commit.as_deref().unwrap_or("unknown")
        );
        println!("gates: {:?}", gates);
        return Ok(());
    }

    let attax_commit = attax_commit.or_else(|| {
        attax_dir.as_ref().and_then(|dir| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        })
    });
    let attax = match attax_dir {
        Some(dir) => OpponentSpec {
            key: "attax".into(),
            display: "attax".into(),
            commit: attax_commit.clone(),
            patch_note: attax_patch.clone(),
            kind: OpponentKind::Attax {
                dir,
                bun: attax_bun,
                commit: attax_commit,
            },
        },
        None => OpponentSpec {
            key: "attax".into(),
            display: "attax".into(),
            commit: None,
            patch_note: attax_patch.clone(),
            kind: OpponentKind::Missing,
        },
    };
    let cc2 = match cc2_program {
        Some(program) => {
            let mut config = TbpOpponentConfig::new("cc2", program, cc2_args);
            config.commit = cc2_commit.clone();
            config.visible_rows = cc2_visible_rows;
            config.suggest_delay_ms = cc2_suggest_delay_ms;
            config.retry_empty_suggest_ms = cc2_retry_empty_ms;
            config.cwd = cc2_cwd;
            config.env = cc2_env;
            OpponentSpec {
                key: "cc2".into(),
                display: "Cold Clear 2".into(),
                commit: cc2_commit,
                patch_note: cc2_patch,
                kind: OpponentKind::Tbp(config),
            }
        }
        None => OpponentSpec {
            key: "cc2".into(),
            display: "Cold Clear 2".into(),
            commit: None,
            patch_note: cc2_patch,
            kind: OpponentKind::Missing,
        },
    };
    let clys = match clys_program {
        Some(program) => {
            let mut config = TbpOpponentConfig::new("clys", program, clys_args);
            config.commit = clys_commit.clone();
            config.visible_rows = clys_visible_rows;
            config.suggest_delay_ms = clys_suggest_delay_ms;
            config.retry_empty_suggest_ms = clys_retry_empty_ms;
            config.cwd = clys_cwd;
            config.env = clys_env;
            OpponentSpec {
                key: "clys".into(),
                display: "clys".into(),
                commit: clys_commit,
                patch_note: clys_patch,
                kind: OpponentKind::Tbp(config),
            }
        }
        None => OpponentSpec {
            key: "clys".into(),
            display: "clys".into(),
            commit: None,
            patch_note: clys_patch,
            kind: OpponentKind::Missing,
        },
    };

    let spec = RoundSpec {
        round,
        seeds,
        seeds_source,
        seeds_commit,
        build_commit: build_commit.clone(),
        out_root,
        report_path: report_path.unwrap_or_else(|| default_report_path(round)),
        bot: BotChoice::Search(Box::new(SearchConfig {
            budget: Duration::from_millis(ms),
            threads,
            ..SearchConfig::default()
        })),
        turn_cap,
        profile: intetrigence_arena::engine::Profile::Official,
        gates,
        opponents: [attax, cc2, clys],
        notes: vec![
            "Staged round (protocol v1.13 §1.4); evaluation seeds are pre-committed and burned after use.".into(),
        ],
        max_stages,
        resume,
        resume_reason,
    };
    let report = run_round(&spec)?;
    println!(
        "round {}: {} stage(s), completed={}, stopped_at={:?}",
        report.round,
        report.stages.len(),
        report.completed,
        report.stopped_at
    );
    for stage in &report.stages {
        println!(
            "  {}: {}-{}-{} (gate {}) {} — {}",
            stage.key,
            stage.wins,
            stage.losses,
            stage.draws,
            stage.gate,
            if stage.passed { "PASSED" } else { "FAILED" },
            stage.reason
        );
    }
    println!("report: {}", spec.report_path.display());
    if !report.completed {
        eprintln!(
            "warning: round did not complete; a retry needs a new build and a new pre-committed seed set (protocol v1.13 §1.4)"
        );
    }
    if !dry_run {
        eprintln!("warning: mark the evaluation seed range as burned in docs/seed-registry.md");
    }
    let _ = current_build();
    Ok(())
}
