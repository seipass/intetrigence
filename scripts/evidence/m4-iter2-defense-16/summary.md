# S2 match summary

- games: 20
- intetrigence wins / losses / draws / forfeits: 19 / 1 / 0 / 17
- win rate: 95.0% (Wilson 95% CI 76.4%..99.1%)
- replay verified: 20/20 (failures: 0)
- win breakdown: TopOut 0, forfeit-vision 17, forfeit-other 0; losses: TopOut 1, forfeit 0; turn-cap games 2
- attax: Attax overhaul/full commit 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b bun 1.4.0
- attax command: `bun C:/Users/Hasilan/Projects/attax\tbp\attax-tbp.ts`

## Games

| seed | sheet | winner | reason | forfeit cause | turns | verified | think median (ms) | think p95 (ms) | first-choice | spin match | start resends |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 9001 | A | intetrigence | Forfeit | vision | 249 | yes | 96.9 / 26.4 | 98.5 / 32.8 | 1.00 / 1.00 | 1.00 / 0.98 | 31 |
| 9001 | B | intetrigence | TurnCap | - | 1000 | yes | 96.9 / 26.2 | 98.3 / 32.3 | 1.00 / 1.00 | 1.00 / 0.99 | 140 |
| 9002 | A | intetrigence | Forfeit | vision | 28 | yes | 96.7 / 25.6 | 97.6 / 30.3 | 1.00 / 1.00 | 1.00 / 0.96 | 6 |
| 9002 | B | intetrigence | Forfeit | vision | 28 | yes | 96.9 / 25.3 | 98.7 / 30.4 | 1.00 / 1.00 | 1.00 / 0.96 | 7 |
| 9003 | A | intetrigence | Forfeit | vision | 215 | yes | 96.8 / 27.2 | 98.0 / 33.6 | 1.00 / 1.00 | 1.00 / 0.98 | 32 |
| 9003 | B | intetrigence | Forfeit | vision | 233 | yes | 96.7 / 27.3 | 98.0 / 32.2 | 1.00 / 1.00 | 1.00 / 0.97 | 34 |
| 9004 | A | intetrigence | Forfeit | vision | 161 | yes | 96.8 / 28.2 | 98.3 / 34.2 | 1.00 / 1.00 | 1.00 / 0.99 | 14 |
| 9004 | B | intetrigence | Forfeit | vision | 392 | yes | 96.8 / 28.6 | 98.3 / 36.5 | 1.00 / 1.00 | 1.00 / 0.99 | 47 |
| 9005 | A | intetrigence | Forfeit | vision | 923 | yes | 96.8 / 26.7 | 98.2 / 34.0 | 1.00 / 1.00 | 1.00 / 0.98 | 142 |
| 9005 | B | intetrigence | TurnCap | - | 1000 | yes | 96.8 / 26.9 | 98.1 / 33.5 | 1.00 / 1.00 | 1.00 / 0.99 | 121 |
| 9006 | A | intetrigence | Forfeit | vision | 209 | yes | 96.8 / 26.9 | 98.0 / 33.8 | 1.00 / 1.00 | 1.00 / 0.98 | 27 |
| 9006 | B | intetrigence | Forfeit | vision | 587 | yes | 96.8 / 27.5 | 98.1 / 34.5 | 1.00 / 1.00 | 1.00 / 0.99 | 83 |
| 9007 | A | intetrigence | Forfeit | vision | 71 | yes | 96.6 / 27.9 | 98.1 / 35.4 | 1.00 / 1.00 | 1.00 / 0.99 | 6 |
| 9007 | B | intetrigence | Forfeit | vision | 822 | yes | 96.8 / 27.3 | 98.3 / 34.8 | 1.00 / 1.00 | 1.00 / 0.99 | 115 |
| 9008 | A | intetrigence | Forfeit | vision | 138 | yes | 96.9 / 27.0 | 98.3 / 33.5 | 1.00 / 1.00 | 1.00 / 0.99 | 19 |
| 9008 | B | intetrigence | Forfeit | vision | 79 | yes | 96.8 / 28.1 | 98.3 / 35.6 | 1.00 / 1.00 | 1.00 / 1.00 | 10 |
| 9009 | A | intetrigence | Forfeit | vision | 445 | yes | 96.9 / 26.9 | 98.2 / 33.6 | 1.00 / 1.00 | 1.00 / 0.98 | 60 |
| 9009 | B | attax | TopOut | - | 48 | yes | 96.7 / 26.6 | 98.1 / 34.7 | 1.00 / 1.00 | 1.00 / 0.92 | 3 |
| 9010 | A | intetrigence | Forfeit | vision | 184 | yes | 96.8 / 27.0 | 98.2 / 33.8 | 1.00 / 1.00 | 1.00 / 0.98 | 29 |
| 9010 | B | intetrigence | Forfeit | vision | 617 | yes | 96.8 / 28.0 | 98.1 / 35.0 | 1.00 / 1.00 | 1.00 / 0.98 | 82 |

## Per-bot audit

| slot | bot | visible rows | placements | spin mismatch | attack mismatch (same spin) | attack/piece | received/game | think > 200ms |
|---|---|---|---|---|---|---|---|---|
| 0 | intetrigence | 40 | 7422 | 0 / 7422 | 0 / 7422 | 0.972 | 111.0 | 0 |
| 1 | Attax | 20 | 7412 | 108 / 7412 | 0 / 7412 | 0.876 | 144.4 | 0 |

## Notes

- The intetrigence side is the bot recorded in each replay header.
- M3 defense-stress diagnostic baseline (16 start garbage rows, --ms 100, 8 threads). Not usable for final claims.
