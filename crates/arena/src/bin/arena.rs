//! `arena` CLI: run an M0/M1 match, verify replays, rebuild the summary.
//!
//! ```text
//! arena match  [--out DIR] [--seeds 1001,1002,...] [--sheets A,B]
//!              [--turns 1000] [--frames 12] [--timeout 30]
//!              [--attax DIR] [--bun bun] [--no-attax]
//! arena verify <replay.jsonl | match-dir>
//! arena report <match-dir>
//! arena render --replay <replay.jsonl> --out <video.mp4> [--fps 60]
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use intetrigence_arena::engine::{Profile, Sheet};
use intetrigence_arena::harness::{
    AttaxConfig, BotChoice, MatchConfig, TbpOpponentConfig, apply_weight_overrides, load_audits,
    now_stamp, parse_weight_overrides, run_match, summarize, verify_all,
};
use intetrigence_arena::render::{RenderOptions, render_replay};
use intetrigence_arena::replay::verify_replay;
use intetrigence_bot::SearchConfig;

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
        "render" => cmd_render(&args[1..]),
        "reclassify" => cmd_reclassify(&args[1..]),
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
    let mut bot = "baseline".to_string();
    let mut ms = 100u64;
    let mut threads = 8usize;
    let mut profile = "official".to_string();
    let mut start_rows = 12u8;
    let mut interval = 8u32;
    let mut pressure_rows = 4u32;
    let mut from_turn = 200u32;
    let mut self_play = false;
    let mut tbp_program: Option<String> = None;
    let mut tbp_args: Vec<String> = Vec::new();
    let mut tbp_cwd: Option<PathBuf> = None;
    let mut tbp_visible_rows = 40u32;
    let mut tbp_suggest_delay_ms = 0u64;
    let mut tbp_retry_empty_ms = 0u64;
    let mut tbp_name = "tbp-bot".to_string();
    let mut attax_visible_rows = 0u32;
    let mut weight_overrides: Vec<(String, String)> = Vec::new();
    let mut attax_patch_sha256: Option<String> = None;

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
            "--rules" => {
                config.rules = value(1)?;
                index += 2;
            }
            "--protocol" => {
                config.protocol = value(1)?;
                index += 2;
            }
            "--no-attax" => {
                no_attax = true;
                index += 1;
            }
            "--bot" => {
                bot = value(1)?;
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
            "--note" => {
                config.notes.push(value(1)?);
                index += 2;
            }
            "--profile" => {
                profile = value(1)?;
                index += 2;
            }
            "--start-rows" => {
                start_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--start-rows: {error}"))?;
                index += 2;
            }
            "--interval" => {
                interval = value(1)?
                    .parse()
                    .map_err(|error| format!("--interval: {error}"))?;
                index += 2;
            }
            "--pressure-rows" => {
                pressure_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--pressure-rows: {error}"))?;
                index += 2;
            }
            "--from-turn" => {
                from_turn = value(1)?
                    .parse()
                    .map_err(|error| format!("--from-turn: {error}"))?;
                index += 2;
            }
            "--self-play" => {
                self_play = true;
                index += 1;
            }
            "--tbp-program" => {
                tbp_program = Some(value(1)?);
                index += 2;
            }
            "--tbp-args" => {
                tbp_args = value(1)?
                    .split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(ToString::to_string)
                    .collect();
                index += 2;
            }
            "--tbp-cwd" => {
                tbp_cwd = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--tbp-visible-rows" => {
                tbp_visible_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--tbp-visible-rows: {error}"))?;
                index += 2;
            }
            "--tbp-suggest-delay-ms" => {
                tbp_suggest_delay_ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--tbp-suggest-delay-ms: {error}"))?;
                index += 2;
            }
            "--tbp-retry-empty-ms" => {
                tbp_retry_empty_ms = value(1)?
                    .parse()
                    .map_err(|error| format!("--tbp-retry-empty-ms: {error}"))?;
                index += 2;
            }
            "--tbp-name" => {
                tbp_name = value(1)?;
                index += 2;
            }
            "--attax-visible-rows" => {
                attax_visible_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--attax-visible-rows: {error}"))?;
                index += 2;
            }
            "--weights" => {
                weight_overrides.extend(parse_weight_overrides(&value(1)?)?);
                index += 2;
            }
            "--attax-patch-sha256" => {
                attax_patch_sha256 = Some(value(1)?);
                index += 2;
            }
            other => return Err(format!("unknown flag `{other}`")),
        }
    }

    if let Some(program) = tbp_program {
        let mut opponent = TbpOpponentConfig::new(tbp_name, program, tbp_args);
        opponent.cwd = tbp_cwd;
        opponent.visible_rows = tbp_visible_rows;
        opponent.suggest_delay_ms = tbp_suggest_delay_ms;
        opponent.retry_empty_suggest_ms = tbp_retry_empty_ms;
        config.tbp_opponent = Some(opponent);
    } else if !no_attax && !self_play {
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
        attax_config.visible_rows = attax_visible_rows;
        attax_config.patch_sha256 = attax_patch_sha256;
        attax_config.timeout = Duration::from_secs(timeout);
        config.attax = Some(attax_config);
    }
    config.profile = match profile.as_str() {
        "official" => Profile::Official,
        "defense-stress" => Profile::DefenseStress { rows: start_rows },
        "garbage-pressure" => Profile::GarbagePressure {
            interval,
            rows: pressure_rows,
        },
        "composite" => Profile::Composite {
            start_rows,
            interval,
            rows: pressure_rows,
            from_turn,
        },
        other => {
            return Err(format!(
                "unknown --profile `{other}` (official|defense-stress|garbage-pressure|composite)"
            ));
        }
    };
    config.self_play = self_play;
    config.bot = match bot.as_str() {
        "baseline" => BotChoice::Baseline,
        "search" => {
            let mut search = SearchConfig {
                budget: Duration::from_millis(ms),
                threads,
                ..SearchConfig::default()
            };
            apply_weight_overrides(&mut search.weights, &weight_overrides)?;
            config.weights_note = weight_overrides
                .iter()
                .map(|(key, value)| format!("--weights {key}={value}"))
                .collect();
            BotChoice::Search(Box::new(search))
        }
        other => return Err(format!("unknown --bot `{other}` (baseline|search)")),
    };
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
        "summary: {} games, wins {} losses {} unresolved {} no-contest {}, verified {}/{}",
        summary.games.len(),
        summary.wins,
        summary.losses,
        summary.unresolved,
        summary.no_contest,
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
    let mut dir: Option<PathBuf> = None;
    let mut notes = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--note" => {
                notes.push(
                    args.get(index + 1)
                        .cloned()
                        .ok_or_else(|| "--note needs a value".to_string())?,
                );
                index += 2;
            }
            other => {
                dir = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }
    let dir = dir.ok_or_else(|| "report needs a match directory".to_string())?;
    if notes.is_empty() {
        notes.push("The intetrigence side is the bot recorded in each replay header.".into());
    }
    let audits = load_audits(&dir)?;
    if audits.is_empty() {
        return Err(format!("no audits under {}", dir.join("audit").display()));
    }
    let summary = summarize(&audits, notes);
    summary
        .write(&dir)
        .map_err(|error| format!("write summary: {error}"))?;
    println!(
        "{} games: wins {} losses {} unresolved {} no-contest {}, verified {}/{}",
        summary.games.len(),
        summary.wins,
        summary.losses,
        summary.unresolved,
        summary.no_contest,
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

fn cmd_render(args: &[String]) -> Result<(), String> {
    let mut replay: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut options = RenderOptions::default();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].clone();
        let value = |offset: usize| -> Result<String, String> {
            args.get(index + offset)
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--replay" => {
                replay = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--out" => {
                out = Some(PathBuf::from(value(1)?));
                index += 2;
            }
            "--fps" => {
                options.fps = value(1)?
                    .parse()
                    .map_err(|error| format!("--fps: {error}"))?;
                index += 2;
            }
            "--width" => {
                options.width = value(1)?
                    .parse()
                    .map_err(|error| format!("--width: {error}"))?;
                index += 2;
            }
            "--height" => {
                options.height = value(1)?
                    .parse()
                    .map_err(|error| format!("--height: {error}"))?;
                index += 2;
            }
            "--end-card" => {
                options.end_card_seconds = value(1)?
                    .parse()
                    .map_err(|error| format!("--end-card: {error}"))?;
                index += 2;
            }
            "--max-turns" => {
                options.max_turns = Some(
                    value(1)?
                        .parse()
                        .map_err(|error| format!("--max-turns: {error}"))?,
                );
                index += 2;
            }
            other => return Err(format!("unknown render flag `{other}`")),
        }
    }
    let replay = replay.ok_or_else(|| "render needs --replay <path>".to_string())?;
    let out = out.ok_or_else(|| "render needs --out <video.mp4>".to_string())?;
    let report = render_replay(&replay, &out, &options)?;
    println!(
        "{}",
        serde_json::json!({
            "replay": replay.display().to_string(),
            "output": report.output.display().to_string(),
            "seed": report.seed,
            "sheet": report.sheet,
            "turns": report.turns,
            "frames": report.frames,
            "duration_seconds": report.duration_seconds,
            "winner": report.winner,
            "reason": report.reason,
            "forfeit_cause": report.forfeit_cause,
            "verified": report.verified,
        })
    );
    Ok(())
}

fn cmd_reclassify(args: &[String]) -> Result<(), String> {
    let mut path: Option<PathBuf> = None;
    let mut visible_rows = 40u32;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].clone();
        let value = |offset: usize| -> Result<String, String> {
            args.get(index + offset)
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--visible-rows" => {
                visible_rows = value(1)?
                    .parse()
                    .map_err(|error| format!("--visible-rows: {error}"))?;
                index += 2;
            }
            other => {
                path = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }
    let path =
        path.ok_or_else(|| "reclassify needs a match directory or replay file".to_string())?;
    let path = path.as_path();
    let mut files = Vec::new();
    if path.is_dir() {
        let replays = path.join("replays");
        let entries = std::fs::read_dir(&replays)
            .map_err(|error| format!("read {}: {error}", replays.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let file = entry.path();
            if file.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                files.push(file);
            }
        }
    } else {
        files.push(path.to_path_buf());
    }
    files.sort();
    let mut games = 0u32;
    let mut topout_wins = 0u32;
    let mut topout_losses = 0u32;
    let mut stuck = 0u32;
    let mut unresolved = 0u32;
    let mut no_contest = 0u32;
    let mut forfeit_vision = 0u32;
    let mut forfeit_other = 0u32;
    let mut forfeit_unknown = 0u32;
    let mut verified = 0u32;
    let mut cause_mismatches = 0u32;
    let mut game_rows: Vec<serde_json::Value> = Vec::new();
    for file in &files {
        let records = intetrigence_arena::replay::read_replay(file)
            .map_err(|error| format!("read {}: {error}", file.display()))?;
        let result = records.iter().find_map(|record| match record {
            intetrigence_arena::ReplayRecord::Result(result) => Some(result),
            _ => None,
        });
        let Some(result) = result else {
            continue;
        };
        let report = intetrigence_arena::replay::verify_replay_with_visible_rows(
            file,
            Some([visible_rows, visible_rows]),
        )
        .map_err(|error| format!("verify {}: {error}", file.display()))?;
        games += 1;
        let non_cause = report
            .mismatches
            .iter()
            .filter(|mismatch| !mismatch.contains("forfeit_cause"))
            .count();
        if non_cause == 0 {
            verified += 1;
        }
        cause_mismatches += report
            .mismatches
            .iter()
            .filter(|mismatch| mismatch.contains("forfeit_cause"))
            .count() as u32;
        let audit_path = if path.is_dir() {
            path.join("audit")
                .join(format!("seed-{}-sheet-{}.json", report.seed, report.sheet))
        } else {
            path.with_extension("json")
        };
        let (attax_max, intetrigence_max) = std::fs::read_to_string(&audit_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|audit| {
                let heights = audit.get("heights")?.as_array()?;
                let h0 = heights.first()?.get("max")?.as_u64()?;
                let h1 = heights.get(1)?.get("max")?.as_u64()?;
                Some((h1, h0))
            })
            .unwrap_or((0, 0));
        let winner_effective = if result.reason == "TopOut" {
            result.winner
        } else {
            None
        };
        game_rows.push(serde_json::json!({
            "seed": report.seed,
            "sheet": report.sheet,
            "winner": result.winner,
            "winner_effective": winner_effective,
            "reason": result.reason,
            "forfeit_cause": report.forfeit_cause.map(|cause| cause.as_str()),
            "forfeit_slot": result.forfeit_slot,
            "turns": result.turns,
            "attax_max_height": attax_max,
            "intetrigence_max_height": intetrigence_max,
        }));
        match result.reason.as_str() {
            "TopOut" => {
                if result.winner == Some(0) {
                    topout_wins += 1;
                } else if result.winner == Some(1) {
                    topout_losses += 1;
                }
            }
            "Stuck" => stuck += 1,
            "TurnCap" => unresolved += 1,
            "Forfeit" => {
                no_contest += 1;
                match report.forfeit_cause {
                    Some(intetrigence_arena::replay::ForfeitCause::Vision) => forfeit_vision += 1,
                    Some(intetrigence_arena::replay::ForfeitCause::Other) => forfeit_other += 1,
                    _ => forfeit_unknown += 1,
                }
            }
            _ => {}
        }
    }
    let summary = serde_json::json!({
        "dir": path.display().to_string(),
        "games": games,
        "wins": topout_wins,
        "losses": topout_losses,
        "unresolved": unresolved,
        "no_contest": no_contest,
        "stuck": stuck,
        "topout_wins": topout_wins,
        "topout_losses": topout_losses,
        "forfeit_vision": forfeit_vision,
        "forfeit_other": forfeit_other,
        "forfeit_unknown": forfeit_unknown,
        "gate_wins": topout_wins,
        "verified": verified,
        "cause_mismatches": cause_mismatches,
        "game_rows": game_rows,
    });
    let out = if path.is_dir() {
        path.join("summary-reclassified.json")
    } else {
        path.with_extension("reclassified.json")
    };
    std::fs::write(
        &out,
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
            + "
",
    )
    .map_err(|error| format!("write {}: {error}", out.display()))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
    );
    println!("wrote {}", out.display());
    Ok(())
}
