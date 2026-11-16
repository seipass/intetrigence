# S2 match summary

- games: 10
- build id: `198fb343677c8c8c`; binary: `C:\Users\Hasilan\Projects\intetrigence\target\release\arena.exe (1901568 bytes, mtime 1794825483)`
- opponent: Attax (commit 1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0, build id unknown, visible rows 40, threads n/a, suggest delay 0 ms, empty-suggest retry 2000 ms)
- intetrigence wins / losses / unresolved / no-contest: 1 / 7 / 0 / 2
- win rate: 10.0% (Wilson 95% CI 1.8%..40.4%)
- replay verified: 10/10 (failures: 0)
- win breakdown: TopOut 1, forfeit-vision 0, forfeit-other 0; losses: TopOut 7, forfeit 0; turn-cap games 0

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | TopOut | - | 173 | yes | 63.1 / 44.1 | 71.1 / 55.7 | 1.00 / 1.00 | 1.00 / 1.00 | 17 |
| 9002 | A | Attax | TopOut | - | 540 | yes | 64.8 / 45.6 | 71.2 / 56.2 | 1.00 / 1.00 | 1.00 / 1.00 | 47 |
| 9003 | A | no contest | Forfeit | other | 269 | yes | 65.5 / 46.1 | 71.2 / 58.1 | 1.00 / 1.00 | 1.00 / 1.00 | 21 |
| 9004 | A | Attax | TopOut | - | 161 | yes | 64.9 / 44.1 | 71.6 / 55.9 | 1.00 / 1.00 | 1.00 / 1.00 | 14 |
| 9005 | A | Attax | TopOut | - | 48 | yes | 62.3 / 43.4 | 69.8 / 54.6 | 1.00 / 1.00 | 1.00 / 1.00 | 2 |
| 9006 | A | Attax | TopOut | - | 149 | yes | 61.3 / 43.3 | 69.5 / 53.5 | 1.00 / 1.00 | 1.00 / 1.00 | 11 |
| 9007 | A | no contest | Forfeit | other | 126 | yes | 65.3 / 44.7 | 73.7 / 59.9 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9008 | A | Attax | TopOut | - | 351 | yes | 64.2 / 45.5 | 71.7 / 57.7 | 1.00 / 1.00 | 1.00 / 1.00 | 30 |
| 9009 | A | Attax | TopOut | - | 228 | yes | 62.5 / 44.1 | 68.0 / 53.7 | 1.00 / 1.00 | 1.00 / 1.00 | 15 |
| 9010 | A | Attax | TopOut | - | 770 | yes | 62.3 / 43.6 | 68.5 / 55.0 | 1.00 / 1.00 | 1.00 / 1.00 | 58 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | mean/max height | think > 200ms |
|---|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 2814 | 0 / 2814 | 0 / 2814 | 0.729 | 105.8 | 6.3 / 19.6 | 0 |
| 1 | Attax | 40 | 2806 | 1 / 2806 | 0 / 2806 | 0.900 | 60.8 | 5.3 / 17.1 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- The intetrigence search is deterministic: depth 8 width 88 over 8 thread(s), no wall-clock deadline. Our moves are a pure function of the position sequence; the opponent is a separate process, so the same seed does not reproduce the same game. The replay is the record.
