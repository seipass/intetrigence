#[cfg(feature = "battle-bot")]
use intetrigence::battle_ws;
use intetrigence::{
    arena, battle,
    game::{Game, RulesProfile},
    hoiko::{config_from_home, HoikoConfig, HoikoSearcher},
    search::{choose, Budget},
};
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{self, BufRead, Write},
    path::Path,
};
fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let arg = |key: &str| {
        args.iter()
            .position(|a| a == key)
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
    };
    let arg_any = |keys: &[&str]| keys.iter().find_map(|key| arg(key));
    let number = |key: &str, default: u64| -> Result<u64, Box<dyn std::error::Error>> {
        Ok(arg(key).map(str::parse).transpose()?.unwrap_or(default))
    };
    let budget = Budget {
        milliseconds: number("--ms", 20)?,
        iterations: number("--iterations", 0)?,
    };
    if budget.milliseconds == 0 && budget.iterations == 0 {
        return Err("positive --ms or --iterations required".into());
    }
    let weights: Option<Value> = arg("--weights")
        .map(|p| -> Result<_, Box<dyn std::error::Error>> {
            Ok(serde_json::from_reader(File::open(p)?)?)
        })
        .transpose()?;
    if let Some(ref w) = weights {
        let _: intetrigence_engine::bot::BotConfig = serde_json::from_value(w.clone())?;
    }
    let pps = arg("--pps")
        .map(str::parse::<f64>)
        .transpose()?
        .map(|value| {
            if value.is_finite() && value > 0.0 {
                Ok(value)
            } else {
                Err("--pps must be a finite positive number")
            }
        })
        .transpose()?;
    let rules = arg("--rules")
        .map(|name| {
            RulesProfile::parse(name).ok_or_else(|| format!("unsupported --rules profile: {name}"))
        })
        .transpose()?
        .unwrap_or_default();
    match args.get(1).map(String::as_str).unwrap_or("help") {
        "arena" => {
            let n = number("--games",20)? as u32;
            let seed = number("--seed",1)?;
            let limit = number("--turns",1000)? as u32;
            if n == 0 || limit == 0 { return Err("games and turns must be positive".into()); }
            let mut replay = arg("--replay").map(File::create).transpose()?;
            let mut wins=0; let mut losses=0; let mut draws=0;
            let mut matches=Vec::new();
            for i in 0..n {
                let r = arena::run_with_pps_rules(seed+i as u64/2, i%2==1, limit, budget, weights.as_ref(), pps, rules, replay.as_mut().map(|f|f as &mut dyn Write))?;
                match r.winner { None=>draws+=1, Some(p) if (p==0) != r.swapped=>wins+=1, _=>losses+=1 }
                eprintln!("game {}/{n}: {wins}W {losses}L {draws}D, {} turns",i+1,r.turns);
                matches.push(r);
            }
            let report=json!({"wins":wins,"losses":losses,"draws":draws,"decisive_wilson95_descriptive_only":arena::wilson(wins,wins+losses),"paired_victory_bound":arena::paired_victory_bound(&matches).ok(),"ms":budget.milliseconds,"iterations":budget.iterations,"pps":pps,"matches":matches,"note":"paired seeds; equal per-move budgets; one thread each; both searches retain trees until state changes; fixed turn cadence when --pps is provided"});
            println!("{}",serde_json::to_string_pretty(&report)?);
            if let Some(path)=arg("--output") { std::fs::write(path,serde_json::to_vec_pretty(&report)?)?; }
        }
        "hoiko-arena" => {
            let n = number("--games", 2)? as u32;
            let seed = number("--seed", 1)?;
            let limit = number("--turns", 1000)? as u32;
            if n == 0 || limit == 0 { return Err("games and turns must be positive".into()); }
            let config = arg("--hoiko-config")
                .map(HoikoConfig::load_or_default)
                .unwrap_or_else(config_from_home);
            let mut replay = arg("--replay").map(File::create).transpose()?;
            let mut wins = 0;
            let mut losses = 0;
            let mut draws = 0;
            let mut matches = Vec::new();
            for i in 0..n {
                let result = arena::run_hoiko_with_pps_rules(
                    seed + i as u64 / 2,
                    i % 2 == 1,
                    limit,
                    budget,
                    pps,
                    rules,
                    &config,
                    replay.as_mut().map(|file| file as &mut dyn Write),
                )?;
                match result.winner {
                    None => draws += 1,
                    Some(player) if (player == 0) != result.swapped => wins += 1,
                    _ => losses += 1,
                }
                eprintln!("game {}/{n}: {wins}W {losses}L {draws}D, {} turns", i + 1, result.turns);
                matches.push(result);
            }
            let report = json!({
                "agent":"hoiko",
                "opponent":"intetrigence",
                "wins":wins,
                "losses":losses,
                "draws":draws,
                "decisive_wilson95_descriptive_only":arena::wilson(wins,wins+losses),
                "paired_victory_bound":arena::paired_victory_bound(&matches).ok(),
                "ms":budget.milliseconds,
                "iterations":budget.iterations,
                "pps":pps,
                "rules":rules.name(),
                "matches":matches,
            });
            println!("{}",serde_json::to_string_pretty(&report)?);
            if let Some(path)=arg("--output") { std::fs::write(path,serde_json::to_vec_pretty(&report)?)?; }
        }
        "battle" | "movement-battle" => {
            if pps.is_some() {
                return Err("battle derives speed from frame inputs; --pps is not accepted".into());
            }
            let first = arg_any(&["--p1", "--player1", "--first"])
                .ok_or("--p1 AGENT required (hoiko|intetrigence|cold_clear_2)")?;
            let second = arg_any(&["--p2", "--player2", "--second"])
                .ok_or("--p2 AGENT required (hoiko|intetrigence|cold_clear_2)")?;
            let first = arena::MatchAgent::parse(first)
                .ok_or_else(|| format!("unsupported --p1 agent: {first}"))?;
            let second = arena::MatchAgent::parse(second)
                .ok_or_else(|| format!("unsupported --p2 agent: {second}"))?;
            let games = number("--games", 2)? as u32;
            let seed = number("--seed", 1)?;
            let piece_limit = number("--turns", 1000)? as u32;
            if games == 0 || piece_limit == 0 {
                return Err("games and turns must be positive".into());
            }
            let opponent = arg("--opponent")
                .unwrap_or("vendor/cold-clear-2/target/release/cold-clear-2");
            let opponent_config = arg("--opponent-config").map(Path::new);
            let strict_external_time = args.iter().any(|value| value == "--strict-external-time");
            let uses_external =
                first == arena::MatchAgent::ColdClear2 || second == arena::MatchAgent::ColdClear2;
            if strict_external_time && !uses_external {
                return Err("--strict-external-time requires cold_clear_2".into());
            }
            let no_swap = args.iter().any(|value| value == "--no-swap");
            let hoiko_config = arg("--hoiko-config")
                .map(HoikoConfig::load_or_default)
                .unwrap_or_else(config_from_home);
            let mut replay = arg("--replay").map(File::create).transpose()?;
            let mut details = Vec::with_capacity(games as usize);
            let mut outcomes = Vec::with_capacity(games as usize);
            let mut wins = 0;
            let mut losses = 0;
            let mut draws = 0;
            for game_index in 0..games {
                let pair_swap = !no_swap && game_index % 2 == 1;
                let match_seed =
                    seed + if no_swap { game_index as u64 } else { game_index as u64 / 2 };
                let (player0, player1) = if pair_swap {
                    (second, first)
                } else {
                    (first, second)
                };
                let result = battle::run_matchup(
                    match_seed,
                    pair_swap,
                    player0,
                    player1,
                    piece_limit,
                    budget,
                    rules,
                    weights.as_ref(),
                    &hoiko_config,
                    Path::new(opponent),
                    opponent_config,
                    strict_external_time,
                    replay.as_mut().map(|file| file as &mut dyn Write),
                )?;
                let first_won = result.winner == Some(usize::from(pair_swap));
                match result.winner {
                    None => draws += 1,
                    Some(_) if first_won => wins += 1,
                    Some(_) => losses += 1,
                }
                eprintln!(
                    "game {}/{games}: {wins}W {losses}L {draws}D, {} frames, {:?} pieces",
                    game_index + 1,
                    result.frames,
                    result.pieces
                );
                outcomes.push((result.seed, result.winner.map(|_| first_won)));
                details.push(json!({
                    "player0": player0.name(),
                    "player1": player1.name(),
                    "first_agent": first.name(),
                    "second_agent": second.name(),
                    "first_won": result.winner.map(|_| first_won),
                    "result": result,
                }));
            }
            let paired_bound = if no_swap {
                None
            } else {
                arena::paired_victory_bound_for(&outcomes).ok()
            };
            let report = json!({
                "command": "battle",
                "first_agent": first.name(),
                "second_agent": second.name(),
                "wins": wins,
                "losses": losses,
                "draws": draws,
                "decisive_wilson95_descriptive_only": arena::wilson(wins, wins + losses),
                "paired_victory_bound_for_first_agent": paired_bound,
                "ms": budget.milliseconds,
                "iterations": budget.iterations,
                "clock_hz": 60,
                "movement": "frame_inputs",
                "rules": rules.name(),
                "opponent": uses_external.then_some(opponent),
                "opponent_config": if uses_external { arg("--opponent-config") } else { None },
                "strict_external_time": strict_external_time,
                "side_swap": !no_swap,
                "matches": details,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
            if let Some(path) = arg("--output") {
                std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
            }
        }
        "match" | "matchup" | "versus" => {
            let first = arg_any(&["--p1", "--player1", "--first"])
                .ok_or("--p1 AGENT required (hoiko|intetrigence|cold_clear_2)")?;
            let second = arg_any(&["--p2", "--player2", "--second"])
                .ok_or("--p2 AGENT required (hoiko|intetrigence|cold_clear_2)")?;
            let first = arena::MatchAgent::parse(first)
                .ok_or_else(|| format!("unsupported --p1 agent: {first}"))?;
            let second = arena::MatchAgent::parse(second)
                .ok_or_else(|| format!("unsupported --p2 agent: {second}"))?;
            if first == second {
                return Err("--p1 and --p2 must name different agents".into());
            }
            let n = number("--games", 2)? as u32;
            let seed = number("--seed", 1)?;
            let limit = number("--turns", 1000)? as u32;
            if n == 0 || limit == 0 {
                return Err("games and turns must be positive".into());
            }
            let opponent = arg("--opponent")
                .unwrap_or("vendor/cold-clear-2/target/release/cold-clear-2");
            let opponent_config = arg("--opponent-config").map(Path::new);
            let strict_external_time = args.iter().any(|a| a == "--strict-external-time");
            if strict_external_time
                && first != arena::MatchAgent::ColdClear2
                && second != arena::MatchAgent::ColdClear2
            {
                return Err("--strict-external-time requires cold_clear_2".into());
            }
            let hoiko_config = arg("--hoiko-config")
                .map(HoikoConfig::load_or_default)
                .unwrap_or_else(config_from_home);
            let uses_external = first == arena::MatchAgent::ColdClear2
                || second == arena::MatchAgent::ColdClear2;
            let mut replay = arg("--replay").map(File::create).transpose()?;
            let mut details = Vec::with_capacity(n as usize);
            let mut outcomes = Vec::with_capacity(n as usize);
            let mut wins = 0;
            let mut losses = 0;
            let mut draws = 0;
            for i in 0..n {
                let pair_swap = i % 2 == 1;
                let (player0, player1) = if pair_swap {
                    (second, first)
                } else {
                    (first, second)
                };
                let result = arena::run_matchup(
                    seed + i as u64 / 2,
                    player0,
                    player1,
                    limit,
                    budget,
                    pps,
                    rules,
                    &hoiko_config,
                    Path::new(opponent),
                    opponent_config,
                    strict_external_time,
                    replay.as_mut().map(|file| file as &mut dyn Write),
                )?;
                let first_won = result.winner == Some(usize::from(pair_swap));
                match result.winner {
                    None => draws += 1,
                    Some(_) if first_won => wins += 1,
                    Some(_) => losses += 1,
                }
                eprintln!(
                    "game {}/{n}: {wins}W {losses}L {draws}D, {} turns",
                    i + 1,
                    result.turns
                );
                details.push(json!({
                    "player0": player0.name(),
                    "player1": player1.name(),
                    "first_agent": first.name(),
                    "second_agent": second.name(),
                    "first_won": if result.winner.is_some() { Some(first_won) } else { None::<bool> },
                    "result": result,
                }));
                outcomes.push((result.seed, result.winner.map(|_| first_won)));
            }
            let report = json!({
                "command": "match",
                "first_agent": first.name(),
                "second_agent": second.name(),
                "wins": wins,
                "losses": losses,
                "draws": draws,
                "decisive_wilson95_descriptive_only": arena::wilson(wins, wins + losses),
                "paired_victory_bound_for_first_agent": arena::paired_victory_bound_for(&outcomes).ok(),
                "ms": budget.milliseconds,
                "iterations": budget.iterations,
                "pps": pps,
                "rules": rules.name(),
                "opponent": uses_external.then_some(opponent),
                "opponent_config": if uses_external { arg("--opponent-config") } else { None },
                "strict_external_time": strict_external_time,
                "matches": details,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
            if let Some(path) = arg("--output") {
                std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
            }
        }
        "external-arena" => {
            let n = number("--games", 2)? as u32;
            let seed = number("--seed", 1)?;
            let limit = number("--turns", 1000)? as u32;
            if n == 0 || limit == 0 {
                return Err("games and turns must be positive".into());
            }
            let opponent = arg("--opponent")
                .unwrap_or("vendor/cold-clear-2/target/release/cold-clear-2")
                .to_string();
            let opponent_config = arg("--opponent-config").map(str::to_string);
            let strict_external_time = args.iter().any(|a| a == "--strict-external-time");
            let no_swap = args.iter().any(|a| a == "--no-swap");
            let mut replay = arg("--replay").map(File::create).transpose()?;
            let mut wins = 0;
            let mut losses = 0;
            let mut draws = 0;
            let mut matches = Vec::new();
            for i in 0..n {
                let match_seed = seed + if no_swap { i as u64 } else { i as u64 / 2 };
                let swapped = !no_swap && i % 2 == 1;
                let result = if strict_external_time {
                    arena::run_external_strict_with_rules(
                        match_seed,
                        swapped,
                        limit,
                        budget,
                        weights.as_ref(),
                        pps,
                        rules,
                        Path::new(&opponent),
                        opponent_config.as_deref().map(Path::new),
                        replay.as_mut().map(|f| f as &mut dyn Write),
                    )?
                } else {
                    arena::run_external_with_rules(
                        match_seed,
                        swapped,
                        limit,
                        budget,
                        weights.as_ref(),
                        pps,
                        rules,
                        Path::new(&opponent),
                        opponent_config.as_deref().map(Path::new),
                        replay.as_mut().map(|f| f as &mut dyn Write),
                    )?
                };
                match result.winner {
                    None => draws += 1,
                    Some(p) if (p == 0) != result.swapped => wins += 1,
                    _ => losses += 1,
                }
                eprintln!(
                    "game {}/{n}: {wins}W {losses}L {draws}D, {} turns",
                    i + 1,
                    result.turns
                );
                matches.push(result);
            }
            let report = json!({
                "wins": wins,
                "losses": losses,
                "draws": draws,
                "decisive_wilson95_descriptive_only": arena::wilson(wins, wins + losses),
                "paired_victory_bound": arena::paired_victory_bound(&matches).ok(),
                "ms": budget.milliseconds,
                "iterations": budget.iterations,
                "pps": pps,
                "opponent": opponent,
                "opponent_config": opponent_config,
                "side_swap": !no_swap,
                "matches": matches,
                "note": if strict_external_time { "strict external TBP window; child is suspended during the local search and gets the same wall-clock window" } else { "external TBP opponent; tree is retained through play/new_piece and restarted only after a garbage rise changes the board" },
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
            if let Some(path) = arg("--output") {
                std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
            }
        }
        "verify-replay" => {
            let path = arg("--input").ok_or("--input FILE required")?;
            let count = intetrigence::replay::verify(io::BufReader::new(File::open(path)?))?;
            println!("verified {count} complete matches");
        }
        "selfplay" => {
            let mut g=Game::new_with_rules(number("--seed",1)?, rules);
            for _ in 0..number("--pieces",100)? { let (mv,_) = choose(&g,budget,true,weights.as_ref()); let Some(mv)=mv else {break}; g.play(mv)?; if g.dead {break;} }
            println!("{}",json!({"pieces":g.pieces,"lines":g.lines,"attack":g.attack,"score":g.score,"dead":g.dead,"board":g.board_json()}));
        }
        "hoiko-selfplay" => {
            let config = arg("--hoiko-config")
                .map(HoikoConfig::load_or_default)
                .unwrap_or_else(config_from_home);
            let searcher = HoikoSearcher::new(config);
            let mut g = Game::new_with_rules(number("--seed", 1)?, rules);
            let mut searches = Vec::new();
            for _ in 0..number("--pieces", 100)? {
                let (mv, stats) = searcher.choose(&g, budget);
                searches.push(stats);
                let Some(mv) = mv else { break };
                g.play(mv)?;
                if g.dead { break; }
            }
            println!("{}", json!({"agent":"hoiko","pieces":g.pieces,"lines":g.lines,"attack":g.attack,"score":g.score,"dead":g.dead,"searches":searches,"board":g.board_json()}));
        }
        "tbp" => protocol(budget, weights.as_ref(), false, None)?,
        "hoiko-tbp" => {
            let config = arg("--hoiko-config").map(HoikoConfig::load_or_default);
            protocol(budget, weights.as_ref(), true, config)?;
        }
        #[cfg(feature = "battle-bot")]
        "battle-bot" | "battle_tet" => {
            let url = battle_bot_url(
                arg("--url"),
                arg("--match-id"),
                arg("--token"),
                arg("--role").unwrap_or("ai"),
            )?;
            battle_ws::run(&url, budget, weights.as_ref())?;
        }
        #[cfg(not(feature = "battle-bot"))]
        "battle-bot" | "battle_tet" => {
            return Err(
                "battle-bot is optional; rebuild with --features battle-bot (or battle-wss for wss://)"
                    .into(),
            );
        }
        _ => println!("intetrigence — Rust versus Tetris AI\n  arena --games 20 --seed 1 --ms 20 --pps 40 --turns 1000 --output results/run.json --replay results/run.jsonl\n  match --p1 hoiko --p2 intetrigence --games 2 --ms 0 --iterations 200 --output results/hoiko-vs-intetrigence.json --replay results/hoiko-vs-intetrigence.jsonl\n  battle --p1 hoiko --p2 intetrigence --games 2 --ms 0 --iterations 200 --turns 1000 --replay results/battle.jsonl\n  match --p1 hoiko --p2 cold_clear_2 --opponent vendor/cold-clear-2/target/release/cold-clear-2 --games 2 --ms 20 --pps 40\n  hoiko-arena --games 2 --seed 1 --ms 20 --pps 40 --turns 1000 [--hoiko-config DIR]\n  external-arena --opponent vendor/cold-clear-2/target/release/cold-clear-2 --games 2 --ms 20 --pps 40\n  external-arena --no-swap --opponent vendor/cold-clear-2/target/release/cold-clear-2 --games 100 --ms 20 --pps 40\n  external-arena --strict-external-time --opponent vendor/cold-clear-2/target/release/cold-clear-2 --games 2 --ms 20 --pps 40\n  selfplay --pieces 100 --ms 20\n  hoiko-selfplay --pieces 100 --ms 20 [--hoiko-config DIR]\n  verify-replay --input results/run.jsonl\n  tbp --ms 20\n  hoiko-tbp --ms 20 [--hoiko-config DIR]\n  battle-bot --url ws://localhost:3000/ws --match-id <match-id> --token <ai-token> --role ai --ms 400\nOptional: --iterations N (fixed expansion attempts instead of time), --weights FILE, --pps N (turn arena only), --rules arena|guideline, --no-swap, --strict-external-time (Unix external process time slicing), battle-bot uses --ms as the per-request search cap"),
    }
    Ok(())
}

#[cfg(feature = "battle-bot")]
fn battle_bot_url(
    base: Option<&str>,
    match_id: Option<&str>,
    token: Option<&str>,
    role: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let base = base.ok_or(
        "battle-bot requires --url ws://host/path, or a full URL with matchId, role, and token",
    )?;
    match (match_id, token) {
        (None, None) => Ok(base.to_owned()),
        (Some(match_id), Some(token)) => {
            let separator = if base.contains('?') { '&' } else { '?' };
            Ok(format!(
                "{base}{separator}matchId={}&role={}&token={}",
                url_encode(match_id),
                url_encode(role),
                url_encode(token)
            ))
        }
        _ => Err("--match-id and --token must be supplied together".into()),
    }
}

#[cfg(feature = "battle-bot")]
fn url_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

#[cfg(all(test, feature = "battle-bot"))]
mod battle_bot_args_tests {
    use super::battle_bot_url;

    #[test]
    fn accepts_server_style_match_arguments() {
        assert_eq!(
            battle_bot_url(
                Some("ws://localhost:3000/ws"),
                Some("example-match"),
                Some("example-token"),
                "ai",
            )
            .unwrap(),
            "ws://localhost:3000/ws?matchId=example-match&role=ai&token=example-token"
        );
    }
}

fn protocol(
    budget: Budget,
    weights: Option<&Value>,
    hoiko: bool,
    hoiko_config: Option<HoikoConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let emit = |v: Value| -> io::Result<()> {
        let mut out = io::stdout().lock();
        writeln!(out, "{v}")?;
        out.flush()
    };
    emit(
        json!({"type":"info","name":"Intetrigence","version":env!("CARGO_PKG_VERSION"),"author":"Intetrigence contributors; based on MinusKelvin Cold Clear 2","features":[]}),
    )?;
    let mut game: Option<Game> = None;
    let hoiko_searcher = if hoiko {
        Some(HoikoSearcher::new(
            hoiko_config.unwrap_or_else(config_from_home),
        ))
    } else {
        None
    };
    let mut rules_randomizer = "unknown".to_string();
    for line in io::stdin().lock().lines() {
        let msg: Value = serde_json::from_str(&line?)?;
        match msg["type"].as_str().unwrap_or("") {
            "rules" => {
                rules_randomizer = msg["randomizer"].as_str().unwrap_or("unknown").to_string();
                if !matches!(rules_randomizer.as_str(), "unknown" | "seven_bag") {
                    emit(json!({"type":"error","reason":"unsupported_rules"}))?;
                } else {
                    emit(json!({"type":"ready"}))?;
                }
            }
            "start" => {
                let randomizer_type = msg["randomizer"]["type"]
                    .as_str()
                    .unwrap_or(rules_randomizer.as_str());
                if !matches!(randomizer_type, "unknown" | "seven_bag") {
                    emit(json!({"type":"error","reason":"unsupported_rules"}))?;
                    continue;
                }
                if rules_randomizer == "seven_bag"
                    && msg["randomizer"]["type"].as_str() != Some("seven_bag")
                {
                    emit(json!({"type":"error","reason":"unsupported_rules"}))?;
                    continue;
                }
                let rows = msg["board"]
                    .as_array()
                    .ok_or("board must be a 40x10 array")?;
                if rows.len() != 40
                    || rows
                        .iter()
                        .any(|r| r.as_array().is_none_or(|r| r.len() != 10))
                {
                    return Err("board must be 40x10".into());
                }
                let mut g = Game::new(0);
                g.queue = serde_json::from_value(msg["queue"].clone())?;
                g.hold = serde_json::from_value(msg["hold"].clone())?;
                g.speculate = randomizer_type == "seven_bag";
                g.b2b = msg["back_to_back"]
                    .as_bool()
                    .ok_or("back_to_back required")?;
                g.combo = msg["combo"].as_u64().ok_or("combo required")?.try_into()?;
                for (y, row) in rows.iter().enumerate() {
                    for (x, cell) in row.as_array().unwrap().iter().enumerate() {
                        if !cell.is_null() {
                            g.board.cols[x] |= 1 << y;
                        }
                    }
                }
                g.bag = if msg["randomizer"]["type"] == "seven_bag" {
                    serde_json::from_value(msg["randomizer"]["bag_state"].clone())?
                } else {
                    crate_pieces()
                };
                game = Some(g);
            }
            "suggest" => {
                if let Some(g) = game.as_ref() {
                    if !g.queue.is_empty() {
                        let (mv, stats) = if let Some(searcher) = hoiko_searcher.as_ref() {
                            let (mv, hs) = searcher.choose(g, budget);
                            let stats = intetrigence::search::SearchStats {
                                nodes: hs.nodes,
                                iterations: hs.depth as u64,
                                elapsed_us: hs.elapsed_us,
                                reused_tree: false,
                            };
                            (mv, stats)
                        } else {
                            choose(g, budget, true, weights)
                        };
                        emit(
                            json!({"type":"suggestion","moves":mv.into_iter().collect::<Vec<_>>(),"move_info":stats}),
                        )?;
                    } else {
                        emit(json!({"type":"suggestion","moves":[]}))?;
                    }
                }
            }
            "play" => {
                if let Some(g) = game.as_mut() {
                    let mv: intetrigence_engine::data::Placement =
                        serde_json::from_value(msg["move"].clone())?;
                    // The frontend owns piece generation; remove simulator-generated previews.
                    let before = g.queue.len();
                    let current = g.queue.front().copied();
                    let extra = g.hold.is_none() && current != Some(mv.location.piece);
                    let bag = g.bag.clone();
                    g.play(mv)?;
                    g.bag = bag;
                    g.queue.truncate(before - 1 - usize::from(extra));
                }
            }
            "new_piece" => {
                if let Some(g) = game.as_mut() {
                    let p = serde_json::from_value(msg["piece"].clone())?;
                    g.queue.push_back(p);
                    if g.bag.is_empty() {
                        g.bag = crate_pieces();
                    }
                    g.bag.retain(|&v| v != p);
                }
            }
            "stop" => {
                game = None;
                rules_randomizer = "unknown".to_string();
            }
            "quit" => break,
            _ => {}
        }
    }
    Ok(())
}
fn crate_pieces() -> Vec<intetrigence_engine::data::Piece> {
    intetrigence::game::PIECES.to_vec()
}
