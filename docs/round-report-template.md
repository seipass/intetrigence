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

| stage | opponent | gate (TopOut wins, no turn-cap) | games | W-L / U / NC | result |
|---|---|---|---|---|---|
| 1 | Cold Clear 2 | 50 TopOut wins, 0 losses, 0 turn-cap, 0 no-contest | 50 | 50-0 / 0 / 0 | PASSED |
| 2 | clys | 50 TopOut wins, 0 losses, 0 turn-cap, 0 no-contest | 50 | 50-0 / 0 / 0 | PASSED |
| 3 | attax | 48 TopOut wins, 0 turn-cap | 50 | 48-2 / 0 / 0 | PASSED |

- A failed gate stops the round; the remaining stages are not run.
- A `forfeit-other` is an adapter failure recorded as no-contest; stages 1-2 fail on any no-contest (perfect record).
- A retry needs a new build and a new pre-committed seed set; partial reruns are not allowed.

## Stage 1 — Cold Clear 2

- opponent commit: `<opponent commit>`
- games: 50 (25 seeds × sheets A/B)
- opponent vision: `40` rows (CC2/clys: S2 scoring applied by the referee; attax: 40-row patch)
- raw games: 50 W / 0 L; unresolved (TurnCap): 0; no-contest (forfeit): 0; stuck: 0
- **gate record: 50 TopOut wins / 50 games** — 100.0% (Wilson 95% CI 92.9..100.0%)
- win breakdown: TopOut `<n>`; unresolved (TurnCap) `<n>`; no-contest (forfeit-vision `<n>` + forfeit-other `<n>`) `<n>`; stuck `<n>`
- no-contest record: `<wins_topout>/<games - no_contest>` (TopOut games)
- attack/piece: `<a>` vs `<b>`
- received garbage/game: `<a>` vs `<b>`
- think median / p95 / p99 / max: `<a> / <b> / <c> / <d> ms` vs `<a> / <b> / <c> / <d> ms`
- think over 200 ms: `<n>` vs `<n>`
- net damage/game: `<+a>` (negative games `<n>/<n>`); collapses `<n>/<n>`; opener lines/game `<a>`
- burst (intetrigence received): max 1-turn `<n>`, max 5-turn `<n>`, over capacity `<n>` (overlapping 5-turn windows, not burst events)
- environment: `<KEY>=<absolute path>` (e.g. `CLYS_TBP_LOG=...` for clys)
- replay verified: **50/50**
- audit: spin mismatches `<n>` / `<n>`, same-spin attack mismatches `<n>` / `<n>`
- per sheet: A `<w>/25`, B `<w>/25`

## Stage 2 — clys

(Same fields as stage 1. clys is not S2-native; the referee applies S2 scoring
to both sides and records clys's declared values as audit only — protocol
v1.13 §7.2. This stage is a generalization check, not an S2-vs-S2 claim.)

## Stage 3 — ATTAX

(Same fields as stage 1. attax is the final stage: 48+ TopOut wins, no
turn-cap; no-contest is allowed only if the 50-game total still holds.)

## Interruption and resume (protocol v1.18 §1.4)

- Verified replays from an interrupted round are kept.
- Resume is allowed only with the **same build and the same seeds**: completed
  `(seed, sheet)` games are reused after verification and the report records
  the resume point, the reused game count and the interruption reason.
- A build or seed change starts a new round; partial results are not used for
  claims.

## Compatibility patches (protocol v1.14 §7.3)

| opponent | upstream commit | patch | purpose | direction |
|---|---|---|---|---|
| CC2 | `ed8b193...` | `adapters/cc2-tbp-compat.patch` (116 lines) | empty-hold representation + spawn row 21 | removes a handicap (opponent stronger) |
| clys | `ccae45f...` | `adapters/clys-tbp-spawn.patch` (38 lines) | spawn row 21 | removes a handicap (opponent stronger) |

- The patches only align TBP/S2 protocol details (spawn position, empty hold). They do **not** touch the opponents' search, evaluation, or move legality.
- Without the patches, CC2 would misread the current piece as held (hold disabled) and both bots would return no moves at a tall stack (vision-style forfeits). The patches remove those handicaps; the matches are therefore real TopOut games, not weakened-opponent wins.

## Claims and caveats

- Only the `official` profile is used; diagnostic profiles are never part of a round.
- Spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences.
- This report does not claim a win rate against a full-vision opponent: no-contest games are reported separately and the TopOut-only record is stated per stage.
- CC2 and clys are not S2-native. The referee applies S2 scoring to both sides and records the bots' declared values as audit only (protocol v1.13 §7.2); their stages are a generalization check, not an S2-vs-S2 claim.
- A `forfeit-other` is an adapter failure recorded as no-contest (protocol v1.13 §7.1.3-4); stages 1-2 fail on any no-contest.
- Stage 3 (attax, 40-row patched) and stages 1-2 (CC2/clys, 40-row vision) are real TopOut battles. The stage win rates are still not directly comparable across opponents; the TopOut-only record per stage is the honest strength signal.

## Reproduction

- `scripts/run-round.sh` with `ROUND=N` and `SEEDS_FILE=docs/seeds-round-N.md`
- `arena verify scripts/evidence/round-N-<opponent>`
- `python scripts/analyze-m2.py` per stage (optional per-match report)

## Acceptance checklist (protocol v1.13 §8.5)

- [ ] profile is `official`
- [ ] seed range and held-out basis (unused by both projects)
- [ ] game count and sheet assignment
- [ ] wins_topout / losses_topout / unresolved / no-contest and Wilson 95% CI
- [ ] win breakdown: TopOut / unresolved (TurnCap) / no-contest (forfeit) / stuck
- [ ] TopOut-only record stated separately
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
