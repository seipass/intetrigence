# S2 attax 10 ゲーム(tuning seeds 9001–9010、sheet A)

条件: 1000 turn、`--ms 100`/8t、**決定論 `--deterministic --depth 8 --width 88`**、
`ATTAX_TBP_ROWS=40` + `ATTAX_TBP_PROFILE=s2`、`--tbp-supports-180 true`、attax pin `1f41806` + `adapters/attax-s2-tbp.patch`。

## 5 分類(合計 10 の検算)

| 分類 | 数 | seeds |
|---|---|---|
| TopOut 勝ち | **1** | 9001 |
| TopOut 負け | **7** | 9002, 9004, 9005, 9006, 9008, 9009, 9010 |
| TurnCap | **0** | — |
| Forfeit(no contest) | **2** | 9003, 9007 |
| 未決着(unresolved) | **0** | — |
| **合計** | **10** | |

- 勝率 10.0%(Wilson 95% CI 1.8%..40.4%)。replay 検証 **10/10**。
- 負けの内訳: collapse 6、race 1、forfeit 2。
- forfeit は両方 **`candidate-unreachable`**(`detail: "none of the 6 candidates is reachable"`、attax slot 1、
  盤面は低い = final height 4/6 と 5/2。TopOut ではない)。

## 指標(10 ゲーム合計)

| 指標 | intetrigence | Attax(S2) |
|---|---|---|
| attack/piece | **0.729** | **0.900** |
| attack/line | **1.061** | **1.560** |
| spin rate(非 none の設置率) | **0.086** | **0.151** |
| net damage / game | **+23.4** | **+123.9** |
| received / game | 105.8 | 60.8 |
| mean / max height | 6.29 / 19.6 | 5.31 / 17.1 |
| collapse(最終高さ ≥ 20) | 1 | 0 |
| collapse 負け / race 負け | 6 / 1 | — |
| think median(ms) | 63.6 | 44.4 |
| think p95 / max(ms) | 73.7 / 96.0 | 59.9 / 75.1 |
| nodes median | 580(depth 8 / width 88) | 10,149 |
| spin 監査 | 0 / 2814 | **1 / 2806** |
| attack 監査(same spin) | 0 / 2814 | 0 / 2806 |
| fallback / aborted | 0 / 0 | 0 / 0 |
| queue invariant | 0 | 0 |

- **attack/piece でも負け**(0.729 対 0.900)。**受信 105.8 対 60.8**、**net damage +23.4 対 +123.9**。
- attax の **spin rate 0.151**(我々 0.086)、**attack/line 1.560**(我々 1.061)。
- attax の **surge 放出**は 9001 の 13 回から 9010 の 112 回まで伸びる(`max_b2b_streak` 27)。
  ただし **`createBot` は S2-only フィールドを渡さない**ため、これはレフェリー採点側の surge であり、
  attax の探索が surge を計画しているわけではない(下記制約)。
- **`load_note`: null**(`--note` 未指定。マシン負荷の記録なし)。

## Forfeit の原因(2/10)

- 両方とも **attax の 180° kick 表が S2 と一致しない**ため。
  - attax `KICKS_180_JLSTZ` は **6 候補**(例 0→2: `(0,0),(0,1),(1,1),(-1,1),(1,0),(-1,0)`)。
  - レフェリー `crates/core/src/kick.rs` の `JLSTZ_180_KICKS` は **5 候補**(`(0,0),(0,1),(1,1),(-1,1),(1,0)`)。
  - attax の **6 番目**が S2 に無い。attax の 6 候補がこの 6 番目を使う 180° 配置だと、レフェリーの movegen が
    none になり forfeit する。
- `--tbp-supports-180 false` の 1 ゲーム(9001)では forfeit 0。`true` の 10 ゲームで 2。
  **180° を許可したことが原因**で、強さの問題ではない。
- 修正候補: attax の `KICKS_180_JLSTZ` / `KICKS_180_I` を **S2 の 5 候補**に合わせるパッチ
  (「相手を強くする方向のみ」の範囲内の**正しさ**修正。SRS+ の 6 番目は S2 ではない)。

## 制約(TBP 経由の S2 は部分適用)

`createBot` は S2-only フィールド(`b2bChain` / `garbageRows` / `dueThisLock` / `frame`)を
`SearchInput` に写さない(`fighter.ts` のコメント)。したがって:
- **有効**: 攻撃表(tetrio-s2)、コンボ、All-Mini+、180°、deathHeight 21、B2B +1、PC +5
- **attax の探索に効かない**: surge の「溜め」、GSB、margin multiplier
- レフェリーの**採点**は S2。attax の**読み**は guideline 相当。
- **完全な S2 attax(`tools/s2-lab/` 経由)との対戦は我々のハーネスでは不可能**。
  「TBP 経由の S2 attax」が比較対象で、主張もそこに限定する。

## エビデンス

- `summary.json` / `summary.md` / `audit/*.json` / `replays/*.jsonl` / `logs/*-opponent.jsonl`
- attax の decision log は今回の 10 ゲームでは未取得(`ATTAX_TBP_LOG` 未設定。`--tbp-*` 経路は
  `decision_log` を渡さない)。forfeit の再現時に `ATTAX_TBP_LOG` を設定して取る。
