# S2 match summary

- games: 10
- build id: `6d26c8530b5a82c0`; binary: `C:\Users\Hasilan\.hasilan-code\agent\web-harness\agent-worktrees\1cf19499-c382-48a1-89f9-aad965f1bf06\target\release\arena.exe (1901568 bytes, mtime 1794820577)`
- opponent: clys (commit ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc, build id unknown, visible rows 40, threads n/a, suggest delay 0 ms, empty-suggest retry 2000 ms)
- intetrigence wins / losses / unresolved / no-contest: 5 / 5 / 0 / 0
- win rate: 50.0% (Wilson 95% CI 23.7%..76.3%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 5, forfeit-vision 0, forfeit-other 0; losses: TopOut 5, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | clys | TopOut | - | 125 | yes | 68.7 / 113.4 | 80.4 / 125.8 | 1.00 / 1.00 | 1.00 / 0.08 | 12 |
| 9002 | A | clys | TopOut | - | 981 | yes | 72.2 / 112.8 | 95.9 / 132.2 | 1.00 / 1.00 | 1.00 / 0.08 | 104 |
| 9003 | A | clys | TopOut | - | 164 | yes | 85.5 / 118.8 | 94.2 / 135.4 | 1.00 / 1.00 | 1.00 / 0.08 | 14 |
| 9004 | A | clys | TopOut | - | 208 | yes | 86.0 / 118.3 | 94.2 / 133.9 | 1.00 / 1.00 | 1.00 / 0.06 | 17 |
| 9005 | A | clys | TopOut | - | 298 | yes | 85.3 / 116.6 | 96.5 / 134.8 | 1.00 / 1.00 | 1.00 / 0.08 | 38 |
| 9006 | A | intetrigence | TopOut | - | 97 | yes | 85.4 / 116.2 | 97.1 / 131.6 | 1.00 / 1.00 | 1.00 / 0.09 | 12 |
| 9007 | A | intetrigence | TopOut | - | 61 | yes | 84.3 / 121.7 | 93.4 / 136.9 | 1.00 / 1.00 | 1.00 / 0.07 | 11 |
| 9008 | A | intetrigence | TopOut | - | 83 | yes | 87.0 / 120.0 | 94.6 / 137.6 | 1.00 / 1.00 | 1.00 / 0.05 | 14 |
| 9009 | A | intetrigence | TopOut | - | 110 | yes | 80.5 / 115.5 | 92.4 / 133.8 | 1.00 / 1.00 | 1.00 / 0.10 | 9 |
| 9010 | A | intetrigence | TopOut | - | 115 | yes | 83.1 / 117.3 | 93.0 / 137.3 | 1.00 / 1.00 | 1.00 / 0.07 | 11 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 2242 | 0 / 2242 | 0 / 2242 | 0.857 | 98.3 | 6.5 / 18.4 | 0 |
| 1 | clys | 40 | 2237 | 2060 / 2237 | 0 / 2237 | 1.030 | 60.0 | 8.5 / 19.3 | 7 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is deterministic: depth 8 width 96 over 8 thread(s), no wall-clock deadline. Our moves are a pure function of the position sequence; the opponent is a separate process, so the same seed does not reproduce the same game. The replay is the record.
