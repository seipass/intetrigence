# S2 match summary

- games: 10
- build id: `a4c99e4b70804931`; binary: `C:\Users\Hasilan\Projects\intetrigence\target\release\arena.exe (1901568 bytes, mtime 1794830895)`
- opponent: clys (commit ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc, build id unknown, visible rows 20, threads n/a, suggest delay 0 ms, empty-suggest retry 2000 ms)
- intetrigence wins / losses / unresolved / no-contest: 4 / 4 / 1 / 1
- win rate: 40.0% (Wilson 95% CI 16.8%..68.7%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 4, forfeit-vision 0, forfeit-other 0; losses: TopOut 4, forfeit 0; turn-cap games 1

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | clys | TopOut | - | 40 | yes | 68.1 / 111.6 | 75.7 / 125.0 | 1.00 / 1.00 | 1.00 / 0.05 | 1 |
| 9002 | A | clys | TopOut | - | 75 | yes | 65.2 / 110.9 | 70.5 / 124.9 | 1.00 / 1.00 | 1.00 / 0.08 | 7 |
| 9003 | A | unresolved | TurnCap | - | 1000 | yes | 64.5 / 110.6 | 72.6 / 124.0 | 1.00 / 1.00 | 1.00 / 0.08 | 103 |
| 9004 | A | clys | TopOut | - | 291 | yes | 64.3 / 109.1 | 70.7 / 124.8 | 1.00 / 1.00 | 1.00 / 0.09 | 27 |
| 9005 | A | intetrigence | TopOut | - | 733 | yes | 64.4 / 110.0 | 72.4 / 125.7 | 1.00 / 1.00 | 1.00 / 0.08 | 77 |
| 9006 | A | intetrigence | TopOut | - | 630 | yes | 62.0 / 108.8 | 69.4 / 123.9 | 1.00 / 1.00 | 1.00 / 0.08 | 68 |
| 9007 | A | no contest | Forfeit | other | 115 | yes | 62.9 / 110.8 | 69.4 / 126.7 | 1.00 / 1.00 | 1.00 / 0.07 | 15 |
| 9008 | A | intetrigence | TopOut | - | 49 | yes | 64.7 / 111.5 | 73.2 / 125.1 | 1.00 / 1.00 | 1.00 / 0.02 | 6 |
| 9009 | A | clys | TopOut | - | 168 | yes | 61.9 / 110.6 | 68.8 / 124.1 | 1.00 / 1.00 | 1.00 / 0.10 | 12 |
| 9010 | A | intetrigence | TopOut | - | 263 | yes | 61.7 / 111.8 | 68.4 / 126.7 | 1.00 / 1.00 | 1.00 / 0.08 | 30 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 3364 | 0 / 3364 | 0 / 3364 | 0.858 | 147.0 | 6.4 / 19.6 | 0 |
| 1 | clys | 20 | 3359 | 3090 / 3359 | 0 / 3359 | 1.043 | 85.3 | 7.9 / 20.9 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- host: Ryzen 7 3700X 8C/16T 15.9GB; weights default holes=-20; deterministic depth 8 width 88; clys visible_rows=20 (as in the 5-5 run)
- The intetrigence search is deterministic: depth 8 width 88 over 8 thread(s), no wall-clock deadline. Our moves are a pure function of the position sequence; the opponent is a separate process, so the same seed does not reproduce the same game. The replay is the record.
