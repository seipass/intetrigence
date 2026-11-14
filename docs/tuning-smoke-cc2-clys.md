# CC2 / clys tuning smoke (diagnostic)

> **All numbers below are the 1-ply greedy bot.** bot-dev found that the
> anytime loop discarded its iteration on a deadline miss and the caller fell
> back to `greedy` (1-ply); every setting (depth 8/12/16, width 2048, 1 thread)
> produced the same 1-ply behaviour. These figures are **not** the bot's real
> strength and must be re-measured after the search fix. They are recorded only
> as the pre-fix baseline.

> **Tuning smoke, not an evaluation round.** Seeds 9001–9010 (sheet A, 1000
> turns) are tuning seeds; the evaluation seeds are untouched. The purpose is
> to check the `--tbp-*` path, forfeit 0, and the current baseline.

## Setup

- intetrigence: `44d4815` (gameplay: `fa7905b` bot), `--bot search --ms 100 --threads 8`
- CC2: `ed8b19327b6bd1410ddd873d8611485bd45d8fae` + `adapters/cc2-tbp-compat.patch`
- clys: `ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc` + `adapters/clys-tbp-spawn.patch`
  + `adapters/clys/tbp/adapter.ts`
- connection: `arena match --tbp-program ... --tbp-cwd ... --tbp-visible-rows 40`
  (no shim; the `--tbp-*` flags are in the main source)
- evidence: `scripts/evidence/tuning-cc2/`, `scripts/evidence/tuning-clys/`
  (replay + audit + TBP transcript per game)

## Result

| opponent | games | wins_topout | losses_topout | unresolved | no_contest | stuck | verified | measurement_complete |
|---|---|---|---|---|---|---|---|---|
| Cold Clear 2 | 10 | **10** | 0 | 0 | 0 | 0 | 10/10 | **true** |
| clys | 10 | **2** | 8 | 0 | 0 | 0 | 10/10 | **true** |

- **forfeit 0 in both**; no turn-cap, no stuck.
- CC2: 10/10 = 100% in this sample (Wilson 95% CI 72.2–100.0%). Turns 54–446.
- clys: 2/10 = 20% (Wilson 95% CI 5.7–51.0%). Wins 9001-A (108t), 9006-A (44t);
  the other 8 are TopOut losses (69–210t).
- attack/piece: CC2 0.80–0.99 vs 0.59–0.87; clys 0.66–0.91 vs 0.94–1.15.
- think median: intetrigence 96.7–97.9 ms; CC2 ~100.1 ms; clys 108.6–114.2 ms.
- spin mismatches: intetrigence 0; CC2 0–9; clys 40–193 (both are not S2-native;
  expected and recorded as audit only).

## measurement_complete

The `--tbp-*` path writes `logs/<game>-opponent.jsonl` (every host message and
suggestion). `run_match` treats that TBP transcript as the opponent decision
log, so `measurement_complete` is **true** for CC2 and clys. The earlier
transparent-shim run did not write the transcript and produced
`measurement_complete = false`; that was a shim artifact, not a protocol gap.

## Interpretation

- The CC2 stage (50-0 required) looks reachable from this sample, but 10 games
  is not 50; the official round must still be run on the evaluation seeds.
- **clys is the hard stage**: 2/10. bot-dev found that the anytime loop was
  falling back to 1-ply greedy (`search` returns `None` on a deadline miss and
  the caller uses `greedy`). That explains the gap to clys (depth 32) and to
  attax. The search fix is in progress; this smoke is the pre-fix baseline.
- This is a diagnostic; no claim is made from tuning seeds.

## Search profile (post-fix format)

After the search fix, the same smoke is reported with the search instrument
(protocol v1.31 §8.4.1). `PROFILE=1 bash scripts/tuning-smoke.sh` prints, and
the round report records, this shape:

| game | turns | depth med / p95 / max | width med / p95 / max | fallback |
|---|---|---|---|---|
| `<seed>-<sheet>` | `<n>` | `<a> / <b> / <c>` | `<a> / <b> / <c>` | `<n>/<n>` |

- `depth` is the completed depth of the winning anytime iteration (not the
  configured maximum); `fallback` means no iteration completed (1-ply greedy).
- `nodes` is expanded beam nodes, not leaves (≈34 placements per node).
- Depth by turn bucket (50 turns) is printed per game, so a late-game depth
  collapse is visible.
- Every table is recorded with the build commit, the weights and the bot's
  search configuration; numbers from different builds are not compared.

## Reproduction

```
bash scripts/tuning-smoke.sh
```
