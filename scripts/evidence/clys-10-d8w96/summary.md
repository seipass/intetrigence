# S2 match summary

- games: 10
- build id: `609f350707b6be91`; binary: `C:\Users\Hasilan\.hasilan-code\agent\web-harness\agent-worktrees\1cf19499-c382-48a1-89f9-aad965f1bf06\target\release\arena.exe (1899008 bytes, mtime 1794819603)`
- opponent: clys (commit ccae45fc3e30c3d3432af2b06d5b4f7f2e8dfbcc, build id unknown, visible rows 40, threads n/a, suggest delay 0 ms, empty-suggest retry 2000 ms)
- intetrigence wins / losses / unresolved / no-contest: 4 / 5 / 0 / 1
- win rate: 40.0% (Wilson 95% CI 16.8%..68.7%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 4, forfeit-vision 0, forfeit-other 0; losses: TopOut 5, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 80 | yes | 93.8 / 116.3 | 98.4 / 137.0 | 1.00 / 1.00 | 1.00 / 0.06 | 11 |
| 9002 | A | clys | TopOut | - | 278 | yes | 88.1 / 114.9 | 97.1 / 135.5 | 1.00 / 1.00 | 1.00 / 0.07 | 23 |
| 9003 | A | clys | TopOut | - | 193 | yes | 87.8 / 115.6 | 96.6 / 134.2 | 1.00 / 1.00 | 1.00 / 0.08 | 18 |
| 9004 | A | clys | TopOut | - | 135 | yes | 86.2 / 117.2 | 94.1 / 135.7 | 1.00 / 1.00 | 1.00 / 0.07 | 11 |
| 9005 | A | intetrigence | TopOut | - | 132 | yes | 85.9 / 113.7 | 95.4 / 136.8 | 1.00 / 1.00 | 1.00 / 0.09 | 18 |
| 9006 | A | no contest | Forfeit | other | 71 | yes | 87.8 / 116.0 | 96.0 / 134.7 | 1.00 / 1.00 | 1.00 / 0.09 | 8 |
| 9007 | A | clys | TopOut | - | 41 | yes | 88.0 / 114.6 | 96.8 / 135.1 | 1.00 / 1.00 | 1.00 / 0.07 | 6 |
| 9008 | A | intetrigence | TopOut | - | 83 | yes | 88.1 / 122.1 | 96.7 / 138.6 | 1.00 / 1.00 | 1.00 / 0.05 | 14 |
| 9009 | A | clys | TopOut | - | 74 | yes | 81.6 / 115.5 | 95.2 / 133.3 | 1.00 / 1.00 | 1.00 / 0.08 | 5 |
| 9010 | A | intetrigence | TopOut | - | 115 | yes | 84.0 / 116.4 | 95.0 / 137.6 | 1.00 / 1.00 | 1.00 / 0.07 | 11 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 1202 | 0 / 1202 | 0 / 1202 | 0.812 | 51.6 | 5.8 / 17.1 | 0 |
| 1 | clys | 40 | 1196 | 1107 / 1196 | 0 / 1196 | 0.961 | 34.3 | 8.1 / 18.9 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is deterministic: depth 8 width 96 over 8 thread(s), no wall-clock deadline. Our moves are a pure function of the position sequence; the opponent is a separate process, so the same seed does not reproduce the same game. The replay is the record.
