# S2 match summary

- games: 10
- build id: `a4c99e4b70804931`; binary: `C:\Users\Hasilan\Projects\intetrigence\target\release\arena.exe (1901568 bytes, mtime 1794830895)`
- opponent: Attax (commit 1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0, build id unknown, visible rows 40, threads n/a, suggest delay 0 ms, empty-suggest retry 2000 ms)
- intetrigence wins / losses / unresolved / no-contest: 1 / 8 / 0 / 1
- win rate: 10.0% (Wilson 95% CI 1.8%..40.4%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 8, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | Attax | TopOut | - | 185 | yes | 63.5 / 42.0 | 69.6 / 49.7 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9002 | A | no contest | Forfeit | other | 439 | yes | 61.6 / 40.8 | 68.2 / 51.2 | 1.00 / 1.00 | 1.00 / 1.00 | 41 |
| 9003 | A | intetrigence | TopOut | - | 106 | yes | 61.5 / 42.5 | 68.7 / 53.2 | 1.00 / 1.00 | 1.00 / 1.00 | 14 |
| 9004 | A | Attax | TopOut | - | 141 | yes | 62.3 / 41.9 | 69.2 / 53.3 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9005 | A | Attax | TopOut | - | 272 | yes | 62.7 / 44.9 | 71.6 / 57.2 | 1.00 / 1.00 | 1.00 / 0.99 | 21 |
| 9006 | A | Attax | TopOut | - | 393 | yes | 61.3 / 43.7 | 68.2 / 55.8 | 1.00 / 1.00 | 1.00 / 1.00 | 37 |
| 9007 | A | Attax | TopOut | - | 219 | yes | 63.1 / 44.8 | 68.3 / 55.3 | 1.00 / 1.00 | 1.00 / 1.00 | 21 |
| 9008 | A | Attax | TopOut | - | 758 | yes | 60.7 / 44.5 | 68.2 / 56.1 | 1.00 / 1.00 | 1.00 / 1.00 | 73 |
| 9009 | A | Attax | TopOut | - | 741 | yes | 61.5 / 42.7 | 68.5 / 53.0 | 1.00 / 1.00 | 1.00 / 1.00 | 74 |
| 9010 | A | Attax | TopOut | - | 57 | yes | 69.4 / 43.8 | 77.3 / 53.8 | 1.00 / 1.00 | 1.00 / 1.00 | 2 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 3311 | 0 / 3311 | 0 / 3311 | 0.771 | 133.4 | 6.0 / 19.5 | 0 |
| 1 | Attax | 40 | 3302 | 5 / 3302 | 0 / 3302 | 0.935 | 80.9 | 5.3 / 16.4 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- host: Ryzen 7 3700X 8C/16T 15.9GB; weights default holes=-20; deterministic depth 8 width 88; attax S2 profile, 180-kick patch, supports_180=true
- The intetrigence search is deterministic: depth 8 width 88 over 8 thread(s), no wall-clock deadline. Our moves are a pure function of the position sequence; the opponent is a separate process, so the same seed does not reproduce the same game. The replay is the record.
