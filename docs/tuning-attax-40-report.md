# attax 40-row tuning baseline (diagnostic)

> **Tuning run, not an evaluation round.** Seeds 9001–9010 are reserved for
> tuning (protocol v1.15 §1.6); they are never used for a claim. The purpose
> is to measure the true TopOut baseline against the 40-row patched attax
> before bot work continues.

## Setup

- intetrigence: `12196ec` (gameplay binary), `--bot search --ms 100 --threads 8`
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b` + `adapters/attax-vision-40.patch`
  (68 lines, SHA-256 `e22aab7b5a2f9a95efb562e24b3f33da351bb942d6daf6e205475b5871ed3b70`)
- visible rows: 40 both sides; seeds 9001–9010 × A/B = 20 games; 1000 turn cap
- evidence: `scripts/evidence/tuning-attax-40/` (replay + audit + attax decision log per game)
- classification: `summary-reclassified.json` (protocol v1.27/v1.28; TurnCap /
  Forfeit / Stuck have `winner = None`)

## Result

- games 20, replay verified **20/20**, measurement complete 20/20
- **wins_topout 2 / losses_topout 16 / unresolved 0 / no_contest 2 / stuck 0**
- TopOut record: **2/18 = 11.1%** (raw 2/20 = 10.0%)
- no-contest: 2 × `forfeit-other` (attax side returned candidates the referee
  could not reach; both at intetrigence height 21)

| seed | sheet | result | turns | attax max | intetrigence max | attack/piece (int / attax) | received garbage (int / attax) |
|---|---|---|---|---|---|---|---|
| 9001 | A | Forfeit (no contest) | 243 | 15 | 21 | 0.885 / 0.971 | 97 / 77 |
| 9001 | B | TopOut loss | 60 | 13 | 20 | 0.450 / 0.733 | 24 / 7 |
| 9002 | A | TopOut loss | 163 | 14 | 20 | 0.730 / 0.864 | 64 / 44 |
| 9002 | B | TopOut loss | 404 | 20 | 21 | 0.886 / 0.856 | 118 / 132 |
| 9003 | A | TopOut loss | 295 | 16 | 19 | 0.756 / 0.796 | 93 / 83 |
| 9003 | B | **TopOut win** | 193 | 15 | 20 | 0.802 / 0.762 | 60 / 61 |
| 9004 | A | TopOut loss | 269 | 17 | 19 | 0.877 / 0.892 | 85 / 83 |
| 9004 | B | TopOut loss | 133 | 12 | 21 | 0.602 / 0.797 | 53 / 27 |
| 9005 | A | TopOut loss | 401 | 18 | 21 | 0.768 / 0.818 | 123 / 106 |
| 9005 | B | TopOut loss | 246 | 16 | 23 | 0.715 / 0.809 | 76 / 55 |
| 9006 | A | TopOut loss | 403 | 17 | 22 | 0.801 / 0.838 | 129 / 117 |
| 9006 | B | TopOut loss | 531 | 20 | 22 | 0.915 / 0.883 | 163 / 185 |
| 9007 | A | TopOut loss | 800 | 18 | 20 | 0.845 / 0.882 | 254 / 226 |
| 9007 | B | TopOut loss | 217 | 17 | 20 | 0.829 / 0.848 | 57 / 54 |
| 9008 | A | TopOut loss | 143 | 16 | 21 | 0.734 / 0.768 | 41 / 34 |
| 9008 | B | TopOut loss | 53 | 14 | 17 | 0.585 / 0.811 | 23 / 10 |
| 9009 | A | **TopOut win** | 64 | 20 | 13 | 1.063 / 0.688 | 12 / 33 |
| 9009 | B | TopOut loss | 239 | 19 | 22 | 0.808 / 0.870 | 79 / 65 |
| 9010 | A | TopOut loss | 537 | 16 | 22 | 0.827 / 0.853 | 150 / 144 |
| 9010 | B | Forfeit (no contest) | 459 | 14 | 21 | 0.784 / 0.782 | 122 / 120 |

Aggregates (mean over 20 games):

- attack/piece: intetrigence **0.783** / attax **0.826**
- received garbage: intetrigence **91.2** / attax **83.2**
- think median: intetrigence **97.3 ms** / attax **33.6 ms**
- think over 200 ms: 0 / 0
- spin mismatches: intetrigence **0** / attax **106** (attax is not S2-native; expected)
- attack mismatches on the same spin: 0 / 0

## Findings

1. **The true TopOut baseline is ~11% (2/18), not 94%.** The earlier high
   figures counted forfeits as wins. The bot is out-attacked (0.783 vs 0.826)
   and receives more garbage (91.2 vs 83.2); it tops out at height 17–23 while
   attax usually stays at 12–20.
2. **The 40-row patch is not forfeit-free.** 2/20 games ended in
   `forfeit-other`: attax returned a placement the referee's movegen could not
   reach. attax-integration re-ran 9001-A and got a 1000-turn TurnCap with no
   forfeit, so the case is **state-dependent** (intetrigence's anytime search
   is non-deterministic), not a fixed coordinate bug. The coordinate conversion
   was checked and is correct; the suspected cause is a rare board desync
   (line clear / garbage).
3. **A single `forfeit-other` fails stage 1** under protocol v1.27
   (`no_contest == 0`). With a ~10% observed rate, a 50-game stage would very
   likely contain at least one. The adapter desync must be fixed (or the
   forfeit rule revisited) before an official round can pass.
4. **The stage-1 gate (`wins_topout == 50`) is far from the current
   strength.** The next step is bot search/evaluation work, not harness work.

## Caveats

- 20 games, tuning seeds; the Wilson 95% CI for 2/18 is 3.1–31.6%. This is a
  baseline, not an estimate of the official round.
- intetrigence's anytime search is non-deterministic across runs (thread
  scheduling); the replay is the record, and a re-run of the same seed may
  differ. The seed discipline (new build + new seeds for a retry) is not
  affected.
- The raw `summary.json` was written before the TurnCap/Stuck-winner fix
  (`bfb77fe`/`4677385`); it agrees with `summary-reclassified.json` here
  because this sample has no TurnCap or Stuck games.
- Spin classification is more lenient than TETR.IO (protocol v1.7 §4.2.1).
