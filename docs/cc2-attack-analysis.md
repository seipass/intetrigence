# CC2 の 2 敗の分析と `attack/line` を上げる構造施策

対象: `scripts/evidence/cc2-40-base-w88`(risk 8/8 + width 88、40 ゲーム、38 勝 2 敗、
TurnCap 0、no-contest 0、`over100` 0、`load_note` なし)。負けは 9008-B と 9017-A。

## 1. 何が負けているか

| seed | 勝敗 | turns | 我々 lines | CC2 lines | 我々 attack | CC2 attack | 我々 a/line | CC2 a/line | 我々 final y | CC2 final y |
|---|---|---|---|---|---|---|---|---|---|---|
| 9008-B | 負け | 178 | 106 | 94 | 103 | 138 | **0.97** | 1.47 | **18** | 5 |
| 9017-A | 負け | 272 | 173 | 172 | 207 | 228 | **1.20** | 1.33 | **21** | 4 |
| 9001-A | 勝ち | 156 | 103 | 84 | 122 | 123 | 1.18 | 1.46 | 2 | 16 |
| 9010-A | 勝ち | 143 | 101 | 76 | 95 | 103 | 0.94 | 1.36 | 4 | 21 |

**負けどちらも「我々が最終 height 18/21 で自滅、CC2 は 4/5」**。CC2 の攻撃で押し切られた
のではなく、**自分のスタックで死んでいる**。9008-B は net damage が −20 対 96、
9017-A は +60 対 +74 で、攻撃量そのものは接戦。

## 2. 攻撃の構成(clear の内訳)

replay の placements から数えた(0 ライン設置は除く)。

| seed | slot | 1 ライン | 2 | 3 | 4 | 1 ライン比率 | spin | B2B≥4 の turn | max B2B | surge 解放 |
|---|---|---|---|---|---|---|---|---|---|---|
| 9008-B | 我々 | 73 | 7 | 1 | **4** | 52% | 11 | **0** | 2 | **0** |
| 9008-B | CC2 | 12 | 19 | 0 | **11** | 76% | 18 | 16 | 7 | 10 |
| 9017-A | 我々 | 93 | 23 | 2 | **7** | 54% | 37 | 10 | 5 | 8 |
| 9017-A | CC2 | 14 | 32 | 6 | **19** | 74% | 22 | 22 | 7 | 10 |
| 9001-A | 我々 | 57 | 7 | 0 | **8** | 54% | 11 | 2 | 5 | 4 |
| 9010-A | 我々 | 50 | 9 | 3 | **6** | 52% | 6 | 0 | 2 | 0 |

CC2 も 1 ライン設置は多い(74–79%)。**差は quad の数(4–7 対 11–19)と B2B チェーン
(0–10 turn 対 7–22 turn)**。我々は「1 ラインで高さを下げる」を繰り返し、quad と B2B を
積まない。結果として `attack/line` が 0.97–1.20 に留まる。

## 3. 原因の候補(重みの構造)

`crates/bot/src/eval.rs` の既定値:

| 重み | 値 | 意味 |
|---|---|---|
| `defense` | **28.0** | 1 ライン設置が止める garbage 1 行の価値。pending 8 なら **224** |
| `attack` | 28.0 | attack 1 の価値。quad は 5 × 28 = 140 |
| `quad_ready` | 30.0 | 4 深い well の価値 |
| `b2b_break` | −20.0 | B2B を切る罰 |
| `wasted_t` | −18.0 | T を spin なしで使う罰 |
| `surge_charge` / `surge_exponent` | 1.5 / 1.0 | charge の価値 |
| `clear[none][1]` | **0.0** | 1 ラインの追加価値(attack 0 なので実質 0) |
| `discount` | 0.97 | 1 ply あたりの割引 |

**仮説**: `defense` が「いますぐ 1 ラインで garbage を止める」を **224** と評価する一方、
quad は「5 attack × 28 = 140 + B2B 1」で、しかも 4 行分の準備が要る。`discount 0.97` が
1 ply ごとに 3% 割り引くので、**準備の要る quad は 1 ラインに負ける**。これが
「1 ライン比率 52–54%、quad 4–7」の直接の原因。

## 4. 提案する A/B(`--weights` で試せるもの)

`--weights key=value,...` は `apply_weight_overrides`(`crates/arena/src/harness.rs:202`)
が受け付ける。以下は**コード変更なしで試せる**。

| 案 | overrides | 狙い |
|---|---|---|
| A: quad を立てる | `quad_ready=45,b2b_break=-32,wasted_t=-30` | well と B2B を保持 |
| B: 1 ラインの相対価値を下げる | `defense=20,quad_ready=40` | 単発 clear の魅力を下げる |
| C: surge を溜める | `surge_charge=2.5,surge_exponent=1.4` | 4–7 ラインで吐かず 9–13 を狙う |
| D: 複合 | `quad_ready=45,defense=22,b2b_break=-32,surge_charge=2.2,surge_exponent=1.3` | A+B+C |

**測定**: 20 ゲーム(seeds 9001–9010 × A,B)、`--deterministic --depth 8 --width 88`、
`--ms 100 --threads 8`、CC2(commit `ed8b193`)、ロック取得、`load_note` が None の
実行だけを採用。主指標は `attack/line`、quad 数、B2B≥4 turn、surge 解放、崩壊数。

**コード変更が要るもの( bot-dev )**:
- `clear[none][1]` を 0 → 小さな負値(例 −2)。1 ラインの「高さを下げる」価値は
  `max_height`/`height_sq` が既に払っているので、`clear` で二重に払わない。
- `defense` の上限(clamp)。pending 8 の 224 は quad の 140 を上回る。`defense` を
  `min(pending, 4)` 行に制限する、または `defense` の係数を下げる。
- `quad_ready` を「well が空いている深さ」に比例させる(現在は 0/1 の二値か)。

## 5. 注意

- 9008-B / 9017-A の負けは **我々の自滅**(final y 18/21)。攻撃量を上げる前に、
  **高さ管理**(`risk_base`/`risk_span`)を壊さないこと。risk 8/8 は 38-2 の構成。
- `attack/line` を上げる A/B は **崩壊数と TurnCap を同時に見る**。a/line が上がっても
  崩壊が増えればゲート(負け 0)に近づかない。
- 評価 seeds は未焼。A/B は tuning seeds(9001–9020)のみ。
