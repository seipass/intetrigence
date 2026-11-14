# attax 40-row patch smoke (diagnostic)

> **This is a diagnostic smoke, not an evaluation round.** The seeds are
> tuning-range seeds (11001–11005), the sample is 10 games, and the numbers
> are not used for any claim. The purpose is to check that the 40-row attax
> patch removes the adapter forfeits and to measure the real TopOut baseline.

## Setup

- intetrigence: `12196ec` (gameplay binary), `--bot search --ms 100 --threads 8`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b` + `adapters/attax-vision-40.patch`
  (68 lines, SHA-256 `e22aab7b5a2f9a95efb562e24b3f33da351bb942d6daf6e205475b5871ed3b70`)
- visible rows: 40 both sides (auto-detected from attax `movegen.ts`)
- seeds: 11001–11005 × sheets A/B = 10 games, 1000 turn cap, official profile
- evidence: `scripts/evidence/attax-40-smoke/` (replay + audit + attax decision log per game)
- classification: `summary-reclassified.json` (protocol v1.27/v1.28 rules:
  `TurnCap` and `Forfeit` have `winner = None`; `Stuck` is separate)

## Result

- games 10, replay verified **10/10**, measurement complete 10/10
- **wins_topout 0 / losses_topout 9 / unresolved (TurnCap) 1 / no_contest 0 / stuck 0**
- **forfeit 0** — the 40-row patch removed the adapter forfeits

| seed | sheet | result | turns | attax max height | intetrigence max height | attack/piece (int / attax) | received garbage (int / attax) |
|---|---|---|---|---|---|---|---|
| 11001 | A | TopOut loss | 92 | 14 | 22 | 0.652 / 0.857 | 40 / 25 |
| 11001 | B | TopOut loss | 126 | 12 | 21 | 0.587 / 0.730 | 39 / 24 |
| 11002 | A | TopOut loss | 600 | 17 | 23 | 0.810 / 0.830 | 176 / 168 |
| 11002 | B | TopOut loss | 72 | 12 | 21 | 0.778 / 0.861 | 26 / 23 |
| 11003 | A | TopOut loss | 576 | 18 | 22 | 0.809 / 0.838 | 170 / 156 |
| 11003 | B | TopOut loss | 115 | 10 | 20 | 0.504 / 0.713 | 34 / 12 |
| 11004 | A | TopOut loss | 572 | 19 | 21 | 0.895 / 0.907 | 179 / 175 |
| 11004 | B | TurnCap (unresolved) | 1000 | 21 | 21 | 0.981 / 0.955 | 296 / 324 |
| 11005 | A | TopOut loss | 562 | 15 | 20 | 0.891 / 0.861 | 162 / 181 |
| 11005 | B | TopOut loss | 160 | 16 | 21 | 0.756 / 0.875 | 64 / 45 |

Aggregates (mean over 10 games):

- attack/piece: intetrigence **0.766** / attax **0.843**
- received garbage: intetrigence **118.6** / attax **113.3**
- think median: intetrigence **97.1 ms** / attax **30.6 ms**
- think over 200 ms: 0 / 0
- spin mismatches: intetrigence **0** / attax **67** (attax is not S2-native; expected)
- attack mismatches on the same spin: 0 / 0

## Interpretation

- The 40-row patch works: **0 forfeits**, 10/10 replays verified, and both
  sides play real TopOut games.
- **intetrigence wins 0 of 10 by TopOut.** The bot stacks higher (max 20–23)
  than attax (10–19) and is out-attacked (0.766 vs 0.843 attack/piece). The
  one non-loss is a 1000-turn TurnCap, which is **unresolved**, not a win.
- This supersedes the earlier "94% / 95%" figures: those counted forfeits as
  wins. The true TopOut record against the patched attax is **0–9** in this
  diagnostic sample.
- The stage-1 gate (`wins_topout == 50`) is far from the current strength.
  The next step is bot strengthening (search/evaluation), not harness work.

## Caveats

- 10 games only; the Wilson 95% CI for 0/9 is wide (0.0–29.9% for a TopOut
  win rate). This is a smoke, not an estimate.
- The raw `summary.json` was written by the run binary before the
  `TurnCap`-winner fix (`bfb77fe`); it records the 11004-B TurnCap as a win.
  `summary-reclassified.json` applies the v1.27 rule (`winner = None`) and is
  the authoritative classification for this evidence.
- Spin classification is more lenient than TETR.IO (protocol v1.7 §4.2.1);
  no claim is made against a full-vision TETR.IO player.
- The 20-game tuning run (seeds 9001–9010 × A/B) is reported separately.
