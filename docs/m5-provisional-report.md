# M5 provisional match (100 games, seeds 11001-11050, current guideline attax)

- evidence: `scripts/evidence/m5-provisional`
- profile: **official** (empty start board; the only profile usable for final claims)
- games: 100 (provisional seeds 11001-11050 x sheets A/B)
- seeds 11001-11050 are a fresh range unused by either project; the reserved final seeds 20001-20050 were **not** used
- held-out basis: attax: tuning 2001-2060 (scripts/tune.ts), held-out benchmark 1001-1200, prior matches 5001-5050 and 7001-7050; intetrigence: M0 1001-1005, M2 9001-9010, M4 10001-10020. Seeds 11001-11050 appear in neither project's evidence.
- intetrigence: real search (`--ms 100 --threads 8`), commit `aca615c`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`, bun `1.4.0`
- rules: `docs/s2-spec.md@v1.3.11+aca615c` / `docs/s2-referee-protocol.md@v1.9+aca615c`
- environment: Windows-11-10.0.26200-SP0, 16 logical CPUs
- commands: intetrigence `intetrigence --ms 100 --threads 8`; attax `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Result

- wins / losses / draws: **94 / 6 / 0**
- forfeits: attax **91** (20-row view), intetrigence **0**
- win rate: **94.0%** (Wilson 95% CI 87.5%..97.2%)
- win composition: TopOut 0, forfeit-vision 91, forfeit-other 0; losses: TopOut 6, forfeit 0; turn-cap games 3
- **non-forfeit games (40-row TopOut / TurnCap, n=9)**: 3 wins / 6 losses = **33.3%** (Wilson 95% CI 12.1%..64.6%)
- replay verified: **100/100**
- per sheet: A 46/50, B 48/50
- seat note: losses by sheet ['A', 'B']; the per-sheet split is above. The sample is too small to separate a seat effect from the opponent's forfeits.

## Claims

- This report does **not** claim a win rate against a full-vision opponent: all vision forfeit wins are reported separately and the non-forfeit (40-row) record is stated above.
- The spin rule is more lenient than TETR.IO (see Caveats); intetrigence's labels are backed by rotation-ending input sequences.

## Think time

- intetrigence: median 97.0 ms, p95 98.8 ms, p99 99.8 ms, max 297.0 ms (median/p95/p99 are means of per-game values; max is the match worst)
- attax: median 31.0 ms, p95 39.7 ms, p99 46.3 ms, max 421.7 ms
- placements over the 200 ms budget: intetrigence 1, attax 2
- first-choice acceptance (mean over games): intetrigence 1.00, attax 1.00
- **CPU contention caveat**: this 100-game run overlapped with bot-dev's
  8-thread A/B experiment on the same 8-core machine. In `11008-A` **both**
  bots' think times spiked in the same game (intetrigence 297.0 ms, attax
  274.1 ms), which is a machine-wide event rather than a bot-side regression.
  The think-time numbers and search quality here are therefore a **lower bound
  under contention**; an idle re-run is needed before using them as a final
  basis. The over-budget placements are `11008-A` (both bots) and `11033-B`
  (attax only, 421.7 ms).

## Attack and garbage

- attack/piece: intetrigence **0.965**, attax **0.862**
- received garbage per game: intetrigence **85.0**, attax **117.1**
- intetrigence search: 29,114,604 nodes total, 6 fallback placements
- `start` resends (attax): 4286; `cellsAboveRow20`: True

## Board height and survival

- mean turns 320.8; turn-cap games 3; TopOut games 6
- slot 0: max height 17.1, mean height 5.7, final 5.8, lines cleared/game 200.6
- slot 1: max height 20.6, mean height 5.9, final 19.2, lines cleared/game 217.2

## Scoring audit

- intetrigence: spin mismatches 0 / 32029, same-spin attack mismatches 0 / 32029
- attax: spin mismatches 430 / 31981, same-spin attack mismatches 0 / 31981

## Loss classification (coarse)

- downstack (cleared but could not keep up with garbage): 5
- opener (died within 40 turns): 1

| seed | sheet | reason | turns | intetrigence recv | attax recv | intetrigence attack/piece | classification |
|---|---|---|---|---|---|---|---|
| 11004 | A | TopOut | 194 | 81 | 76 | 0.907 | downstack (cleared but could not keep up with garbage) |
| 11011 | A | TopOut | 306 | 93 | 99 | 0.961 | downstack (cleared but could not keep up with garbage) |
| 11020 | B | TopOut | 245 | 73 | 65 | 0.886 | downstack (cleared but could not keep up with garbage) |
| 11025 | A | TopOut | 149 | 33 | 48 | 0.826 | downstack (cleared but could not keep up with garbage) |
| 11027 | A | TopOut | 37 | 24 | 13 | 0.595 | opener (died within 40 turns) |
| 11030 | B | TopOut | 261 | 99 | 90 | 0.969 | downstack (cleared but could not keep up with garbage) |

## Non-forfeit analysis

- The 3 non-forfeit wins are **turn-cap wins** (both sides survived 1000
  turns; the referee ranks by received garbage). intetrigence won **no
  non-forfeit game by TopOut**.
- All 6 losses are TopOuts: 5 downstack + 1 opener. In 5 of the 6 the bot's
  attack/piece was 0.886-0.969 (it kept attacking) but it finished at a high
  stack; the opener loss (11027-A) died at turn 37 with attack/piece 0.595.
- **The non-forfeit record is 3/9 = 33.3% (Wilson 12.1..64.6)**. This is
  lower than M4 reserve (5/8 = 62.5%) and is the honest strength signal:
  the 94.0% headline is inflated by 91 vision forfeits.

## Comparison with M4 reserve (same protocol, different seeds)

| metric | M4 reserve (pre-v2) | M4 reserve v2 | M5 provisional |
|---|---|---|---|
| games | 40 | 40 | 100 |
| wins / losses | 38 / 2 | 37 / 3 | **94 / 6** |
| win rate | 95.0% (83.5..98.6) | 92.5% (80.1..97.4) | **94.0% (87.5..97.2)** |
| forfeit-vision / turn-cap | 34 / 4 | 32 / 5 | 91 / 3 |
| TopOut losses | 2 | 3 | 6 |
| non-forfeit record | 4/6 (66.7%) | 5/8 (62.5%) | **3/9 (33.3%)** |
| attack/piece | 0.977 | 0.966 | 0.965 |
| received garbage/game | 113.05 | 97.0 | 85.0 |
| think median / p95 | 97.1 / 98.7 ms | 97.0 / 98.7 ms | 97.0 / 98.8 ms |
| mean / max height | 5.71 / 17.18 | 5.81 / 17.98 | 5.71 / 17.08 |
| spin mismatches | 0 / 16486 | 0 / 14407 | 0 / 32029 |
| same-spin attack mismatches | 0 / 16486 | 0 / 14407 | 0 / 32029 |

- Attack/piece and received garbage are consistent with M4; the **non-forfeit
  record is the outlier** (3/9 vs 4/6 and 5/8). The sample is small (n=9), so
  the Wilson interval is wide, but the direction matches the known weakness:
  the bot out-attacks and still tops out.
- Think budget: **1 intetrigence placement exceeded 200 ms (max 297.0 ms)**;
  attax had 2 (max 421.7 ms). This is reported per protocol §8.4; it did not
  cause a forfeit.

## Caveats

- attax at this commit is guideline-scored and has a 20-row vision limit; the S2 version is not released upstream (7b38f51 vs upstream 57a5a6a: docs/page-generation only, no scoring/search/vision changes). This match is a **reference value**; the official 90% claim will be made against the attax S2 version with the reserved seeds 20001-20050.
- spin classification follows protocol v1.7 §4.2.1 (max over paths), which is more lenient than TETR.IO; intetrigence's labels are backed by rotation-ending input sequences, so the leniency only helps a cell-only client.
- seeds 11001-11050 are a fresh range unused by either project; the reserved final seeds 20001-20050 were **not** used.
- the run overlapped with bot-dev's 8-thread A/B experiment (same 8-core
  machine). The 200 ms spike in `11008-A` hit **both** bots at once, so it is
  a machine-wide contention event; the strength numbers are a lower bound.

## Reproduction

- `scripts/m5-provisional.sh`
- `arena verify scripts/evidence/m5-provisional`
- `python scripts/analyze-m2.py`
