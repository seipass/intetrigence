# S2 match summary

- games: 10
- build id: `a4c99e4b70804931`; binary: `C:\Users\Hasilan\Projects\intetrigence\target\release\arena.exe (1901568 bytes, mtime 1794830895)`
- opponent: Attax (commit 1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0, build id unknown, visible rows 40, threads n/a, suggest delay 0 ms, empty-suggest retry 2000 ms)
- intetrigence wins / losses / unresolved / no-contest: 1 / 7 / 0 / 2
- win rate: 10.0% (Wilson 95% CI 1.8%..40.4%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 7, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 173 | yes | 62.5 / 42.2 | 69.7 / 53.5 | 1.00 / 1.00 | 1.00 / 1.00 | 17 |
| 9002 | A | Attax | TopOut | - | 540 | yes | 62.4 / 44.6 | 71.6 / 58.1 | 1.00 / 1.00 | 1.00 / 1.00 | 47 |
| 9003 | A | no contest | Forfeit | other | 269 | yes | 66.6 / 45.1 | 77.3 / 58.1 | 1.00 / 1.00 | 1.00 / 1.00 | 21 |
| 9004 | A | Attax | TopOut | - | 161 | yes | 62.2 / 41.6 | 68.2 / 54.7 | 1.00 / 1.00 | 1.00 / 1.00 | 14 |
| 9005 | A | Attax | TopOut | - | 48 | yes | 60.9 / 47.3 | 69.0 / 59.5 | 1.00 / 1.00 | 1.00 / 1.00 | 2 |
| 9006 | A | Attax | TopOut | - | 149 | yes | 59.6 / 43.8 | 69.6 / 53.9 | 1.00 / 1.00 | 1.00 / 1.00 | 11 |
| 9007 | A | no contest | Forfeit | other | 126 | yes | 64.4 / 44.6 | 70.1 / 56.3 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9008 | A | Attax | TopOut | - | 351 | yes | 64.1 / 45.7 | 70.3 / 57.1 | 1.00 / 1.00 | 1.00 / 1.00 | 30 |
| 9009 | A | Attax | TopOut | - | 228 | yes | 64.7 / 48.0 | 70.8 / 59.3 | 1.00 / 1.00 | 1.00 / 1.00 | 15 |
| 9010 | A | Attax | TopOut | - | 705 | yes | 67.2 / 50.2 | 80.4 / 61.7 | 1.00 / 1.00 | 1.00 / 1.00 | 67 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 2749 | 0 / 2749 | 0 / 2749 | 0.734 | 103.4 | 6.3 / 19.6 | 0 |
| 1 | Attax | 40 | 2741 | 1 / 2741 | 0 / 2741 | 0.891 | 61.8 | 5.3 / 17.2 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- host: Ryzen 7 3700X 8C/16T 15.9GB; weights holes=-30; deterministic depth 8 width 88; attax S2 profile, 180-kick patch, supports_180=true
- The intetrigence search is deterministic: depth 8 width 88 over 8 thread(s), no wall-clock deadline. Our moves are a pure function of the position sequence; the opponent is a separate process, so the same seed does not reproduce the same game. The replay is the record.
