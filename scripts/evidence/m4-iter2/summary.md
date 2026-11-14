# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 20 / 0 / 0 / 19
- win rate: 100.0% (Wilson 95% CI 83.9%..100.0%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 19, forfeit-other 0; losses: TopOut 0, forfeit 0; turn-cap games 1
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10021 | A | intetrigence | Forfeit | vision | 133 | yes | 96.9 / 27.7 | 98.6 / 36.2 | 1.00 / 1.00 | 1.00 / 0.99 | 21 |
| 10021 | B | intetrigence | Forfeit | vision | 101 | yes | 96.9 / 28.3 | 98.1 / 34.2 | 1.00 / 1.00 | 1.00 / 0.99 | 15 |
| 10022 | A | intetrigence | Forfeit | vision | 179 | yes | 96.9 / 26.3 | 98.4 / 33.0 | 1.00 / 1.00 | 1.00 / 0.99 | 28 |
| 10022 | B | intetrigence | Forfeit | vision | 206 | yes | 96.9 / 26.5 | 98.0 / 32.6 | 1.00 / 1.00 | 1.00 / 0.98 | 27 |
| 10023 | A | intetrigence | TurnCap | - | 1000 | yes | 96.8 / 27.0 | 98.1 / 33.8 | 1.00 / 1.00 | 1.00 / 0.99 | 129 |
| 10023 | B | intetrigence | Forfeit | vision | 355 | yes | 96.8 / 27.3 | 98.2 / 34.0 | 1.00 / 1.00 | 1.00 / 0.99 | 46 |
| 10024 | A | intetrigence | Forfeit | vision | 258 | yes | 96.9 / 26.6 | 98.5 / 33.8 | 1.00 / 1.00 | 1.00 / 0.99 | 29 |
| 10024 | B | intetrigence | Forfeit | vision | 551 | yes | 96.9 / 26.9 | 98.3 / 34.4 | 1.00 / 1.00 | 1.00 / 0.98 | 79 |
| 10025 | A | intetrigence | Forfeit | vision | 51 | yes | 96.8 / 26.7 | 98.1 / 34.6 | 1.00 / 1.00 | 1.00 / 1.00 | 9 |
| 10025 | B | intetrigence | Forfeit | vision | 375 | yes | 96.7 / 27.4 | 98.2 / 34.4 | 1.00 / 1.00 | 1.00 / 0.99 | 39 |
| 10026 | A | intetrigence | Forfeit | vision | 279 | yes | 96.9 / 28.4 | 98.2 / 35.0 | 1.00 / 1.00 | 1.00 / 0.99 | 42 |
| 10026 | B | intetrigence | Forfeit | vision | 246 | yes | 96.8 / 27.2 | 97.8 / 33.8 | 1.00 / 1.00 | 1.00 / 0.99 | 35 |
| 10027 | A | intetrigence | Forfeit | vision | 53 | yes | 96.6 / 29.4 | 98.5 / 35.7 | 1.00 / 1.00 | 1.00 / 1.00 | 9 |
| 10027 | B | intetrigence | Forfeit | vision | 375 | yes | 96.9 / 28.5 | 98.0 / 35.8 | 1.00 / 1.00 | 1.00 / 0.99 | 52 |
| 10028 | A | intetrigence | Forfeit | vision | 424 | yes | 96.8 / 26.9 | 98.2 / 33.0 | 1.00 / 1.00 | 1.00 / 0.99 | 59 |
| 10028 | B | intetrigence | Forfeit | vision | 43 | yes | 96.6 / 27.2 | 98.0 / 33.9 | 1.00 / 1.00 | 1.00 / 0.95 | 7 |
| 10029 | A | intetrigence | Forfeit | vision | 102 | yes | 96.7 / 28.6 | 97.7 / 32.7 | 1.00 / 1.00 | 1.00 / 1.00 | 17 |
| 10029 | B | intetrigence | Forfeit | vision | 692 | yes | 96.8 / 27.3 | 98.2 / 34.0 | 1.00 / 1.00 | 1.00 / 0.98 | 99 |
| 10030 | A | intetrigence | Forfeit | vision | 482 | yes | 96.8 / 27.0 | 98.0 / 34.0 | 1.00 / 1.00 | 1.00 / 0.98 | 64 |
| 10030 | B | intetrigence | Forfeit | vision | 217 | yes | 96.9 / 27.4 | 98.1 / 33.5 | 1.00 / 1.00 | 1.00 / 0.99 | 31 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 6112 | 0 / 6112 | 0 / 6112 | 0.982 | 82.6 | 0 |
| 1 | Attax | 20 | 6103 | 81 / 6103 | 0 / 6103 | 0.869 | 115.7 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M2 gate: real intetrigence search (--ms 100, 8 threads) vs attax 7b38f51. Forfeit wins are broken down by cause in the report.
