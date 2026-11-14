# potential 項の仕様 — `quad_ready` / `tsd_ready` / `single_penalty` / 待ちの予算

**目的**: 探索の地平(depth 2〜4)では見えない「準備の価値」を評価で補う。**しきい値と定義を確定する**。
**前提**: `docs/s2-spec.md` §6(採点)、`docs/s2-combo-strategy.md`(attack/line)、`docs/s2-burst-defense.md`(危険の定義)
**日付**: 2026-11-16

---

## 0. 結論

| 項 | 判定 |
|---|---|
| `tsd_ready` | **既存の `t_slot_lines() >= 1` で足りる**。`t_slot_exact` で検証できる |
| `quad_ready` | **新規に必要**。定義を §2 に確定 |
| `single_penalty` | **不要。もっと良い方法がある**(§3) |
| 待ちの予算 | **`wait_pieces <= 21 - H - pending`**(§4) |

**最重要**: **`combo` / `combo_potential` / `single_penalty` を「実際の攻撃量」1 項に置き換えるべき**(§3)。single の攻撃は 0 なので、**順序が自動的に正しくなる**。

---

## 1. `tsd_ready` — 既存の `t_slot` で足りる

**現状の実装**(`crates/bot/src/eval.rs`):

```rust
pub fn t_slot_lines(board: &Board) -> u8            // 0/1/2(nub-down の T スロット)
pub fn t_slot_cutout(board, available, weights)     // T を最大 2 個使う想定
pub fn t_slot_exact(board: &Board) -> u8            // movegen + エンジン分類の参照実装
pub t_slot: [f64; 3]                                // [0.0, 20.0, 45.0]
```

**判定: 十分。** `t_slot_lines() >= 1` が `tsd_ready`、`>= 2` が「2 連続 TSD 可能」。

**1 点だけ要求**: **`t_slot_lines` と `t_slot_exact` の一致率を測ってください。** 安いスキャンは nub-down のみを見るので、**取りこぼしがある**。一致率が 80% を切るなら、`tsd_ready` の価値が過小評価されます。

**TSD の形(参考)**: 3 列幅の溝 + 屋根(両脇が 1 段高い)+ 溝の深さ 2。T を `Rotation::Two`(nub-down)で落とす。

---

## 2. `quad_ready` — 新規。定義を確定

### 2.1 定義

**「縦 I が 4 行入る、蓋のない 1 列の穴」**。

```
quad_ready(board) = 1 if ∃ column c:
    - h_c + 4 <= 21                                    # 4 行入る余地
    - 行 h_c .. h_c+3 の列 c がすべて空                 # 縦に空いている
    - 行 h_c+4 の列 c が空、または h_c+4 >= 21          # 蓋が無い(上に何も無い)
    - 行 h_c .. h_c+3 の列 c-1 と c+1 が埋まっている     # 両脇が埋まっている(溝である)
```

- `h_c` = 列 c の高さ(最も低い空セルの行)
- **両脇が埋まっている**ことを要求するのは、**平らな盤面の「空き列」を quad と誤認しない**ため。両脇が空なら、そこは「穴」ではなく「盤面の端」

### 2.2 「蓋があると価値ゼロ」の判定

| 状態 | 判定 |
|---|---|
| 行 `h_c+4` の列 c が**埋まっている** | **価値 0**(I が入れない) |
| 行 `h_c+4` の列 c が**空** | 価値あり |
| 行 `h_c+4` が盤面の外(`>= 21`) | 価値あり(蓋が無い) |
| 行 `h_c..h_c+3` のどこかが埋まっている | **価値 0**(4 行入らない) |
| 列 `c-1` または `c+1` が空 | **価値 0**(溝ではない) |

**「蓋」の判定は 1 セルだけ見ればよい**(列 c の `h_c+4`)。**斜めの屋根は考えない**(I は真下に落ちる)。

### 2.3 重み

`quad_ready` の価値 = **「あと 1 ピースで quad が撃てる」状態の価値**。

- quad の攻撃 = **5**(base 4 + B2B 1、combo 0)
- 撃つのに 1 ピース(I)
- **`quad_ready` の重み ≈ `W_attack × 5 / 1`** ではなく、**「I が来るまでの期待値」で割り引く**

**推奨**: `quad_ready = W_attack × 5 × P(I arrives)`

- `P(I arrives)` は 7-bag で **1/7 ≈ 0.14**(次の 1 ピース)
- ただし hold があるので、**I を hold していれば 1.0**
- **推奨値: `quad_ready ≈ W_attack × 5 × 0.3`**(hold 込みの実効確率)。`W_attack = 28` なら **42**

### 2.4 注意: `quad_ready` は `t_slot` と競合する

**同じ盤面で quad と TSD の両方は作れない**ことが多い(TSD は 3 列幅、quad は 1 列)。**両方の項が同時に高いと、どちらも完成しない**。

**推奨**: `max(quad_ready, tsd_ready)` ではなく、**両方を足すが、片方だけが高い状態を選びやすくする**(現状の足し算でよい。探索が選ぶ)。

---

## 3. `single_penalty` は不要 — **「実際の攻撃量」1 項に置き換える**

### 3.1 なぜ penalty ではダメか

**single の攻撃は 0**(`BASE_ATTACK[none][1] = 0`)。したがって:

- **single の価値は「高さが 1 減る」ことだけ**
- ところが `combo_potential`(「1 セルで埋まる行」の数)が **single を直接報酬している**
- `combo` 項も single で維持できるので **single を報酬する**

**penalty を足すと、3 つの項が押し合って、重みの調整でしか釣り合わない。** それは今まで失敗してきた方法。

### 3.2 置き換え案

**`combo` + `combo_potential` + `single_penalty` を、次の 1 項に置き換える**:

```
placement_value = W_attack × (この設置が実際に送る攻撃量)
```

**この値は `compute_attack` で厳密に計算できる**(`clear.lines` / `spin` / `combo_before` / `b2b_before` / `garbage_cleared` / `perfect_clear` が全部ある)。近似ではない。

**順序が自動的に正しくなる**:

| 設置 | combo 0 | combo 2 | combo 5 |
|---|---|---|---|
| **single** | **0** | 1 | 1 |
| double | 1 | 1 | 2 |
| triple | 2 | 3 | 4 |
| quad | 5 | 7 | 10 |
| TSD | 5 | 7 | 10 |

**single が常に最下位**。penalty も combo 項も要らない。

### 3.3 `combo_potential` の扱い

**「1 セルで埋まる行」= single を作る形**なので、**攻撃の指標としては有害**。

**残すとすれば「防御の指標」として**:
- `combo_potential >= 1` は「**次の 1 手で 1 ライン消せる**」= **ゴミの出現を止められる**(`docs/s2-burst-defense.md` §1.3)
- **`H + pending >= 21` のときだけ価値がある**
- **それ以外では 0 にする**

```rust
let defensive = if max_height + incoming >= 21 { combo_potential } else { 0.0 };
```

**これで「常に single を作る」バイアスが消える。**

### 3.4 `single_penalty` の値(どうしても入れる場合)

**`W_attack × 1`**。理由: single の攻撃は 0 で、double は 1。**「double との差」= 1 攻撃分**を penalty にすれば、順序が揃う。

ただし **§3.2 の置き換えを強く推奨**。penalty は「他の項との押し合い」を残す。

---

## 4. 準備と攻撃のトレードオフ — 待ちの予算

### 4.1 何ピース待てるか

**待つ = 消さずに積む = 高さが上がる**。

- 1 ピース = 4 セル。**最悪ケースでは最大高さが 1 上がる**(一番高い列に積む)
- 平均では 0.4 上がる(4 セル / 10 列)

**待てるピース数**:

```
wait_pieces_max = 21 - H - pending          # 最悪ケース(1 ピースで +1)
wait_pieces_avg = (21 - H - pending) / 0.4  # 平均(1 ピースで +0.4)
```

| H | pending | `wait_max` | `wait_avg` |
|---|---|---|---|
| 5 | 0 | **16** | 40 |
| 10 | 0 | **11** | 27 |
| 15 | 0 | **6** | 15 |
| 15 | 5 | **1** | 2 |
| 18 | 3 | **0** | 0 |

### 4.2 推奨ルール

**`quad_ready` / `tsd_ready` の価値に、待ちの余裕を掛ける**:

```rust
let budget = (21 - max_height - incoming).max(0);   // 最悪ケースの待ちピース数
let readiness_scale = (budget as f64 / 4.0).min(1.0);  // 4 ピース待てれば満額
value += readiness_scale * (W_quad_ready * quad_ready + W_tsd_ready * tsd_ready);
```

- **`budget >= 4`** なら満額(準備する余裕がある)
- **`budget = 0`** なら 0(準備している場合ではない)
- **`budget 1〜3`** は比例

**根拠**: `budget` は「今 0 ラインを置いても死なない回数」。**準備は 2〜3 ピースかかる**ので、**4 ピース待てなければ準備の期待値がマイナス**。

### 4.3 なぜ「最悪ケース」を使うか

平均(0.4/ピース)を使うと、**一番高い列に積み続けたときに死ぬ**。**`wait_max` は保守側**で、`docs/s2-burst-defense.md` の危険判定(`H + pending >= 21`)と同じ基準。

---

## 5. 実装への要求(まとめ)

| # | 要求 | 受け入れ |
|---|---|---|
| 1 | **`placement_value = W_attack × 実際の攻撃量`** に置き換え | single が最下位になる |
| 2 | **`combo_potential` は `H + pending >= 21` のときだけ有効** | 「常に single」バイアスが消える |
| 3 | **`quad_ready` を実装**(§2 の定義) | 盤面に I ウェルがあるとき値が上がる |
| 4 | **`tsd_ready` は既存の `t_slot_lines`** | `t_slot_exact` との一致率 >= 80% |
| 5 | **準備項に `readiness_scale` を掛ける**(§4.2) | `budget = 0` で準備項が 0 |
| 6 | **`single_penalty` は入れない**(§3.2 の置き換えで不要) | — |

---

## 6. 限界

- **`quad_ready` の重み 42 は概算**。`P(I arrives)` の実効値を実測していない
- **`t_slot_lines` の一致率を測っていない**(要求として書いた)
- **`readiness_scale` の 4 ピースは経験値**。実測で調整が必要
- **「同じ盤面で quad と TSD が競合する」問題**を定量化していない

---

## 7. 改訂履歴

| 版 | 日付 | 変更 |
|---|---|---|
| v1 | 2026-11-16 | 初版。`tsd_ready` は既存の `t_slot_lines` で足りる(一致率の測定を要求)。`quad_ready` の定義(4 行・1 列・蓋なし・両脇充填)。**`single_penalty` は不要**で、`combo` + `combo_potential` + penalty を「実際の攻撃量」1 項に置き換えるべき。待ちの予算 `21 - H - pending` と `readiness_scale`。 |
