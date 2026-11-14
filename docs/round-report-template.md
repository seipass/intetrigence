# Round N — staged round report (template)

> This file documents the structure the round runner writes to
> `docs/round-N-report.md`. The runner fills every section from the three
> `summary.json` files; the checklist at the bottom is the acceptance list from
> `docs/s2-referee-protocol.md` v1.13 §8.5.

## Header

- profile: **official** (the only profile usable for claims; protocol v1.11 §1.3)
- build: `<intetrigence commit>` (same commit for all three stages)
- seeds: `docs/seeds-round-N.md` at commit `<seeds commit>` (25 values, range `30001-30025`)
- seeds registry: `docs/seed-registry.md` (marked evaluation before the run, burned after)
- started: `<timestamp>`
- environment: `<OS>` / `<logical CPUs>` / bun `<version>`

## Gates

| stage | opponent | gate | games | W-L-D | result |
|---|---|---|---|---|---|
| 1 | attax | 50 wins | 50 | 50-0-0 | PASSED |
| 2 | Cold Clear 2 | 48 wins | 50 | 48-2-0 | PASSED |
| 3 | clys | 48 wins | 50 | 48-2-0 | PASSED |

- A failed gate stops the round; the remaining stages are not run.
- A `forfeit-other` win is an invalid game and fails the stage (protocol v1.13 §7.1.3-4).
- A retry needs a new build and a new pre-committed seed set; partial reruns are not allowed.

## Stage 1 — ATTAX

- opponent commit: `<attax commit>`
- games: 50 (25 seeds × sheets A/B)
- result: **50 W / 0 L / 0 D** — 100.0% (Wilson 95% CI 92.9..100.0%)
- win breakdown: TopOut `<n>`, forfeit-vision `<n>`, forfeit-other `<n>`, turn-cap `<n>`
- non-forfeit record: `<w>/<n>` (TopOut + turn-cap games)
- attack/piece: `<a>` vs `<b>`
- received garbage/game: `<a>` vs `<b>`
- think median / p95 / p99 / max: `<a> / <b> / <c> / <d> ms` vs `<a> / <b> / <c> / <d> ms`
- think over 200 ms: `<n>` vs `<n>`
- replay verified: **50/50**
- audit: spin mismatches `<n>` / `<n>`, same-spin attack mismatches `<n>` / `<n>`
- per sheet: A `<w>/25`, B `<w>/25`

## Stage 2 — Cold Clear 2

(Same fields as stage 1. CC2 is not S2-native; the referee applies S2 scoring
to both sides and records CC2's declared values as audit only — protocol
v1.13 §7.2. This stage is a generalization check, not an S2-vs-S2 claim.)

## Stage 3 — clys

(Same fields as stage 1. clys is not S2-native; same interpretation as CC2.)

## Claims and caveats

- Only the `official` profile is used; diagnostic profiles are never part of a round.
- Spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences.
- This report does not claim a win rate against a full-vision opponent: forfeit wins are reported separately and the non-forfeit record is stated per stage.
- CC2 and clys are not S2-native. The referee applies S2 scoring to both sides and records the bots' declared values as audit only (protocol v1.13 §7.2); their stages are a generalization check, not an S2-vs-S2 claim.
- A `forfeit-other` win is an invalid game (protocol v1.13 §7.1.3-4) and fails the stage.

## Reproduction

- `scripts/run-round.sh` with `ROUND=N` and `SEEDS_FILE=docs/seeds-round-N.md`
- `arena verify scripts/evidence/round-N-<opponent>`
- `python scripts/analyze-m2.py` per stage (optional per-match report)

## Acceptance checklist (protocol v1.13 §8.5)

- [ ] profile is `official`
- [ ] seed range and held-out basis (unused by both projects)
- [ ] game count and sheet assignment
- [ ] wins/losses and Wilson 95% CI
- [ ] win breakdown: TopOut / forfeit-vision / forfeit-other / turn-cap
- [ ] non-forfeit record stated separately
- [ ] no claim of a win rate against a full-vision opponent
- [ ] both sides' think time (median / p95 / p99 / max) and 200 ms overruns
- [ ] both sides' attack/piece and received garbage/game
- [ ] scoring audit per bot: spin mismatches / same-spin attack mismatches
- [ ] replay verification: all games verified
- [ ] both sides' commits, launch commands, environment (CPU, threads, Bun)
- [ ] spin leniency vs TETR.IO stated
- [ ] staged gates and the stop point recorded
- [ ] CC2/clys S2-non-native interpretation stated
- [ ] reproduction commands
