# TETR.IO Season 2 ルール仕様(凍結版)

**対象**: intetrigence の S2 ルールエンジン(`crates/core`)・レフェリー(`crates/arena`)・attax 接続の共通仕様
**プロファイル**: TETRA LEAGUE デフォルト相当(S2 本命プロファイル)
**凍結日**: 2026-11-13
**状態**: 凍結(v1)。以降の変更は「改訂履歴」に追記し、`crates/core` のバージョンと同時に上げる。

この文書は PLAN.md §3.1 の表を一次ソースで裏取りし、数値定数として固定する。**ソース間で決着しなかった項目は §11 で我々が決定(暫定自決)として確定**しており、各項目に「決定値・根拠・差し替え条件」を書いてある。作者確認は非ブロッキング(AGPL 利用許可のみ取得済み・公開 I/F のみ使用)なので、**§11 の決定が実装の正**である。

> **マッチの運用(ターン・席・提出・replay・監査)は [`docs/s2-referee-protocol.md`](s2-referee-protocol.md) が定める。** 本書はルールの数値と判定だけを持ち、レフェリーの手順は持たない。片方を変えるときはもう片方への影響を確認すること。

---

## 0. 読み方

### 0.1 確度の表記

各定数には確度を付ける。

| 記号 | 意味 |
|---|---|
| **[確定]** | 一次ソース(公式パッチノート/公式 wiki)と、独立した 2 つ以上の実装が一致 |
| **[実装一致]** | 一次ソースに数値が無く、TETR.IO の実挙動に合わせて作られた独立実装 2 つ以上が一致 |
| **[単一実装]** | 実装 1 つだけが出典。数値は採択し、§11 に差し替え条件を書く |
| **[自決]** | ソース間で矛盾、または数値ソースが無い。§11 で我々が決定(暫定自決)として確定した |

### 0.2 座標系

この文書では **盤面座標 (x, y)** を使う。

- `x`: 0 = 左端、9 = 右端
- `y`: 0 = 最下段(可視領域の底)、19 = 可視領域の最上段、20〜39 = 内部バッファ
- 回転 kick のオフセットは **(Δx, Δy)**、`Δy > 0` が**上方向**(TETR.IO / guideline の慣習)

> 実装上の注意: teto(`@haelp/teto`)の kick データは `Δy > 0` が**下方向**で格納されている。取り込み時に符号反転が必須。本書の表はすべて「上方向を正」に正規化済み。

#### 0.2.1 座標表を転記するときの規約

S7(teto)由来の座標表を本書やコードに写すときは、**必ず次の 3 つをセットで**行う。v1.3 で §4.3 の front corner 表を符号反転せずに転記し、**到達可能な TSD がすべて mini になる**バグを作った反省から規約化した。

1. **反転する**: S7 の `Δy` は下方向が正。本書の `Δy` は上方向が正。`Δy_new = -Δy_s7`。
2. **規約を書く**: その表が「どちら向きが正か」を表の直近に 1 行書く。書かないと次の読者が同じ罠を踏む。
3. **反転を検出するテストを書く**: 表を反転させると**落ちる**テストを入れる。座標表は「正しく見える」ので、通常のテストでは誤りを検出できない。

> 転記の正しさは「S7 の生データを機械的に反転したものと一致するか」でスクリプト照合するのが早い。v1.3.1 の §4.3 修正はこの方法で 4/4 隅の一致を確認した。

### 0.3 出典 ID

| ID | 出典 | 取得日 | 備考 |
|---|---|---|---|
| **S1** | tetrio.wiki.gg — [Mechanics](https://tetrio.wiki.gg/wiki/Mechanics) (`oldid=1673`) | 2026-11-13 | Combo Multiplier / B2B Charging / All-Mini+ / Opener Phase / GSB |
| **S2** | tetrio.wiki.gg — [Spins](https://tetrio.wiki.gg/wiki/Spins) (`oldid=1593`) | 2026-11-13 | immobile / 2-corner / 3-corner / 5th kick |
| **S3** | tetrio.wiki.gg — [TETRA LEAGUE](https://tetrio.wiki.gg/wiki/TETRA_LEAGUE) (`oldid=1770`) | 2026-11-13 | S2 変更点の一覧 |
| **S4** | tetr.io 公式パッチノート `https://tetr.io/about/patchnotes/notes.json` | 2026-11-13 | BETA 1.0.0〜1.7.8 の全項目。以下 `PN <ver>` |
| **S5** | tetris.wiki — [TETR.IO](https://tetris.wiki/TETR.IO) | 2026-11-13 | Multiplier / B2B / All-Mini+ / QP 差分 |
| **S6** | 61bi-234469/s2-bot-lab `79505ca93bd15f7bcbd58724cb2a6ba312e09434` (MIT) | 2026-11-13 | TETR.IO クライアントから観測した**end option 全 173 キー** (`rulesets/tetrio-s2-v19-beta-1-5-0-observed.json`)、golden fixture、Rust 実装 |
| **S7** | halp1/triangle `aa0698ab8b4299b85c5f87e9b54793e1b7d9d380` = `@haelp/teto` 4.2.8 (MIT) | 2026-11-13 | TETR.IO クライアントの再実装。kick 表・spin 判定・garbage queue の数値ソース |
| **S8** | chouhy/MisaMino-Tetrio `df36ed3875ad853e9bb8ac869096128869493c09` | 2026-11-13 | TETR.IO 攻撃テーブル + **garbage cap = 8** の独立裏取り |
| **S9** | chouhy/cold-clear-2 `b20a92b0ed3230dd910d0674f7a09c552a34dd46` | 2026-11-13 | all-spin 処理の参考実装(PLAN §3.2 指定) |
| **S10** | 61び「TETR.IO Terminology and System Explanation Summary [Season 2 Compatible]」note.com | 2026-11-13 | 日本語圏の二次解説(数値の裏取りには使わない) |
| **S11** | MochBot/fusion (Rust, TETR.IO replay coaching engine, S2 attack) | 2026-11-13 | `rules.rs` の `SPAWN_ROW = 21`、S2 attack 実装。spawn 位置と攻撃の独立裏取り |

**S6 の位置づけ**: S6 の `rulesets/tetrio-s2-v19-beta-1-5-0-observed.json` は、TETR.IO クライアントが実際に送る **end option ブロックをそのまま記録したもの**。本書では「TETR.IO のデフォルト値」の一次証拠として扱う。個々のキーの**意味**は S1/S4 と S7 の実装で解釈する。

---

## 1. PLAN.md §3.1 の裏取り表

PLAN §3.1 の全 15 行を 1 行ずつ潰す。

| # | PLAN §3.1 の項目 | 凍結値 | 出典 | 確度 |
|---|---|---|---|---|
| 1 | 攻撃 base | Single 0 / Double 1 / Triple 2 / Quad 4 / TSM 0 / TSS 2 / TSD 4 / TST 6 / mini Double 1(完全表は §6.1) | S7 `damageCalc`、S8、S10 | [実装一致] |
| 2 | コンボ | `base × (1 + 0.25 × combo)`、切り捨て。`base = 0` は `ln(1 + 1.25 × combo)`(combo ≥ 2)。丸めは **DOWN** | S1、S5、S7 | [確定] |
| 3 | B2B | **B2B Charging**: difficult clear 毎に **+1 フラット**。表示 B2B ≥ 4 で surge 溜め、非 difficult clear で全放出(3 分割) | S1、S4 `PN BETA 1.1.0`/`PN BETA 1.2.0`、S7、S6 golden | [確定] |
| 4 | All-Spin | **All-Mini+**: 非 T は immobile 判定で mini。T は 3-corner / 2-corner + 5th kick。B2B は維持・加算 | S1、S2、S4 `PN BETA 1.5.0`、S7 `spinbonusRules["all-mini+"]`、S6 (`spinbonuses: "all-mini+"`) | [確定] |
| 5 | パーフェクトクリア | **+5 ゴミ、+1 B2B**(+2 ではない。§6.5 で決着) | S4 `PN BETA 1.2.0`、S6 (`allclear_garbage: 5`, `allclear_b2b: 1`)、S6 golden `perfect-clear-adds-b2b` | [確定] |
| 6 | ゴミ込みボーナス | Garbage Special Bonus: ゴミを 1 行以上消し、かつ (spin または 4 行消し) のとき **+1 フラット** | S1、S4 `PN BETA 1.3.0`、S7、S6 (`garbagespecialbonus: true`) | [確定] |
| 7 | Opener Phase | 設置 1〜14 個目の間、`pending ≥ sent` なら相殺量 **×2** | S1、S4 `PN BETA 1.1.0`/`1.2.0`、S7 `GarbageQueue.cancel`、S6 (`openerphase: 14`) | [確定] |
| 8 | ゴミ | change-on-attack(**TL は change = 1.0 = 毎攻撃で必ず列変更**。×2.5 は QUICK PLAY の話 → §7.6)。キュー + 遅延挿入。cap 8 行/ロック、遅延 20 frame、相殺優先 | S6 (option 全値)、S7、S8 | [確定] |
| 9 | 回転 | **SRS+**(I の kick が左右反転)+ **180°**(全ピース、5 テスト) | S1、S5、S7 `kicks["SRS+"]` | [実装一致] |
| 10 | 盤面 | 10×20 可視、内部 40 行。**lock-out は無効**、block-out のみ。Clutch Clear 有効 | S6 (`boardwidth 10`, `boardheight 20`, `boardbuffer 20`, `nolockout: true`, `clutch: true`)、S1、S7 | [確定] |
| 11 | ランダマイザー | 7-bag、5 preview + hold | S6 (`bagtype: "7-bag"`, `nextcount: 5`, `display_hold: true`) | [確定] |
| 12 | ペース | 5 PPS(ペアドターン)、1000 turn cap | プロジェクト決定(PLAN §3.1)。TETR.IO のルールではなく**マッチプロトコル** | [確定(プロトコル)] |
| 13 | 思考時間 | intetrigence `--ms 100`、attax は S2 版デフォルト | プロトコル | [確定(プロトコル)] |
| 14 | シート | 各 seed を両シートでプレイ | プロトコル | [確定(プロトコル)] |
| 15 | シード | 両プロジェクト未使用の held-out seeds | プロトコル | [確定(プロトコル)] |

**PLAN の記述に対する訂正(2 点)**

1. §3.1「ゴミ」行の「攻撃間でばらけ率 ×2.5」は **QUICK PLAY 専用**の記述(S5 QP 節)。TETRA LEAGUE は `messiness_change = 1.0` で、**攻撃が切り替わるたびに 100% 列を引き直す**。PLAN の文言は「攻撃内は同列、攻撃間は必ず別列(ただし乱数で同列になり得る)」に読み替える。§7.6 参照。
2. §3.1「パーフェクトクリア」行の「+1 / +2 の表記揺れ」は **解消済み**。+2 は S2 プレシーズン(`PN BETA 1.1.0`)の値で、S2 本開幕(`PN BETA 1.2.0`)で +5 ゴミ / +1 B2B に変更された。S1 wiki の「All Clear は B2B を 2 得る」記述(S1/S3)が残っているだけ。

---

## 2. 盤面・ピース供給

### 2.1 盤面

| 定数 | 値 | 出典 | 確度 |
|---|---|---|---|
| 可視幅 `W` | 10 | S6 `boardwidth: 10` | [確定] |
| 可視高 `H` | 20 | S6 `boardheight: 20` | [確定] |
| 内部バッファ | 20 行(内部 40 行) | S6 `boardbuffer: 20` | [確定] |
| 内部総高 `FULL_H` | 40 | 上記の和 | [確定] |
| lock-out | **無効** | S6 `nolockout: true`、S1「lock out was disabled by default」 | [確定] |
| Clutch Clear | **有効** | S6 `clutch: true`、S4 `PN BETA 1.5.0` | [確定] |
| Lock delay | 500 ms(30 frame) | S1、S6 `locktime: 30` | [確定] |
| Lock reset 上限 | 15 | S6 `lockresets: 15` | [確定] |
| 重力 | `g: 0.02` G(初期)、`gincrease: 0.0035`/s、`gmargin: 7200` frame | S6 | [確定] |
| ARE | 0 | S6 `are: 0`, `lineclear_are: 0` | [確定] |

**レフェリー上の扱い**: 本マッチはペアドターン・5 PPS の離散イベント系なので、重力・lock delay・ARE は**勝敗に影響しない**。レフェリーは「1 turn = 1 ピース設置」として時間を進め、`frame` は `turn × 12`(5 PPS = 12 frame/piece)で進める。frame が効くのは **garbage multiplier の margin time と、ゴミパケットの 20 frame 遅延**だけである(§7.5)。

### 2.2 spawn

```
spawn_x(piece) = floor(W/2 - matrix_w/2)
spawn_y        = H + 2.04      # TETR.IO 内部座標(y 上向き、原点は可視最下段)
spawn_rotation = 0 (North)
```

- `matrix_w`: I = 4、O = 2、それ以外 = 3
- したがって **I・T・S・Z・J・L は x = 3、O は x = 4**
- 出典: S7 `Tetromino.constructor`(`Math.floor(boardWidth / 2 - matrix.w / 2)`, `boardHeight + 2.04`)、S6 (`spawn_rotation` 空 = 全ピース 0)

#### 2.2.1 spawn セルの絶対座標(実装はこれを直接使う)

`y = H + 2.04` の `.04` は sub-row offset で、**設置ベースのレフェリーには不要**(占有行は `floor` で決まる)。重要なのは **最下段セルが row 21** であること。可視領域は row 0〜19 なので、spawn したピースは**完全にバッファ内(不可視)**にある。

| ピース | row 21(下段) | row 22(上段) |
|---|---|---|
| I | x = 3,4,5,6 | — |
| O | x = 4,5 | x = 4,5 |
| T | x = 3,4,5 | x = 4 |
| S | x = 3,4 | x = 4,5 |
| Z | x = 4,5 | x = 3,4 |
| J | x = 3,4,5 | x = 3 |
| L | x = 3,4,5 | x = 5 |

**全ピース共通: 最下段セルは row 21**(= 可視最上段 19 の 2 行上)。

- 出典: S7 `Tetromino`(`location[1] = boardHeight + 2.04`、セルは `y - box_row` で row 21.04 / 22.04)、S11 `rules.rs`(`SPAWN_ROW = 21`、pivot 基準・row 0 = 床)
- 確度: [実装一致](TETR.IO 専用エンジン 2 つが一致)
- **帰結**: 可視 20 行(row 0〜19)を**すべて埋めても spawn できる**。block-out するのはスタックが **row 21** に達したとき。

> **実装監査メモ(2026-11-13)**: `crates/core/src/piece.rs` の `spawn_position` は最下段セルを **row 20** に置いている(I = (3,18)、O = (4,20)、他 = (3,19))。本書は **row 21** を要求する。**1 行低いため、block-out が 1 行早く起きる**(可視最終行が埋まった時点で死ぬ)。トップアウト争いの局面で勝敗に直結するので修正が必要。

### 2.3 block-out / lock-out / Clutch

- **block-out**: spawn 直後のピースが既存ブロックと重なる(または盤外)→ 死亡。
- **lock-out**: S2 TL デフォルトでは**無効**(`nolockout: true`)。可視領域より上でロックしても死なない。次の spawn が block-out するまで続く。
- **Clutch Clear**: 直前の設置でライン消去が発生していた場合、spawn 位置が重なるとピースを 1 行ずつ上へ押し上げ、`y < FULL_H` の範囲で合法な位置を探す。見つかれば生存、見つからなければ block-out。
  - 出典: S7 `#considerBlockout`、S4 `PN BETA 1.5.0`「If you clear line(s) and would immediately top out afterwards, your next piece will be pushed up」

**レフェリー実装要求**: 死亡判定は「spawn 位置の衝突」+「Clutch の押し上げ探索」の 2 段。lock-out 判定を入れてはいけない。

### 2.4 7-bag

- 7 種を 1 袋としてシャッフルし、袋が尽きたら新しい袋を引く。**同じ袋に同じ種は 2 回入らない**。
- シャッフルは Fisher–Yates(降順)、乱数は MINSTD(§7.7 と同じ RNG)。
- 出典: S7 `Bag7`/`RNG.shuffleArray`、S6 `bagtype: "7-bag"`

```
shuffle(a):  for i = len-1 downto 1:
               r = floor(next_float() * (i+1)); swap(a[i], a[r])
```

**重要**: 本マッチでは**両者同一 seed**を使う(PLAN §3.1)。seed → 乱数系列の対応は §7.7 の MINSTD を共通に使い、ピース袋とゴミ列で**別々の RNG インスタンス**を持つ(TETR.IO と同じ。混ぜない)。

### 2.5 hold

- hold は 1 ピースにつき 1 回まで。設置(ロック)すると再び使える。
- hold が空のときは next の先頭を持ち、next から 1 つ補充する。
- hold は **opener phase のピースカウントを進めない**(§7.4)。
- 出典: S6 (`display_hold: true`, `infinite_hold: false`)、S7 `hold()`

### 2.6 preview

- **5 preview**。`nextcount: 5`(S6)。
- 探索・評価は 5 preview + hold を完全に見てよい(PLAN §2 の attax 制約との差分)。

---

## 3. 回転系

### 3.1 SRS+(既定)

SRS+ は「SRS の I の kick を左右対称にしたもの」。J/L/S/T/Z は SRS と同一。

#### 3.1.1 J, L, S, T, Z(5 テスト)

`(Δx, Δy)`、`Δy > 0` が上。テストは**この順**に試す。

| 遷移 | T1 | T2 | T3 | T4 | T5 |
|---|---|---|---|---|---|
| 0 → R | (0,0) | (-1,0) | (-1,+1) | (0,-2) | (-1,-2) |
| R → 0 | (0,0) | (+1,0) | (+1,-1) | (0,+2) | (+1,+2) |
| R → 2 | (0,0) | (+1,0) | (+1,-1) | (0,+2) | (+1,+2) |
| 2 → R | (0,0) | (-1,0) | (-1,+1) | (0,-2) | (-1,-2) |
| 2 → L | (0,0) | (+1,0) | (+1,+1) | (0,-2) | (+1,-2) |
| L → 2 | (0,0) | (-1,0) | (-1,-1) | (0,+2) | (-1,+2) |
| L → 0 | (0,0) | (-1,0) | (-1,-1) | (0,+2) | (-1,+2) |
| 0 → L | (0,0) | (+1,0) | (+1,+1) | (0,-2) | (+1,-2) |

出典: S7 `kicks["SRS+"].kicks`(符号正規化済み)。S1「SRS+ は I の kick のみ変更」と整合。

#### 3.1.2 I(5 テスト)

| 遷移 | T1 | T2 | T3 | T4 | T5 |
|---|---|---|---|---|---|
| 0 → R | (0,0) | **(+1,0)** | **(-2,0)** | (-2,-1) | (+1,+2) |
| R → 0 | (0,0) | **(-1,0)** | **(+2,0)** | (-1,-2) | (+2,+1) |
| R → 2 | (0,0) | (-1,0) | (+2,0) | (-1,+2) | (+2,-1) |
| 2 → R | (0,0) | **(-2,0)** | **(+1,0)** | (-2,+1) | (+1,-2) |
| 2 → L | (0,0) | (+2,0) | (-1,0) | (+2,+1) | (-1,-2) |
| L → 2 | (0,0) | **(+1,0)** | **(-2,0)** | (+1,+2) | (-2,-1) |
| L → 0 | (0,0) | (+1,0) | (-2,0) | (+1,-2) | (-2,+1) |
| 0 → L | (0,0) | (-1,0) | (+2,0) | **(+2,-1)** | **(-1,+2)** |

**太字 = 素の SRS との差分**(SRS の I 表は 0→R が `(-2,0),(+1,0),(-2,-1),(+1,2)` 等)。差分は「左右反転」と「テスト順の入れ替え」の両方を含む。

- 出典: S7 `kicks["SRS+"].i_kicks`(符号正規化済み)
- 確度: オフセットの**集合**は [実装一致]、**テスト順**は [単一実装] → §11-U1 で決定(暫定)

> **実装監査メモ(2026-11-13)**: `crates/core/src/kick.rs` の `I_KICKS` は現状**素の SRS**(0→R = `(-2,0),(1,0),(-2,-1),(1,2)`)。SRS+ ではない。差し替えが必要。§12 参照。

#### 3.1.3 O

O は回転してもセルが変わらないため kick 不要(全テスト `(0,0)` と等価)。ただし 180° は §3.2 の一般表を使う(S7 は O に専用表を持たず汎用 180 表を引く)。

### 3.2 180° kick(全ピース・全方向)

180° 回転は `0↔2`, `R↔L` の 4 遷移。各遷移 **5 テスト**(`(0,0)` + 表の 4 件)。

#### 3.2.1 J, L, S, T, Z, O(汎用)

| 遷移 | T1 | T2 | T3 | T4 | T5 |
|---|---|---|---|---|---|
| 0 → 2 | (0,0) | (0,+1) | (+1,+1) | (-1,+1) | (+1,0) |
| 2 → 0 | (0,0) | (0,-1) | (-1,-1) | (+1,-1) | (-1,0) |
| R → L | (0,0) | (+1,0) | (+1,+2) | (+1,+1) | (0,+2) |
| L → R | (0,0) | (-1,0) | (-1,+2) | (-1,+1) | (0,+2) |

出典: S7 `kicks["SRS+"].kicks["02"/"20"/"13"/"31"]`(符号正規化済み)。S1「osk が Discord に pin した 180 kick 図」と整合。

> 注意: `0→2` と `2→0` は 5 テスト目まで含めて**上下反転の関係**にある。`R→L` と `L→R` は 5 テスト目が共通 `(0,+2)`。

#### 3.2.2 I

I の 180° は**各遷移 1 テストのみ**(`(0,0)` + 1 件)。

| 遷移 | T1 | T2 |
|---|---|---|
| 0 → 2 | (0,0) | (0,+1) |
| 2 → 0 | (0,0) | (0,-1) |
| R → L | (0,0) | (+1,0) |
| L → R | (0,0) | (-1,0) |

出典: S7 `kicks["SRS+"].i_kicks["02"/"20"/"13"/"31"]`
確度: [単一実装] → §11-U1 で決定(暫定)

### 3.3 5th kick(FIN / TST)判定

T スピンの「mini か full か」を決める際、**5 番目の kick を使ったか**が効く(§4.3)。判定は kick の**オフセット値**で行う:

```
fin_or_tst =
     (遷移 ∈ {2→L, 0→L} かつ kick = (+1,-2))
  or (遷移 ∈ {2→R, 0→R} かつ kick = (-1,-2))
```

- `(+1,-2)` は 0→L / 2→L の **5 番目**、`(-1,-2)` は 0→R / 2→R の **5 番目**に一致する。
- 出典: S7 `#isTSpinKick`
- 意味: この 2 つは「TST / FIN 用の特殊 kick」で、これを使った T スピンは **front corner が 1 個でも full spin** として扱う(S2)。

---

## 4. スピン判定(All-Mini+)

`spinbonuses = "all-mini+"`(S6)。判定は 3 系統。

### 4.1 前提: 「最後の操作が回転」

スピン判定は**最後の移動が回転**であることが前提。移動・落下が後に入ると spin は無効。設置ベースのレフェリーでは「提出された placement が回転を伴うか」を movegen 側で保証する(`crates/core` の movegen が回転由来の配置のみ spin 候補にする)。

### 4.2 非 T ピース: immobile 判定 → 常に mini

```
immobile(p) =
   !legal(p at x-1, y)  かつ
   !legal(p at x+1, y)  かつ
   !legal(p at x,   y+1) かつ   # 上方向(overhang)
   !legal(p at x,   y-1)
```

実際には「接地済み」なので下方向チェックは常に false、左右はライン消去に寄与しないため、**本質は上方向(overhang)チェック**。

- 条件を満たせば **mini**(非 T に full spin は存在しない)
- 満たさなければ spin なし
- 出典: S2「Immobile」節、S7 `Tetromino.isAllSpinPosition`、S4 `PN BETA 1.2.0`「All-Spins (immobile spins with pieces other than T) will be detected as a Mini」

**mini の攻撃値**(§6.1): mini single = 0、mini double = 1、mini triple = 2、mini quad = 10。

> 直感に反する点: **非 T の mini single は 0 ライン**。ただし B2B は加算・維持されるので、B2B 中の Z-Spin Single は +1(フラット B2B 分)を送る。S10 の記述(ZSS: B2B 無し 0 / B2B 有り 1)と一致。

### 4.3 T ピース: 3-corner / 2-corner + 5th kick

T のみ corner ベースの判定を使う。

```
1. 最後の操作が回転でなければ spin なし
2. T の 4 隅(回転状態ごとに定義)のうち、埋まっている数を corners とする
   - 盤外(左右の壁・底)は「埋まっている」とみなす
3. corners < 3 → spin なし
4. front_corners = 「T が向いている 2 隅」のうち埋まっている数
5. front_corners == 2 → full spin
   front_corners != 2 → mini
   ただし fin_or_tst(§3.3)が真なら full spin
```

- 出典: S2「2-corner and 3-corner」節、S7 `#detectSpinFromCorners`
- 4 隅の座標は回転状態ごとに固定(S7 `cornerTable.t`)。T の 4 状態は同じ 4 隅 `(-1,-1), (+1,-1), (+1,+1), (-1,+1)` を使い、各隅に「front になり得る回転 2 つ」を割り当てる。
- **基準点**: 隅の座標は **T の回転中心**(3×3 ボックスの中心 = ボックス左下 + (1,1))からの相対座標。実装が `(x, y)` にボックス左下を受けるなら隅は `(x+1+Δx, y+1+Δy)`、回転中心を受けるなら `(x+Δx, y+Δy)`。**どちらを受ける API かを明示しないと 1 マスずれる。**

| 隅 (Δx, Δy) | front になる回転 |
|---|---|
| (-1,-1) 左下 | 2, 3 |
| (+1,-1) 右下 | 1, 2 |
| (+1,+1) 右上 | 0, 1 |
| (-1,+1) 左上 | 3, 0 |

(回転: 0 = North, 1 = East/R, 2 = South/2, 3 = West/L。`Δy > 0` が**上**)

**回転ごとの front 2 隅**(上の表から導出。実装はこちらで照合すると間違えにくい):

| 回転 | nub の向き | front 2 隅 |
|---|---|---|
| 0 North | 上 | (-1,+1), (+1,+1) |
| 1 East/R | 右 | (+1,+1), (+1,-1) |
| 2 South | 下 | (+1,-1), (-1,-1) |
| 3 West/L | 左 | (-1,-1), (-1,+1) |

front 2 隅 = **nub が向いている側の 2 隅**。これが 3-corner 則の標準的な意味であり、`front == 2` で full spin になる。

> **符号の罠(重要)**: S7 の `cornerTable` は `Δy > 0` が**下方向**で格納されている(occupied 呼び出しが `y - dy - 1`、pivot が `(x+1, y-1)`)。本書は上方向を正に正規化済み。**S7 の生データをそのまま転記すると front が上下逆になり、到達可能な TSD がすべて mini になる**(front が T の上の空マスを指すため)。
>
> **検算**: 標準の TSD スロット(T が回転 2 で、nub の左右 2 マスが埋まっている)では front = 2 → full、base 4。壊れた表では front = 0 → mini、base 1 になり、TSD が 4 行ではなく 1 行になる。

- `PN BETA 1.5.0`:「Immobile T-Spins that don't fulfill the three-corner-rule now count as Minis」= All-Mini+ で **T も immobile 判定の mini を取れる**。実装上は「3-corner を満たさない T スピンでも、immobile なら mini」ではなく、**T の mini 判定は 2-corner ルールで行い、immobile は非 T 専用**と解釈する(S7 の実装がそうなっている)。§11-U2 で決定(暫定)。

### 4.4 B2B との関係

- **mini も difficult clear**(`spin != none` なら mini でも difficult)。→ B2B を維持し、+1 する。
- 出典: S1「Back-to-Back is amount of difficult line clears (Quad or higher, **Spin mini single or higher**, Spin single or higher)」、S3「All-spins now count towards B2B and are classified as a difficult clear」
- **0 ラインのスピン**(ライン消去なし)は B2B を維持も破壊もしない(§6.2 の `lines > 0` 条件)。

---

## 5. 用語: combo / b2b の内部表現

実装間で 0-based / 1-based が混在するので、本書の表現を固定する。

| 変数 | 定義 | 初期値 |
|---|---|---|
| `combo` | **この設置を行う前の連続消去数**(0 = 直前の設置が消去なし)。式に入るのはこの値。設置後は `combo + 1` になる | 0 |
| **`b2b_streak`** | **連続 difficult clear 数 `N`**。0 = B2B なし、1 = 1 回目、… | **0** |
| `b2b_disp` | 派生値。**表示 B2B 数** = `max(N - 1, 0)` | 0 |
| `lines` | その設置で消したライン数(0〜4) | — |
| `spin` | `none` / `mini` / `full` | — |
| `pc` | パーフェクトクリアか | — |
| `g_cleared` | その設置で消したゴミ行数 | — |

**状態変数は `b2b_streak`(= N)ただ 1 つ**。`b2b_disp` は表示と surge 量のための派生値で、状態としては持たない。

- 「表示 B2B x N」= `b2b_disp = N`。S1「The first difficult line clears are not counted. For example, when you clear 10 Quads, "B2B x9" will show.」→ 10 連続 Quad で `N = 10`, `b2b_disp = 9`。
- `combo` は S1 の式の `x` にそのまま入る(S1: `base*(1+0.25x)` where x is combo)。
- **具体例**: 1 回目の消去では `combo = 0`(乗算なし)。2 連続目では `combo = 1`(×1.25)。3 連続目では `combo = 2`(×1.5)。
- 実装間の対応: S11(fusion)は `-1` を「消去なし」のセンチネルに持ち、`combo_after = pre + 1` を式に入れる。S7(teto)も `stats.combo` が `-1` 始まりで、`max(stats.combo, 0)` を式に入れる。**本書の `combo` は両者の「式に入る値」と一致する**。

**遷移**(`lines > 0` のときのみ動く):

| 条件 | `b2b_streak` の遷移 |
|---|---|
| difficult clear(`spin != none` または `lines >= 4`)、PC でない | `N' = N + 1` |
| PC(`pc == true`) | `N' = N + 1`(**+1 のみ**。difficult 分と重ねない) |
| それ以外のライン消去 | `N' = 0`(B2B が切れる) |
| `lines == 0` | `N' = N`(維持) |

> **なぜ `b2b_disp` を状態にしないか**: `b2b_disp` は 0 が「B2B なし」と「1 回目の difficult clear 直後」の両方を表してしまい、遷移が書けない。S7 は `-1` を「なし」のセンチネルに使い、S6 は 1-based の `N` を使っている。**本書は S6 と同じ `N` を採る**。
>
> S6 の golden fixture `chain-and-surge.json` は 1-based(`b2b_after = 1` が最初の difficult clear)なので、**そのまま `N` として読める**。S7 の `stats.b2b` は `N - 1`。

---

## 6. 攻撃計算

### 6.1 base テーブル

| 消去 | `spin` | `lines` | base |
|---|---|---|---|
| Single | none | 1 | **0** |
| Double | none | 2 | **1** |
| Triple | none | 3 | **2** |
| Quad | none | 4 | **4** |
| T-Spin Mini Single (TSM) | mini | 1 | **0** |
| Mini Double | mini | 2 | **1** |
| Mini Triple | mini | 3 | **2** |
| Mini Quad | mini | 4 | **10** |
| T-Spin Single (TSS) | full | 1 | **2** |
| T-Spin Double (TSD) | full | 2 | **4** |
| T-Spin Triple (TST) | full | 3 | **6** |
| T-Spin Quad | full | 4 | **10** |

- 出典: S7 `garbageData`(`single 0, double 1, triple 2, quad 4, tspinMiniSingle 0, tspinMiniDouble 1, tspinMiniTriple 2, tspinDouble 4, tspinTriple 6, tspinQuad 10`)、S8、S10
- **非 T スピンは mini 固定**(§4.2)。したがって Z-Spin Double は base 1、J-Spin Triple は base 2。
- 5 ライン以上(ペンタ)は TL では発生しない(`lines ≤ 4`)。実装は `lines > 4` を reject してよい。
- **0 ラインのスピン**は base 0(攻撃なし)。B2B 維持のみ。

### 6.2 B2B Charging の +1

```
if lines > 0 and N_after >= 2:      # N_after = その設置を反映した後の b2b_streak
    base += 1
```

- **フラット +1**。streak の長さに依存しない(S1「adds one extra line of garbage to each attack until the streak is broken」)。
- **判定は「消した後」の状態で行う**(`N_after`)。理由:
  - 1 回目の difficult clear は `N_before = 0` → `N_after = 1` → **加算なし**(S1「first difficult clear is not counted」)。
  - 2 連続目は `N_before = 1` → `N_after = 2` → **+1**。
  - B2B が切れる消去は `N_after = 0` → **加算なし**(切った攻撃に B2B ボーナスは乗らない)。
  - PC は `N_after = N_before + 1` なので、PC 連鎖中は +1 が乗る。
- 出典: S1、S4 `PN BETA 1.2.0`「B2B sends 1 for every attack (like Guideline)」、S7(`garbageCalcV2` に渡す `b2b` は**更新後**の `max(stats.b2b, 0)`)、S11(`if b2b_after.max(0) > 0 { garbage += 1 }`)、S6 golden `second-difficult-clear` / `ordinary-clear-breaks-b2b`
- **旧 B2B Chaining の表(S1 の 0-1/2-3/4-8/… の表)は使わない。** あれは S1 と 40L/BLITZ のもの。

### 6.3 Multiplier コンボ

```
if combo >= 1:
    base = base * (1 + 0.25 * combo)
if combo >= 2:
    base = max(base, ln(1 + 1.25 * combo))
```

- `combo` は §5 の 0-based。1 回目の消去は `combo = 0` なので**乗算なし**。
- **`ln` 項は「置き換え」ではなく `max`**。base > 0 のとき combo が小さいと ln 項が負けるので実質効かない。**base = 0(Single / TSM / 非 T mini single)のときだけ ln 項が攻撃を作る**。これが S1 の「If the base attack is 0, the function ln(1+1.25x) is used for 2-combo and above」の実装上の意味。
- 出典: S1、S5、S7 `garbageCalcV2`
- 浮動小数の注意: `1 + 0.25*combo` は combo < 2^50 で正確、`base ≤ 10` との積も正確。**乗算側は浮動小数誤差が出ない**。`ln` 側は `ln_1p` を使い、丸めは floor。

### 6.4 丸め

| モード | 挙動 | 使うモード |
|---|---|---|
| **DOWN** | `floor(x)` | **TETRA LEAGUE デフォルト** |
| RNG | `floor(x) + (rng() < frac(x) ? 1 : 0)` | QUICK PLAY |

- 出典: S1、S5、S6 `roundmode: "down"`
- **TL は DOWN**。レフェリーは RNG モードを実装しなくてよい(実装してもデフォルト off)。
- 丸めは **`attack × garbage_multiplier + GSB` の後に 1 回**。途中で丸めない。

### 6.5 パーフェクトクリア

```
if pc:
    attack += floor(5 * garbage_multiplier)
```

- **+5 ゴミ**(multiplier の影響を受ける = 掛けてから floor)、**+1 B2B**。
- 出典: S4 `PN BETA 1.2.0`「All Clears now send 5 Garbage. However, they also count as Back-to-Back.」、S6 `allclear_garbage: 5` / `allclear_b2b: 1` / `allclear_b2b_sends: true` / `allclear_b2b_dupes: false` / `allclear_charges: false`、S11 `S2_TL_PC_GARBAGE = 5` / `S2_TL_PC_B2B = 1`、S6 golden `perfect-clear-adds-b2b`
- **+2 ではない**。+2 は `PN BETA 1.1.0`(S2 プレシーズン)の値。S1 の「All Clear will be gained by 2」は S1 の記述で、S2 本開幕で +1 に変わっている。S3 も「All Clear garbage attack was reduced from 10 to 5, and now worth 1 B2B」と明記。
- **注意**: S11 の `AttackConfig::tetra_league()` には `pc_b2b: 2` / `b2b_chaining: true` という**S1 時代の古いフィールドが残っている**。S11 の S2 経路は `S2_TL_*` 定数を使っており、そちらが `1` / Charging。**古いフィールドをコピーしないこと。**
- **`allclear_b2b_dupes: false`**: PC が同時に difficult clear(TSD 等)でも B2B は **+1 だけ**(difficult 分と PC 分を重ねない)。S7 の実装(`if (difficult && !(pc && pc.b2b)) b2b++` / `if (pc && pc.b2b) b2b += pc.b2b`)がこれに一致。
- **`allclear_charges: false`**: PC の B2B 加算が surge の溜めに寄与しない、と読める。**ただし S7 は区別せず `b2b` を素直に加算しており、surge にも効いてしまう**。→ **§11-U3 で決定(暫定): 寄与する**。

### 6.6 Garbage Special Bonus (GSB)

```
gsb = 1  if  g_cleared > 0  and  (spin != none  or  lines >= 4)
      0  otherwise
```

- **フラット +1**。multiplier の影響を受けない(S1「send a flat (not influenced by multipliers) +1 extra garbage」)。
- 条件は「**その設置でゴミ行を 1 行以上消した**」かつ「**スピン or 4 行消し**」。
  - Quad でゴミを消す → +1
  - TSD でゴミを消す → +1
  - 非 T mini double でゴミを消す → +1(spin なので)
  - Single でゴミを消す → **+0**(spin でも 4 行でもない)
  - Quad でゴミを消さない → **+0**
- 出典: S1「Quads and Spins (including All-Spins) that clear garbage send a flat +1」、S4 `PN BETA 1.3.0`、S7(`garbageCleared > 0 && (spin || lines >= 4)`)、S11(`garbage_cleared > 0 && difficult_clear`)、S6 `garbagespecialbonus: true`
- 加算位置: `floor(attack * mult + gsb)`。**丸めの内側**に入るが、gsb は整数なので結果は `floor(attack*mult) + 1` と等しい。

### 6.7 surge(B2B Charging)

#### 6.7.1 溜め

- 溜め量は**状態として持たない**。`b2b_streak`(= `N`)から毎回導出する(実装が単純で、S7/S6 もそうしている)。
- 表示上の surge 量:

```
b2b_disp     = max(N - 1, 0)
surge_charge = b2b_disp      if b2b_disp >= 4
               0             otherwise
```

- すなわち **表示 B2B x4 から溜まり始め、溜め量 = 表示 B2B 数**。上限なし。
- 出典: S1「When you reach a B2B streak of 4, you begin charging a Surge attack. Surge has a starting power of 1 line in QUICK PLAY and 4 lines in other modes. Each level of B2B adds 1 line to Surge … if the player has an 8 B2B streak, then 8 lines are charged (5 lines in QUICK PLAY)」、S6 `b2bcharge_at: 4` / `b2bcharge_base: 3`、S11 `S2_TL_B2B_CHARGE_AT = 4` / `S2_TL_B2B_CHARGE_BASE = 3`、S6 golden `chain-and-surge.json`
- 一般式(TETR.IO のオプション表現): `surge = b2b_disp_before - at + base + 1`。TL は `at = 4, base = 3` → `surge = b2b_disp_before`。QP は `base = 0` → `surge = b2b_disp_before - 3`。
- `N` で書くと: 放出時の `N_before >= 5` が条件で、`surge_charge = N_before - 1`。

#### 6.7.2 放出条件

```
release  iff  (lines > 0) and (not difficult) and (not pc) and (N_before >= 1)
```

- つまり **difficult clear でも PC でもないライン消去**で B2B が切れた瞬間に放出。
- ライン消去なし(`lines == 0`)では**切れない**(放出しない)。
- PC は B2B を**加算**するので切れない。
- `N_before >= 1` は「そもそも B2B が繋がっていた」ことの条件(繋がっていなければ「切る」ものが無い)。
- 出典: S1「When you break a B2B streak, all stored Surge lines are sent at once」、S7(`brokeB2B` フラグは `stats.b2b` が `-1` でないときのみ真)、S6 golden `ordinary-clear-breaks-b2b`

#### 6.7.3 量と 3 分割

```
S = floor(surge_charge * garbage_multiplier)      # multiplier を掛けて floor
if S > 0:
    r = round(S / 3)                              # 四捨五入(0.5 は切り上げ)
    chunks = [r, r, S - 2*r]                      # 0 の要素は捨てる
```

- **`r = round(S/3)` を 2 回使い、3 番目は残り**。したがって:
  - `S = 3k`   → `[k, k, k]`
  - `S = 3k+1` → `[k, k, k+1]` … **端数は 3 番目**
  - `S = 3k+2` → `[k+1, k+1, k]` … **端数は 1・2 番目**
- 出典: S7(`[Math.round(surged/3), Math.round(surged/3), surged - 2*Math.round(surged/3)]`)、S6 golden `chain-and-surge.json`(`amount 4 → [1,1,2]`, `amount 5 → [2,2,1]`)
- **S1 の文言「first and second segments carrying the remainder」は 3k+2 の場合しか当たらない。3k+1 では 3 番目が端数を持つ。S1 の記述は不正確**(実装 2 つが一致している方を採る)。→ §11-U4 で決定(暫定)。

#### 6.7.4 送出順序

surge の 3 分割は、**その設置の通常攻撃より前に**送られる。

```
送信イベント列 = [surge_chunk_1, surge_chunk_2, surge_chunk_3, 通常攻撃, PC ボーナス]
```

- 出典: S7(`gEvents.splice(0, 0, ...garbages)` で先頭に挿入、PC は末尾に push)
- 各イベントは**独立に**相殺される(§7.3)。合計は同じだが、**相殺の順序とキューへの積算順序が変わる**ので実装上は無視できない。

### 6.8 1 設置あたりの攻撃計算(完成形)

```
fn attack(clear, state) -> (attack_events, next_state):
    # 0. 前提
    N_before = state.b2b_streak
    combo    = state.combo                   # その設置の「前」の値
    difficult = (clear.spin != none) or (clear.lines >= 4)

    # 1. chain 更新(§5 の遷移表)
    if clear.lines > 0:
        combo_after = combo + 1
        if clear.pc:                         # PC は +1 のみ(difficult 分と重ねない)
            N_after = N_before + 1
            broke   = false
        elif difficult:
            N_after = N_before + 1
            broke   = false
        else:
            N_after = 0
            broke   = (N_before >= 1)
    else:
        combo_after = 0
        N_after     = N_before
        broke       = false

    # 2. base(§6.1)+ B2B フラット +1(§6.2、判定は N_after)
    base = BASE[clear.spin][clear.lines]     # lines == 0 のとき 0
    if clear.lines > 0 and N_after >= 2:
        base += 1

    # 3. combo multiplier(§6.3)
    if combo >= 1:
        base = base * (1 + 0.25 * combo)
    if combo >= 2:
        base = max(base, ln_1p(1.25 * combo))

    # 4. 通常攻撃イベント(§6.6 GSB 込み)
    gsb  = 1 if (clear.g_cleared > 0 and (clear.spin != none or clear.lines >= 4)) else 0
    main = floor(base * mult + gsb)  if (base > 0 or gsb > 0) else NONE

    # 5. surge(§6.7。N_before を使う。N_after は 0 になっている)
    events = []
    if broke and N_before >= 5:
        S = floor((N_before - 1) * mult)     # = floor(b2b_disp_before * mult)
        if S > 0:
            r = round_half_up(S / 3)
            events += [c for c in [r, r, S - 2*r] if c > 0]

    # 6. 通常攻撃 → PC の順に追加(§6.7.4)
    if main != NONE: events += [main]
    if clear.pc:     events += [floor(5 * mult)]

    return (events, combo_after, N_after)
```

注意 1: `combo` は**その設置の前の値**を式に入れる(1 回目の消去は combo 0)。
注意 2: B2B の +1 判定だけは**設置後**の `N_after` を使う(§6.2)。surge 量は**設置前**の `N_before` を使う(§6.7.1)。この 2 つを取り違えると「B2B を切った攻撃に +1 が乗る」「surge が 1 少ない」というバグになる。

---

## 7. ゴミシステム

### 7.1 定数(S6 の観測値)

| 定数 | 値 | 意味 |
|---|---|---|
| `garbagecap` | **8** | 1 ロックで挿入できる最大行数 |
| `garbagecapmax` | 40 | cap の上限(cap が増加する設定のとき) |
| `garbagecapincrease` | 0 | cap の時間増加(0 = 増えない) |
| `garbagecapmargin` | 0 | cap 増加の開始時間 |
| `garbageabsolutecap` | 0 | 0 = 無限(キュー全体の上限なし) |
| `garbagespeed` | **20** frame | パケットが「到着」してから挿入可能になるまでの遅延 |
| `garbageholesize` | 1 | 穴の幅 |
| `messiness_change` | **1.0** | 攻撃(パケット)を消化し終えたときに列を引き直す確率 |
| `messiness_inner`(within) | **0.0** | 同一パケット内で列を引き直す確率 |
| `messiness_nosame` | false | 直前と同じ列を避けるか |
| `messiness_timeout` | 0 | 時間経過での列変更なし |
| `messiness_center` | false | 中央寄せなし |
| `garbagemultiplier` | 1.0 | 攻撃倍率の初期値 |
| `garbageincrease` | **0.008** /s | margin time 後の増加量 |
| `garbagemargin` | **10800** frame(180 s) | margin time 開始 |
| `garbageblocking` | `combo blocking` | ライン消去したロックでは挿入しない |
| `roundmode` | `down` | 丸め |
| `passthrough` | `zero` | パススルー無効 |

- 出典: S6 `normalizedOptions`(TETR.IO クライアントの end option をそのまま記録)
- 独立裏取り: S8 README「in tetrio, you will at most accept 8 lines of garbage at a time, which is garbage cap = 8」、S1「Garbage takes 20 frames, or 0.333 seconds, to travel」

### 7.2 キューへの積算

- 送信側の攻撃は、**送信イベントごとに 1 パケット**として相手のキューに積まれる。
- パケットは `(amount, arrival_frame)` を持つ。`arrival_frame` は送信時点の frame。
- キューは `arrival_frame` 昇順、同時刻なら**積まれた順**(FIFO)。
- キュー全体の上限は**なし**(`garbageabsolutecap: 0`)。
- 出典: S7 `GarbageQueue.receive` / `tank`(`this.queue.sort((a,b) => a.frame - b.frame)`)、S6 `garbageabsolutecap: 0`

### 7.3 相殺(キャンセル)

```
cancel(attack_amount, pieces_placed):
    send   = attack_amount
    cancel = 0
    if (pieces_placed + 1 <= 14) and (queue_total >= sent_total):
        cancel = attack_amount          # Opener Phase: 相殺予算が 2 倍

    while (send > 0 or cancel > 0) and queue not empty:
        queue[0].amount -= 1
        if queue[0].amount == 0:
            pop queue[0]
            if rng.next_float() < messiness_change:   # 1.0 → 常に真
                reroll_column()
                has_changed_column = true
        if send > 0: send -= 1
        else:        cancel -= 1

    sent_total += send
    return send        # 残り = 相手に送られる行数
```

- **相殺が送信より優先**(`send` を先に消費するので、結果として「まず自分の攻撃でキューを削り、削りきれなかった分が相手へ飛ぶ」)。
- **1 行ずつ**キューを削る。パケットの途中でも削れる。
- パケットを削りきった瞬間に列を引き直す(§7.6)。
- `sent_total` は「**実際に相手へ飛んだ行数の累計**」を保持する(Opener Phase の条件で使う)。
- 出典: S7 `GarbageQueue.cancel`、S6 `spike_stress.rs` の同等実装

**複数イベントがある場合**(surge 3 分割 + 通常攻撃 + PC): §6.7.4 の順に、**イベントごとに `cancel` を呼ぶ**。1 つのイベントでキューを削りきれなかった残りが `sent_total` に加算され、次のイベントの相殺はその続きから始まる。

### 7.4 Opener Phase

```
条件: pieces_placed + 1 <= 14   かつ   queue_total >= sent_total
効果: 相殺予算が 2 倍(cancel = attack_amount を追加）
```

- `pieces_placed` は**その設置より前**に置いたピース数。したがって **1 個目〜14 個目**の設置が対象。15 個目から無効。
- **hold はカウントを進めない**(hold は設置ではない)。hold を挟んでも「設置 14 個」で終わる。
- `queue_total` は**現在キューに残っている行数**(到着前のパケットも含む)。
- `sent_total` は**このラウンドで相手に飛ばした行数の累計**(相殺で消えた分は含まない)。
- 効果は「**相殺量が 2 倍**」であって「送信量が 2 倍」ではない。相手への送信量は変わらない。
- 出典: S1「For the first 14 pieces placed, if your lines sent that round is less than the amount pending, you cancel twice as much」、S4 `PN BETA 1.1.0`/`1.2.0`、S7、S6 `openerphase: 14`
- 確度: [確定]。ただし「`>=` か `>` か」は S1 の文言(`less than`)と実装(`>=`)が食い違う。**実装(`>=`)を採る** → §11-U5 で決定(暫定)。

**具体例**: 相手から 4 行 pending、自分はまだ 0 行送信、5 個目の設置で Double(攻撃 1)。
→ 条件成立(`5 <= 14` かつ `4 >= 0`)。`send = 1, cancel = 1` → キューから 2 行消える。`sent_total += 0`。
→ 結果: 2 行相殺、送信 0。

### 7.5 挿入(タンク)

```
on_lock(lines):
    if lines > 0:
        return                      # combo blocking: 消したら挿入しない
    rows = 0
    sort queue by arrival_frame
    while rows < 8 and queue not empty:
        if queue[0].arrival_frame + 20 > current_frame:
            break
        insert_one_row(hole = current_column)
        queue[0].amount -= 1
        rows += 1
        if queue[0].amount == 0:
            pop queue[0]
            if rng.next_float() < messiness_change:
                reroll_column()
                has_changed_column = true
```

- **挿入はロック後・即時**。ライン消去をしなかったロックでのみ発生。
- **1 行ずつ**盤面を上に押し上げる。1 ロックで最大 **8 行**。
- 遅延: パケットの `arrival_frame + 20` を過ぎたものだけ挿入できる。5 PPS(12 frame/piece)なら、送信から約 2 ピース後に挿入可能。
- 出典: S7 `GarbageQueue.tank` / `#lock`、S6 `spike_stress.rs`

**盤面上端の扱い**: 行を挿入すると全行が 1 つ上へずれ、40 行目を超えた分は**破棄**される。S7(teto)は「内部 41 行配列の index 37 を削除し、38・39 を残す」という特殊な実装をしており、**上端 3 行の挙動が素直な実装と異なる**。→ §11-U6 で決定(暫定): **素直な実装を採る**。この quirk は本プロファイルでは**到達不能**(スタック上端は最大 row 30 で、row 37 に届かない)ことを §11-U6 で証明済み。

**穴の縦順(実装の検算)**: 挿入は「1 行ずつ下に差し込む」ので、**先に挿入した行ほど上に押し上げられる**。`take_insertable` が返す配列は**挿入順**なので、`holes[0]` が**最上段**になる。

例: パケット A(2 行、穴 3)と B(1 行、穴 7)が同時に due のとき、返り値は `[3, 3, 7]`。挿入後の盤面は下から:

| 行(挿入ブロック内) | 穴の列 | 由来 |
|---|---|---|
| row 2(最上段) | 3 | `holes[0]` |
| row 1 | 3 | `holes[1]` |
| row 0(最下段) | 7 | `holes[2]` |

**バッチで一括代入する実装は `holes` を逆順に割り当てる**(`holes[0]` → 最上段)。ここを素直に書くと穴の縦順が反転し、1 パケット内は同列なので**複数パケットにまたがる挿入でだけ**盤面が変わる — 発見が遅れる典型パターン。

### 7.6 change-on-attack(穴列の乱数)

```
初期: last_column = null, has_changed_column = false

insert_one_row():
    col = last_column
    if (col == null) or (rng.next_float() < within):    # within = 0.0
        if not has_changed_column:
            col = reroll_column()
            has_changed_column = true
    place row with hole at col
    has_changed_column = false
    # パケットを消化しきったら
    if packet.amount == 0:
        pop packet
        if rng.next_float() < change:                   # change = 1.0 → 常に真
            reroll_column()
            has_changed_column = true

reroll_column():
    col = floor(rng.next_float() * 10)                  # nosame=false, center=false
    last_column = col
    return col
```

- **TL の実効挙動**: パケット(= 1 回の攻撃)の**全行が同じ列**。パケットを消化し終えた時点で**必ず**列を引き直す。したがって「攻撃内は同列、攻撃間は必ず別列(ただし乱数が同じ値を返すことはある)」。
- `within = 0.0` なので、同一パケット内で列が変わることはない。
- **PLAN §3.1 の「攻撃間でばらけ率 ×2.5」は QUICK PLAY の記述**(S5 QP 節)。TL では change = 1.0 で、×2.5 は存在しない。
- 出典: S7 `GarbageQueue.#reroll_column` / `tank`、S5「change on attack」、S6 `messiness_change: 1`, `messiness_inner: 0`

**RNG 消費順**(再現性に直結):
1. 行挿入時: `last_column == null` なら draw なしで reroll(内部で 1 draw)。`last_column != null` なら `within` 判定で **1 draw**(結果は必ず false)。
2. パケット消化時: `change` 判定で **1 draw**、真なら `reroll_column` で **1 draw**。
3. 相殺でパケットを消化したときも 2. と同じ(挿入時と同じ順)。

> 実装は**この draw 順序を厳密に再現**しないと、同じ seed でも穴列がずれる。`crates/core` のプロパティテストで「同一 seed + 同一イベント列 → 同一盤面」を検証すること。

### 7.7 乱数(MINSTD / Park–Miller)

```
MODULUS    = 2_147_483_647
MULTIPLIER = 16_807
MAX_FLOAT  = 2_147_483_646

seed_init(s):  v = s mod MODULUS;  if v <= 0: v += MAX_FLOAT
next():        v = (MULTIPLIER * v) mod MODULUS;  return v
next_float():  (next() - 1) / MAX_FLOAT
```

- 出典: S7 `RNG`、S6 `spike_stress.rs` の `Rng`(同一実装)
- ピース袋とゴミ列で**別インスタンス**。seed の割り当て方は **§11-U7 のレフェリープロトコル**で確定(テストベクタあり)。

### 7.8 margin time(攻撃倍率)

```
multiplier(frame) = 1.0 + max(0, frame - 10800) * (0.008 / 60)
```

- **ただし閉じた式で計算してはいけない。** TETR.IO は **1 frame ごとに `+0.008/60` を累積**する。閉じた式と IEEE-754 で結果が食い違い、**実際に 1 行差が出る**:
  - frame 12300 で、累積値は `1.199999999999978`、閉じた式は `1.2`。
  - 攻撃 5 に multiplier を掛けて floor すると、累積では **5**、閉じた式では **6**。
- 出典: S6 `src-js/dynamic-values.mjs` のコメント(実測付き)、S6 `spike_stress.rs` の `Multiplier`
- **実装要求**: `f64` の逐次加算で `multiplier[frame]` を構築する(または同じ丸め結果になる累積キャッシュ)。閉じた式は禁止。

---

## 8. 攻撃計算の具体例

すべて `multiplier = 1.0`(frame ≤ 10800)を前提。`N` は §5 の `b2b_streak`(連続 difficult clear 数)、`combo` は 0-based。**「B2B x5」= `N = 6`** であることに注意。
各式は §6.8 の擬似コードにそのまま対応する。全例は §6.8 の参照実装で機械検証済み(§8 末尾)。

### 例 1: 素の Single(combo 0、B2B なし)

```
N_before = 0, combo = 0, lines = 1, spin = none
base = 0 (Single)
N_after = 1 → N_after >= 2 でない → B2B 加算なし
combo = 0 → 乗算なし
attack = floor(0) = 0
```
**送信 0 行。** これが TETR.IO の 0 コンボ Single(「0 コンボ Single は飛ばない」)。

### 例 2: 2 連続目の Double(combo 1、B2B なし)

```
N_before = 0, combo = 1, lines = 2, spin = none
base = 1 (Double)
N_after = 1 → 加算なし
combo = 1 → 1 × (1 + 0.25) = 1.25
attack = floor(1.25) = 1
```
**送信 1 行。**(DOWN モードなので 1.25 → 1。RNG モードなら 25% で 2。)

### 例 3: B2B x5・combo 2 での TSD

```
N_before = 6 (表示 B2B x5), combo = 2, lines = 2, spin = full
base = 4 (TSD)
N_after = 7 → N_after >= 2 → base = 4 + 1 = 5
combo = 2 → 5 × (1 + 0.5) = 7.5
combo >= 2 → max(7.5, ln_1p(2.5)) = max(7.5, 1.2528) = 7.5
attack = floor(7.5) = 7
```
**送信 7 行。** N は 7 に増え、surge の溜めは 6 になる(まだ放出しない)。

### 例 4: 例 3 と同じだが、その TSD がゴミを 1 行消した

```
例 3 の 7 に GSB +1(g_cleared > 0 かつ spin != none)→ 8
```
**送信 8 行。**

### 例 5: B2B x8 を Single で切る(combo 0)

```
N_before = 9 (表示 B2B x8), combo = 0, lines = 1, spin = none
Single: base = 0, N_after = 0 → 加算なし, combo 0 → 通常攻撃 0
surge: broke = true, N_before = 9 >= 5 → S = floor((9-1) × 1.0) = 8
       r = round_half_up(8/3) = 3 → [3, 3, 8-6=2]
送信イベント列 = [3, 3, 2]   (通常攻撃は 0 なのでイベントなし)
```
**送信 8 行(3 + 3 + 2)。** N は 0 に戻る。

### 例 6: B2B x4 を Single で切る

```
N_before = 5 (表示 B2B x4) → S = floor(4 × 1.0) = 4, r = round_half_up(4/3) = 1 → [1, 1, 2]
Single の通常攻撃 = 0
```
**送信 4 行(1 + 1 + 2)。** 端数が 3 番目に来ることに注意(§6.7.3)。

### 例 7: B2B x3 を Single で切る

```
N_before = 4 (表示 B2B x3) → 4 < 5 → surge なし
Single = 0
```
**送信 0 行。** 表示 B2B が 4 未満だと切っても何も出ない。

### 例 8: B2B x5 を Double(combo 0)で切る

```
N_before = 6 (表示 B2B x5) → S = floor(5 × 1.0) = 5, r = round_half_up(5/3) = 2 → [2, 2, 1]
Double: base 1, N_after = 0 → 加算なし, combo 0 → 1
送信イベント列 = [2, 2, 1, 1]
```
**送信 6 行。** 合計は 5 + 1。順序は surge が先(§6.7.4)。

### 例 9: パーフェクトクリア(Double、combo 0、B2B なし)

```
N_before = 0, combo = 0, lines = 2, spin = none, pc = true
Double: base 1, N_after = 1 → 加算なし, combo 0 → 1
PC ボーナス: floor(5 × 1.0) = 5
N_after = 1
送信イベント列 = [1, 5]
```
**送信 6 行。** 次の difficult clear から B2B ボーナス +1 が乗る。

### 例 10: パーフェクトクリアを TSD で(表示 B2B x2)

```
N_before = 3 (表示 B2B x2), combo = 0, lines = 2, spin = full, pc = true
base = 4 (TSD)
N_after = 4 → N_after >= 2 → base = 4 + 1 = 5
combo 0 → 5
PC ボーナス: +5
N_after は +1 のみ(allclear_b2b_dupes = false。difficult 分と PC 分を重ねない)→ 4
送信イベント列 = [5, 5]
```
**送信 10 行。**

### 例 11: combo 5 での Single(base 0 の ln 項)

```
N_before = 0, combo = 5, lines = 1, spin = none
base = 0 (Single), N_after = 1 → 加算なし
乗算: 0 × (1 + 1.25) = 0
ln 項: max(0, ln_1p(1.25 × 5)) = ln(7.25) = 1.9810
attack = floor(1.9810) = 1
```
**送信 1 行。** base 0 でも combo が乗ると ln で 1 行出る。

### 例 12: margin time の 1 行差(frame 12300)

```
multiplier(12300):
  累積実装 → 1.199999999999978
  閉じた式 → 1.2

N_before = 2 (表示 B2B x1), lines = 4, spin = none
base = 4 (Quad), N_after = 3 → 加算 +1 → 5
combo 0 → 5
attack = floor(5 × 1.199999999999978) = floor(5.99999999999989) = 5
attack = floor(5 × 1.2)               = floor(6.0)              = 6
```
**累積実装で 5 行、閉じた式で 6 行。** 逐次加算が正(§7.8)。

### 例 13: Opener Phase の相殺(§7.4 の再掲)

```
5 個目の設置、相手の pending = 4、自分の sent_total = 0、攻撃 = 1(Double, combo 0)
条件: 5 <= 14 かつ 4 >= 0 → 成立
send = 1, cancel = 1 → キューから 2 行消える(1 行目が send、2 行目が cancel を消費)
sent_total += 0
```
**2 行相殺、送信 0。** 攻撃 1 で 2 行消えるのが Opener Phase。

### 例 14: 非 T mini double と B2B(All-Mini+ の攻撃値)

```
(a) 単発(表示 B2B なし): N_before = 0, lines = 2, spin = mini
    base = 1 (mini double), N_after = 1 → 加算なし
    attack = floor(1) = 1

(b) 2 連続目(表示 B2B x1): N_before = 1, lines = 2, spin = mini
    base = 1, N_after = 2 → 加算 +1 → 2
    attack = floor(2) = 2
```
**単発 1 行、B2B 中 2 行。** 非 T スピンは full にならないので base は mini のまま。B2B の +1 だけが乗る。

### 例 15: TSS(表示 B2B x1)

```
N_before = 2, lines = 1, spin = full, combo = 0
base = 2 (TSS), N_after = 3 → 加算 +1 → 3
attack = floor(3) = 3
```
**送信 3 行。** mini single なら base 0 で、B2B 中でも +1 の 1 行にしかならない(差が大きい)。

### 8.16 検証サマリ

上の 15 例(と補助ケース)を §6.8 の擬似コードどおりに実装した参照実装で実行し、**全 18 ケースが一致**することを確認済み(2026-11-13)。

**独立実装との照合**: 同じ 18 ケースを S11(MochBot/fusion)の `s2_attack_parts` のロジックに通したところ、**イベント列と合計が 18/18 で完全一致**した。差分はカウンタのセンチネル規約だけ(S11 は `-1` を「なし」に使い、本書は 0 始まり)。

機械可読な fixture を **`docs/s2-attack-examples.json`** に置いた(`schema: intetrigence/s2-attack-examples/1`)。`crates/core` のテストはこれを読んで照合すること。

表は「入力 → 送信イベント列(合計)」。

| # | 入力(N / combo / lines / spin / pc / g / mult) | イベント列 | 合計 | N_after |
|---|---|---|---|---|
| 1 | 0 / 0 / 1 / none / – / 0 / 1.0 | `[]` | 0 | 1 |
| 2 | 0 / 1 / 2 / none / – / 0 / 1.0 | `[1]` | 1 | 1 |
| 3 | 6 / 2 / 2 / full / – / 0 / 1.0 | `[7]` | 7 | 7 |
| 4 | 6 / 2 / 2 / full / – / 1 / 1.0 | `[8]` | 8 | 7 |
| 5 | 9 / 0 / 1 / none / – / 0 / 1.0 | `[3,3,2]` | 8 | 0 |
| 6 | 5 / 0 / 1 / none / – / 0 / 1.0 | `[1,1,2]` | 4 | 0 |
| 7 | 4 / 0 / 1 / none / – / 0 / 1.0 | `[]` | 0 | 0 |
| 8 | 6 / 0 / 2 / none / – / 0 / 1.0 | `[2,2,1,1]` | 6 | 0 |
| 9 | 0 / 0 / 2 / none / PC / 0 / 1.0 | `[1,5]` | 6 | 1 |
| 10 | 3 / 0 / 2 / full / PC / 0 / 1.0 | `[5,5]` | 10 | 4 |
| 11 | 0 / 5 / 1 / none / – / 0 / 1.0 | `[1]` | 1 | 1 |
| 12 | 2 / 0 / 4 / none / – / 0 / 1.199999999999978 | `[5]` | 5 | 3 |
| 12' | 2 / 0 / 4 / none / – / 0 / 1.2(閉じた式) | `[6]` | 6 | 3 |
| 13 | Opener 相殺(pending 4、sent 0、攻撃 1) | 2 行相殺 | 送信 0 | – |
| 14a | 0 / 0 / 2 / mini / – / 0 / 1.0 | `[1]` | 1 | 1 |
| 14b | 1 / 0 / 2 / mini / – / 0 / 1.0 | `[2]` | 2 | 2 |
| 15 | 2 / 0 / 1 / full / – / 0 / 1.0 | `[3]` | 3 | 3 |
| 補1 | 0 / 0 / 4 / none / – / 0 / 1.0(1 個目の Quad) | `[4]` | 4 | 1 |
| 補2 | 1 / 0 / 4 / none / – / 0 / 1.0(2 個目の Quad) | `[5]` | 5 | 2 |

---

## 9. レフェリーが実装すべき状態

`crates/core` の `GameState` は最低限これを持つ(§6.8 / §7 の式が参照する)。

| フィールド | 型 | 用途 |
|---|---|---|
| `board` | 40 行 × 10 列 bitboard | 盤面 |
| `combo` | u32 | §6.3 |
| `b2b_streak` | u32 | §6.2 / §6.7(**表示 B2B ではなく連続 difficult clear 数 N**) |
| `pieces_placed` | u32 | §7.4 |
| `sent_total` | u32 | §7.4 |
| `garbage_queue` | Vec<Packet{amount, arrival_frame}> | §7.2 |
| `garbage_rng` | MINSTD 状態 | §7.6 |
| `last_column` | Option<u8> | §7.6 |
| `has_changed_column` | bool | §7.6 |
| `bag` | 7-bag 状態 + RNG | §2.4 |
| `hold` | Option<Piece> + `hold_available` | §2.5 |
| `next` | [Piece; 5] | §2.6 |
| `frame` | u64 | §7.5 / §7.8 |
| `multiplier_cache` | 累積 f64 | §7.8 |

**決定論の要求**: `(seed, 入力列)` から盤面・攻撃・キューが**完全に再現**できること。verify-replay(PLAN §3.3)の前提。

---

## 10. 実装チェックリスト(core-engine 向け)

- [ ] §6.1 の base テーブルが全 12 ケースで一致(特に **mini triple = 2 / mini quad = 10**)
- [ ] §6.2 B2B +1 が「1 回目の difficult clear では乗らない」(判定は `N_after >= 2`)
- [ ] §6.3 combo の ln 項が **max** であること(base > 0 で ln が勝つケースを作らない)
- [ ] §6.4 丸めが DOWN、**丸めは 1 回だけ**
- [ ] §6.5 PC = +5 / +1 B2B、TSD+PC で **+1 のみ**
- [ ] §6.6 GSB の条件が「ゴミを消した」かつ「spin or 4 行」
- [ ] §6.7.3 surge 3 分割が `[r, r, S-2r]`、`S=4 → [1,1,2]`、`S=5 → [2,2,1]`
- [ ] §6.7.4 surge が通常攻撃より**前**に送られる
- [ ] §7.3 相殺が**1 行ずつ**、`sent_total` が「実際に飛んだ行数」だけ増える
- [ ] §7.4 Opener が `pieces_placed + 1 <= 14`、hold で進まない
- [ ] §7.5 挿入が「ライン消去なしのロック」のみ、最大 8 行、20 frame 遅延
- [ ] §7.6 RNG draw 順序が仕様どおり(同一 seed で同一盤面)
- [ ] §7.8 multiplier が**逐次加算**(閉じた式を使っていない)
- [ ] §3.1.2 I の kick が **SRS+**(素の SRS ではない)
- [ ] §3.2 180 kick が全ピース・全 4 遷移
- [ ] §4.3 T の 5th kick 判定が `(+1,-2)` / `(-1,-2)`
- [ ] §2.3 lock-out 判定が**存在しない**こと
- [ ] **座標表の変異テスト**(§0.2.1): `FRONT` 表を y 反転版に差し替えるとテストが**落ちる**こと。少なくとも「TSD スロット → `Full`」と「`corners=3` / `front=1` → `Mini`」の**両方向**を固定する(bot-dev は 8 テストが落ちることを実証済み)
- [ ] §7.5 ゴミ挿入の**穴の縦順**: 1 回の挿入が複数パケットにまたがるとき、`holes[0]`(最初に挿入した行)が**最上段**に来ること

---

## 11. 決定済み事項(暫定自決)

v1.3 で旧「未確定リスト」の 9 件すべてを**我々の決定**として確定した。作者との連携は廃止され(AGPL 利用許可のみ取得済み・公開 I/F のみ使用)、未確定仕様は我々が自決する。以下は**実装の正**であり、「暫定」は「差し替え条件を満たす新しい証拠が出るまで有効」の意味である。

### 11.0 決定の原則

1. **TETR.IO 専用実装(S7 teto / S11 fusion)が一致するなら、それを採る。** 両者は実リプレイに合わせて作られており、独立 2 実装の一致は一次ソースの散文より強い。
2. **一次ソース(S1〜S5)の散文と実装が食い違うときは、実装を採る。** wiki の文章は古い版の記述が残っていることがある(U4 が実例)。
3. **我々のレフェリーでしか観測できないもの(U7 / U9)は、プロトコルとして定義する。** TETR.IO の再現は目指さない。両者に同じ文書を渡し、replay に記録する。
4. **差し替えは「観測可能なトリガー」でのみ行う。** 曖昧な懸念や「実機で見たい」だけでは変えない。各項目に具体的なトリガーを書いた。

### 11.1 決定一覧

| # | 項目 | 決定(暫定) | v1.2 からの変更 |
|---|---|---|---|
| U1 | SRS+ I kick のテスト順 | S7 の順序(§3.1.2 の表) | 変更なし |
| U2 | T の mini 判定 | 3-corner + 2-corner のみ(immobile は非 T 専用) | 変更なし |
| U3 | PC の surge 寄与 | **寄与する**(PC の +1 も溜めに数える) | 変更なし |
| U4 | surge 3 分割の端数 | `[r, r, S-2r]`(`S=4 → [1,1,2]`) | 変更なし |
| U5 | Opener 境界 | `queue_total >= sent_total`(以上) | 変更なし |
| U6 | ゴミ挿入の上端 | 40 行・上端破棄(quirk 不実装) | 変更なし(**到達不能を証明**) |
| U7 | seed → RNG 割り当て | **MINSTD の前進で導出**(下記 U7) | **変更あり**(旧案は相関バグ) |
| U8 | mini triple | base **2** | 変更なし |
| U9 | frame 進行 | `frame = turn × 12` | 変更なし(遅延・margin time を明文化) |

**v1.2 の暫定措置から値が変わるのは U7 だけ**。core-engine には U7 のみ連絡する。

---

### U1. SRS+ の I kick のテスト順

**決定(暫定)**: §3.1.2 の表をそのまま使う。すなわち 0→R は `(0,0), (+1,0), (-2,0), (-2,-1), (+1,+2)` の順に試す。

- **根拠**: S7(teto)の `kicks["SRS+"].i_kicks`。オフセット集合は素の SRS と同一で、**順序だけ**が入れ替わっている(0→R / R→0 / 2→R / L→2 の T2/T3、0→L の T4/T5)。S1 は「I の kick が左右対称」としか書かず、図は画像で数値化できない。
- **影響範囲**: 両方合法な局面でどちらに着地するか、および 5th kick 判定(§3.3)の成立可否。I の配置全体。
- **差し替え条件**: 次のいずれかが起きたら再検討する。
  1. attax の S2 版が提出した I の配置を**我々の kick 順では到達不能**としてレフェリーが拒否し、その拒否が 1 マッチで 3 回以上発生した(→ attax 側の表を確認し、多い方を採る)。
  2. TETR.IO のリプレイで、両方合法な局面の着地位置が本書と逆だった実例が得られた。
- **実装上の備え**: `I_KICKS` を 1 箇所に隔離し、差し替え時に表だけ変えられるようにする(core-engine は対応済み)。

### U2. T の mini 判定における immobile の扱い

**決定(暫定)**: **T は 3-corner / 2-corner ルールのみ**で mini / full を決める。immobile 判定は**非 T 専用**。T が 3-corner を満たさない場合、immobile でも mini にはならない(スピンなし扱い)。

- **根拠**: S2「In All-Mini, the T-piece does not score Spin Minis when immobile」と S7 の実装(`#detectSpin` が T には `#detectSpinFromCorners` のみを使い、`isAllSpinPosition` は非 T の分岐でのみ使う)が一致。S4 `PN BETA 1.5.0` の「Immobile T-Spins … now count as Minis」だけが素直に読むと食い違うが、これは「3-corner を満たさない T スピンは mini になる」の意味であって、immobile 判定を T に適用する意味ではないと解釈する。
- **差し替え条件**: TETR.IO のリプレイで「3-corner を満たさず、immobile でもない T」が mini として記録されている実例が得られた場合。
- **影響範囲**: T の置き方のうち、隅が 2 個以下しか埋まっていない局面。実戦では稀。

### U3. PC の B2B 加算が surge の溜めに寄与するか

**決定(暫定)**: **寄与する。** PC の +1 は `b2b_streak` を 1 増やし、その結果として surge の溜め量も 1 増える。**別カウンタは持たない。**

- **根拠**: S7(teto)も S11(fusion)も、B2B カウンタを 1 本だけ持ち、PC の加算をそのまま surge の導出に使う(S11 は `b2b_after += PC_B2B` の後に `broke` を `None` にして surge を抑止するが、カウンタ自体は増えるので次の break で効く)。S6 の option 名 `allclear_charges: false` だけが「PC は溜めない」と読めるが、**その意味を裏付ける実装も記述も無い**。
- **設計判断**: カウンタを 2 本にすると、表示 B2B と溜め量がずれて UI/評価/リプレイ検証のすべてが複雑になる。PC は 1 試合に 0〜2 回程度しか起きず、**影響は最大 1 行**。複雑さに見合わない。
- **差し替え条件**: TETR.IO のリプレイで「B2B x3 → PC → 非 difficult clear」を打って surge が出ない実例が得られた場合(この 1 ケースだけが判別点)。
- **影響範囲**: PC の直後に B2B を切る局面のみ。

### U4. surge 3 分割の端数の位置

**決定(暫定)**: `r = round_half_up(S / 3)`、`chunks = [r, r, S - 2r]`(0 は捨てる)。したがって `S = 3k → [k,k,k]`、`S = 3k+1 → [k,k,k+1]`、`S = 3k+2 → [k+1,k+1,k]`。

- **根拠**: S7(`[Math.round(surged/3), Math.round(surged/3), surged - 2*Math.round(surged/3)]`)と S6 golden fixture(`amount 4 → [1,1,2]`, `amount 5 → [2,2,1]`)が一致。S11 も同じ(`floor(S/3 + 0.5)`)。S1 の散文「first and second segments carrying the remainder」は **3k+2 の場合しか当たらない**。
- **差し替え条件**: TETR.IO のリプレイで B2B x4 を Single で切り、**1 発目が 2 行**来ている実例が得られた場合(本書は 1 行)。
- **影響範囲**: surge の合計は変わらない。**到着順と、相殺の順序**だけが変わる(§6.7.4)。合計が同じなので勝率への影響は小さい。

### U5. Opener Phase の境界条件

**決定(暫定)**: **`queue_total >= sent_total`**(以上)で発動する。すなわち pending と sent が同数でも相殺は 2 倍になる。

- **根拠**: S7(`currentSize >= this.sent`)と S6(`garbage.size() >= sent`)が一致。S1 の散文は「less than」= `pending > sent` で、`pending == sent` のときだけ差が出る。
- **設計判断**: 同数は「まだ相手に 1 行も通していない」状態に近く、プレイヤーの体感とも合う。実装 2 つが一致している以上、散文より実装を採る(原則 2)。
- **差し替え条件**: TETR.IO のリプレイで、`pending == sent` の局面の相殺量が 1 倍だった実例が得られた場合。
- **影響範囲**: `pending == sent` の 1 点のみ。opener 中の相殺量が 1 行変わる。

### U6. ゴミ挿入時の上端 3 行の扱い

**決定(暫定)**: **素直な実装**を採る。40 行の盤面で、挿入により全行を 1 つ上へずらし、**row 39 を超えた分を破棄**する。S7(teto)の「内部 41 行配列の index 37 を削除し 38・39 を残す」quirk は**実装しない**。

- **根拠(到達不能の証明)**: この quirk は「スタックが row 37 以上に達した状態でゴミを挿入したとき」だけ差が出る。しかし本書のレフェリーでは、その状態に**到達しない**:
  1. spawn セルは row 21〜22(§2.2.1)。スタックが row 21 に達すると block-out して試合が終わる。
  2. ピースは spawn 位置から下にしか動かないので、1 ロック後のスタック上端は最大 **row 22**。
  3. ゴミ挿入は 1 ロックあたり最大 **8 行**(§7.5)。よって挿入後のスタック上端は最大 **row 30**。
  4. `30 < 37` なので、row 37 の削除は**一度も発火しない**。
- **差し替え条件**: 盤面高を 40 行より大きくする、1 ロックの挿入上限を 8 行より大きくする、または spawn 位置を下げる変更をした場合(いずれも本プロファイルでは予定なし)。そのときは S7 の quirk を再検討する。
- **影響範囲**: 無し(到達不能)。**この項目は今後も再検討不要**。

### U7. seed から各 RNG への割り当て(レフェリープロトコル)

**決定(暫定)**: MINSTD の状態を**前進させて**導出する。TETR.IO の導出式は非公開なので再現せず、**我々のプロトコルとして定義**する。

```
M = 2_147_483_647
A = 16_807

base = MINSTD(s)                 # 状態 = s mod M(0 になったら M-1 に補正)
piece_seed       = base.state    # ピース袋用
garbage_seed(0)  = base.next()   # slot 0 用
garbage_seed(1)  = base.next()   # slot 1 用

piece_rng         = MINSTD(piece_seed)
garbage_rng(slot) = MINSTD(garbage_seed(slot))
```

- **根拠**: TETR.IO の導出式は非公開で、S6 も「hole RNG state は入力」「callers must pass a public value」として導出を対象外にしている。よって**我々のプロトコルとして定義する**(原則 3)。乱数器は §7.7 の MINSTD をそのまま使い、**新しい定数を導入しない**。導出に同じ RNG を使うので、実装は 1 つのプリミティブだけで済み、両言語での一致検証も容易。
- **`slot` は席ではなく bot の identity**: `slot 0 = intetrigence`、`slot 1 = attax`。**両シートで席を入れ替えても、bot ごとの穴列は同一 seed なら同じ**になる(シート間の対称性)。
- **`piece_seed` は 1 本だけ導出し、両者が同じ値を使う**(PLAN §3.1「両者同一 seed」)。ただし**袋のインスタンスはプレイヤーごとに持つ**(`Bag::new(piece_seed)` を 2 つ)。したがって hold を使うまで両者の列は一致し、hold を使った本人の袋だけが 1 個進む。詳細は [`docs/s2-referee-protocol.md`](s2-referee-protocol.md) §2.2。
- **ラウンド開始時に毎回この導出から初期化**する。ラウンド間で RNG を引き継がない。
- **テストベクタ**(実装の検算用。機械可読版は `docs/s2-rng-derivation.json`):

| seed `s` | `piece_seed` | `garbage_seed(0)` | `garbage_seed(1)` |
|---|---|---|---|
| 1 | 1 | 16_807 | 282_475_249 |
| 12_345 | 12_345 | 207_482_415 | 1_790_989_824 |
| 123_456_789 | 123_456_789 | 469_049_721 | 2_053_676_357 |
| 2_147_483_646 | 2_147_483_646 | 2_147_466_840 | 1_865_008_398 |

- **衝突しないこと**: `piece_seed`、`garbage_seed(0)`、`garbage_seed(1)` は相異なり、いずれも 0 にならない(M が素数で `s ≠ 0` のため)。20 万個のランダム seed で検証済み。
- **v1.2 からの変更理由**: v1.2 の暫定案 `garbage_rng(p) = MINSTD(seed ^ (p * 0x9E3779B9))` は **`p = 0` のとき `MINSTD(seed)` となり、ピース袋とゴミ列が同一の乱数ストリームになる**。slot 0 の穴列が bag 順と完全に相関するため、対称性が壊れる。**この案は破棄する。**
- **差し替え条件**: 次のいずれか。
  1. TETR.IO のリプレイと**穴列まで一致**させたい要求が出た場合(現状は不要。我々のレフェリーが正)。
  2. 導出の実装が 2 言語以上で食い違った場合(→ 本節を唯一の正として実装を直す。プロトコルなので差し替えではなく修正)。
- **影響範囲**: ゴミの穴列。**攻撃量には影響しない**。ただし replay の完全一致検証に効くので、レフェリーと bot で同じ導出を使うこと。

### U8. 非 T スピンの「mini triple」の存在

**決定(暫定)**: 非 T の 3 ライン消しスピンは **mini + lines 3 → base 2**。mini quad は **10**。

- **根拠**: S7 の `garbageData`(`tspinMiniTriple: 2`、`tspinQuad: 10`)。S11 も同じ定数(`SPIN_MINI_TRIPLE = 2`、`SPIN_QUAD = 10`)を持つ。S1/S2 に記述は無いが、**TETR.IO 専用実装 2 つが一致**。
- **差し替え条件**: TETR.IO のリプレイで L/J/S/Z のスピン triple が **0 行**だった実例が得られた場合(base 0 説)。
- **影響範囲**: 非 T の 3 ライン消しスピンのみ。実戦頻度は低いが、B2B を繋ぐ手段として使われることがある。

### U9. 5 PPS ペアドターンと frame 進行(レフェリープロトコル)

**決定(暫定)**: `frame = turn × 12`。`turn` は 0 始まりの**ペアドターン番号**で、1 turn に両者が 1 ピースずつ置く。

- **根拠**: TETR.IO は 60 fps 固定。5 PPS = 12 frame/piece。ペアドターンなので両者のピースは同じ frame に置かれる。
- **帰結(実装が知っておくべき数値)**:
  - **ゴミ遅延**: turn `t` に送ったパケットは `arrival_frame = 12t`、挿入可能は `frame >= 12t + 20`。整数 turn では `12(t+1) = 12t+12 < 12t+20`、`12(t+2) = 12t+24 >= 12t+20`。**よって遅延は実質 2 turn**。
  - **margin time**: `garbagemargin = 10800` frame = **turn 900**。1000 turn cap では turn 900〜999 が倍率 > 1。
  - **最大倍率**: turn 999(frame 11988)で累積 **1.1583999999999826**(§7.8 の逐次加算)。閉じた式では 1.1584 になり、floor の境界で 1 行差が出得るので**逐次加算必須**。
- **差し替え条件**: ペース設定を 5 PPS から変える場合(→ `frame = turn × (60 / PPS)` に読み替える)。それ以外では変更しない。
- **影響範囲**: ゴミの到着 turn、margin time の発動 turn、終盤の攻撃倍率。**評価関数と探索のリーフ評価にも効く**ので、bot 側もこの frame 規約を共有すること。

---

### 11.2 決定の反映先

| 決定 | 反映が必要な場所 |
|---|---|
| U1 / U2 / U3 / U4 / U5 / U8 | `crates/core` の採点・スピン判定(§6 / §4) |
| U6 | 無し(到達不能) |
| U7 | `crates/core` のゴミ RNG 導出、`crates/arena` のレフェリー初期化、replay 形式 |
| U9 | `crates/core` の frame 進行、`crates/arena` のターン管理、`crates/bot` のリーフ評価 |

### 11.3 差し替えの手続き

差し替え条件を満たす証拠が出たときは、次の順で行う。

1. 本書の該当 U 項目に「差し替え理由」と「証拠(replay のファイル名・コマ数・観測値)」を追記する。
2. §13 の改訂履歴に版を上げて記録する(`v1.3.1` のように)。
3. **値が変わる場合は core-engine(採点・ゴミ・frame)と bot-dev(リーフ評価)に直接連絡**する。
4. 影響する fixture(`docs/s2-attack-examples.json`)を更新し、`crates/core` のテストを通す。

---

## 12. 実装監査(2026-11-13 時点)

`crates/core` を本書と突き合わせた結果。**基準**: `main` = `dd20cda` + `08e883e`(S2 採点・opener・ゴミキュー・スピン分類・U7 導出)。

**独立検証**: `cargo test -p intetrigence-core` = **151 passed / 0 failed**。`attack_fixtures_match_18_of_18` と `rng_derivation_known_vectors` が実際に `docs/*.json` を読んで照合していることを確認済み。

### 12.1 一致を確認したもの

| 対象 | 確認内容 |
|---|---|
| `kick.rs` `JLSTZ_KICKS` | §3.1.1 の 8 遷移 × 5 テストすべて一致 |
| `kick.rs` `I_KICKS` | §3.1.2 の 8 遷移 × 5 テストすべて一致(**SRS+ 対応済み**) |
| `kick.rs` `JLSTZ_180_KICKS` | §3.2.1 の 4 遷移 × 5 テストすべて一致 |
| `kick.rs` `I_180_KICKS` | §3.2.2 の 4 遷移 × 2 テストすべて一致 |
| `kick.rs` `kicks()` の O 分岐 | 90° は `O_KICKS`、180° は汎用表(§3.1.3 の要求どおり) |
| `kick.rs` `is_fin_or_tst_kick` | §3.3 の `(+1,-2)` / `(-1,-2)` 判定と一致 |
| `piece.rs` spawn の x | I/T/S/Z/J/L = 3、O = 4(§2.2 と一致) |
| `piece.rs` spawn の行 | **row 21**(`a97e630` で修正済み、§2.2.1 と一致) |
| `piece.rs` 回転中心 | 3×3 = (1,1)、I 4×4 = (1.5,1.5)、O 2×2 = (0.5,0.5)(SRS true rotation) |
| `board.rs` `FRONT` 表 | **v1.3.1 準拠で `main` にコミット済み**(`dd20cda`)。4 隅 × 4 回転すべて一致 |
| `board.rs` `spin_type` / `is_immobile` | §4.2 / §4.3 と一致(短絡条件・盤外の扱いまで) |
| `scoring.rs` `compute_attack` | §6.8 と**全項目一致**(chain 更新・`N_after >= 2` の B2B +1・ln の max・GSB 条件・`N_before` の surge・イベント順・分割の丸め) |
| `scoring.rs` `BASE_ATTACK` | §6.1 の 3×5 表と一致(mini 行 = `[0,0,1,2,10]`) |
| `garbage.rs` `cancel` / `next_hole` / `take_insertable` | §7.3 / §7.6 / §7.5 と一致(RNG draw 順序・cap 8・20 frame 遅延・combo blocking・stable sort) |
| `opener.rs` | §7.4 と一致(`pieces_placed + 1 <= 14`、`queue_total >= sent_total`、budget = attack、`on_sent` が残り) |
| `rng.rs` `derive_seeds` / `MinStd` | §11-U7 と一致(seed=1 → 1 / 16807 / 282475249) |
| `constants.rs` | §6 / §7 の全定数と一致。`GARBAGE_SEED_MIX`(旧 U7 案)は削除済み |
| `board.rs` `insert_garbage` | **穴の縦順を修正済み**(`holes.iter().rev()` で `holes[0]` を最上段に)。§7.5 の検算例と一致 |

### 12.2 修正が必要なもの

| 対象 | 現状 | 本書の要求 | 優先度 |
|---|---|---|---|
| `opener.rs` の doc | `U5 provisional:` が残る | `U5 (spec v1.3)`。§11 で決定済み | 低 |
| frame 進行 | 未実装(`Game` に frame/turn が無い) | **§11-U9**: `frame = turn × 12`。`arena` のターン管理と `bot` のリーフ評価が共有する | 中 |
| `crates/arena` | 未着手 | レフェリー本体(PLAN §3.3 の監査機能を含む) | — |

### 12.3 本書側で用意したもの

| ファイル | 内容 | 使い方 |
|---|---|---|
| `docs/s2-attack-examples.json` | 攻撃計算の fixture 18 件(`schema: intetrigence/s2-attack-examples/1`) | `crates/core` のテストで読み、`attack()` の出力と `expected` を照合する |
| `docs/s2-rng-derivation.json` | seed → RNG 導出のテストベクタ 4 件(`schema: intetrigence/s2-rng-derivation/1`) | `crates/core` / `crates/arena` の RNG 初期化テストで読む(§11-U7) |
| `docs/s2-referee-protocol.md` | マッチ運用の仕様(ターン/席/提出検証/replay/監査) | `crates/arena` の実装はこれを正とする。ルール値は本書を参照 |

`fixture` は「入力(`lines` / `spin` / `combo` / `b2bStreak` / `perfectClear` / `garbageCleared` / `multiplier`)+ 期待値(`events` / `total` / `comboAfter` / `b2bStreakAfter` / `base` / `gsb` / `surgeAmount`)」の形。`base` は B2B +1 と combo 乗算を適用した**後**の値(丸め前)。

**フィールドの意味(取り違えやすいので明記)**:

| フィールド | 意味 |
|---|---|
| 入力 `combo` | **この設置で式に入る値** = 直前までの連続消去数(0 = 直前が消去なし)。§5 の `combo` |
| 入力 `b2bStreak` | **この設置前**の連続 difficult clear 数 `N`(0 = B2B なし)。§5 の `b2b_streak` |
| 出力 `comboAfter` | **次の設置で式に入る値** = 消去ありなら `combo + 1`、消去なしなら 0 |
| 出力 `b2bStreakAfter` | この設置後の `N` |

> **S11(fusion)との対応**: S11 の `combo_after` は「その設置で式に入る値」なので、**本書の入力 `combo` と同値**。S11 の `b2b_after` は `-1` 始まりのセンチネルなので **`本書の b2bStreakAfter - 1`**。名前が紛らわしいので、S11 を参照するときは値の意味で読み替えること。

### 12.4 `crates/bot` の監査(bot-dev `7cff8c0`)

exact movegen の実装を本書と突き合わせた結果。**`main`(core `08e883e`)に rebase 済み。**

| 対象 | 確認内容 |
|---|---|
| spin 分類 | **core に一本化**。`crates/bot/src/spin.rs` のコーナー表コピーを削除し、`board.spin_type(...)` / `board.is_immobile(...)` を直接呼ぶ。表の二重管理が解消 |
| 座標 | `Placement.piece` の `(x, y)` = ボックス左下 = core の期待と一致。**変換コード無し** |
| 独立再導出 | **テスト側に第 2 オラクルとして残置**(占有セルから nub 方向を求めて dot product で front を選ぶ)。毎配置で `placement.spin` と比較 |
| 生値ガード | `(2,2)` / `(2,0)` のクロス回転、TSD `(3,2)`、`corners=3 / front=1` の `(3,1)` を固定 |
| end-to-end | `movegen → spin == Full → 2 ライン → base_attack(Full,2) == 4 → compute_attack().total == 4` |
| 変異テスト | `FRONT` を反転させると **8 テストが落ちる**(§0.2.1 の要求を満たす) |

**私の独立検証**:
- TSD 盤面(`rows = 0x1FF, 0x3EF, 0x3C7, 0x1CF`)をビット展開して確認 → T を `(Rotation::Two, 3, 1)` に置くと **2 行が消える**、corners = 3、front(rot 2) = 2 → `Full`。col 9 の穴は 4 隅のどれでもないので、bot-dev のフィクスチャ修正は**分類を変えていない**
- `cargo test --workspace` = **168 passed / 0 failed**(core + bot の統合状態)

### 12.5 `crates/arena` + `crates/tbp` の監査(harness-dev `2dad514` / `4460ab4`)

レフェリーと TBP host を [`docs/s2-referee-protocol.md`](s2-referee-protocol.md) と突き合わせた結果。**コード修正は 0 件**(仕様側を 2 箇所直した)。

| 対象 | 確認内容 |
|---|---|
| `frame = turn × 12` | `begin_turn()` が `turn * frames_per_turn`(既定 12)✓ §2.1 |
| margin time | `GarbageMultiplier::advance_to` が **1 frame ずつ加算** ✓ §7.8 |
| U7 導出 | `derive_seeds(seed)` → 1 つの `piece_seed` + 2 つの garbage seed ✓ §11-U7 |
| ピース供給 | `Game::new(piece_seed)` を 2 つ = **プレイヤーごとの袋** ✓ §2.2(v1.1) |
| 席順 | `sheet.order()` で seat 1 → seat 2 ✓ §2.3 |
| 第 1 候補採用 | movegen 集合の最初の候補を採用し `candidate_index` を記録 ✓ §4.3 |
| 到達可能性 | `moves.get(&cells)` で検証し、Action 列を replay して `accepted` と位置一致を確認 ✓ §4.2 |
| hold | `held_piece().or(preview[0])` と照合 ✓ §4.2-4 |
| 採点入力 | `combo_before` / `b2b_before` を渡す ✓ §6.8 |
| 相殺 | **イベントごと**に `cancel_with_opener`、順序は `attack.events` のまま ✓ §5.2 |
| 送信 | イベントごとに別パケット、`arrival_frame = frame` ✓ §5.3 |
| opener | `on_piece_placed()` が相殺の**後**(設置前の値で判定)✓ §7.4 |
| 挿入 | `lines == 0` のときだけ `take_insertable(0, frame)` ✓ §5.4 |
| GSB | `lock.clear.garbage_lines` ✓ §6.6 |
| 採点オーディット | `claimed_spin` で `declared_attack` を別計算して記録 ✓ PLAN §3.3 |
| ターン上限 | 受信少 → 送信多 → 引き分け ✓ §6.3 |
| トップアウト | `is_over()`(= 次 spawn 衝突)で判定し即終了 ✓ §6.1 |
| verify-replay | `placement.piece` のみ使用(`turn.piece` は不使用)✓ §8.1 |

**仕様側で直した 2 件**(実装は正しかった):

1. **§2.2 の「共有 7-bag」→「プレイヤーごとの袋(同一 `piece_seed`)」**(protocol v1.1)。共有袋だと hold が列に影響せず、TETR.IO より hold が強くなる。
2. **§6.1 の「同一 turn で両者トップアウト → 引き分け」は到達不能**(protocol v1.2)。先に死んだ側で打ち切るため。実装の「即終了・相手の勝ち」に統一。

**残るリスク・未追随(2 件)**:

| 種別 | 内容 | 対応 |
|---|---|---|
| リスク | **movegen の二重実装**。`crates/arena/src/movegen.rs` が `crates/bot` の BFS を独立実装している(bot 未統合のため)。片方だけ直すと**合法手集合がずれ、正当な着手が拒否される** — §4.3 の FRONT 表バグと同じ構造 | bot 統合後に `(cells, spin)` の**完全一致テストを恒久化**する。差し替え後も残す |
| 到達不能(仕様として明記) | **空の候補の 2 段判定**(protocol §7)。`NoMoves`(レフェリーの movegen にも合法手が無い)は**実戦では到達しない** — 生きている active は定義上 spawn から到達可能なので movegen は必ず 1 個以上返す。実装は防御分岐として保持(`c50397d`) | 対応済み。テストは構成できないので、コードに理由コメントを残す扱い(§6.1 の同時トップアウトと同じ) |

### 12.6 attax 現行版との差分(レフェリー設計上の注意)

`docs/attax-analysis.md`(attax-integration 担当)が記録した attax `7b38f51` の採点実装は **S2 ではない**。レフェリーは attax の自己申告する攻撃値を信用せず、**必ず本書の式で再計算**すること(PLAN §3.3 の「採点一致オーディット」の前提)。

| 項目 | attax 現行(guideline) | 本書(S2 TL) |
|---|---|---|
| Mini Triple | 0 | **2** |
| Mini Quad | 未定義 | **10** |
| T-Spin Quad | 未定義 | **10** |
| コンボ | 加算テーブル | **乗算** `base × (1 + 0.25 × combo)` + ln 項 |
| B2B | +1 のみ | **+1 + surge** |
| PC | +10 | **+5** |
| ゴミ込みボーナス | なし | **+1** |
| Opener Phase | なし | **相殺 ×2** |
| 180° 回転 | なし | **あり** |
| All-Spin | T のみ | **All-Mini+(全ピース)** |

attax の T-spin 判定(3-corner + 5th kick で full)は本書 §4.3 と**一致**する。差分は上表の項目に限られる。

**実測による裏付け**(M0、seeds 1001–1005 × 両シート = 10 ゲーム、attax `7b38f51`。`scripts/evidence/m0/`):

| ボット | 配置数 | spin 不一致 | 攻撃不一致(同 spin) | 200 ms 超 |
|---|---|---|---|---|
| baseline(自作) | 735 | 0 | 0 | 0 |
| attax | 730 | **12** | **0** | 0 |

- attax の spin 不一致 12 件は**すべて「attax `none` → S2 `mini`」の非 T スピン**。All-Mini+ の非 T immobile 判定を attax が実装していないことの直接証拠で、上表の「All-Spin | T のみ」と一致する。
- **同じ spin での攻撃不一致は 0 件**。つまり attax の攻撃計算そのもの(コンボ加算テーブル等)は、この 10 ゲームでは差が出る局面に遭遇していない。**「attax の採点が違う」と言えるのは spin 分類の差だけ**であり、レポートではその区別を明示すること。
- 12 件の spin 不一致のうち攻撃量が変わったのは 2 件のみ。mini single の base は 0 で none single の base も 0 なので、**B2B が乗っていない限り攻撃量は一致する**(§6.1 / §6.2)。差が出た 2 件は B2B が有効な局面。


### 12.7 M2 ゲートの実測と forfeit 問題(2026-11-13)

`scripts/evidence/m2-tuning/`(seeds 9001–9010 × 両シート = 20 ゲーム、attax `7b38f51`、replay 20/20 検証):

| 内訳 | 件数 |
|---|---|
| `Forfeit` 勝ち(attax の 20 行視野による候補 0) | **18** |
| `TopOut` 負け(40 行トップアウトまで到達) | **2**(いずれも sheet A) |
| 合計 | 18–2 = 90.0%(Wilson 95% CI 69.9–97.2%) |

**この数字を「フル視野の attax に対する 90%」と読んではいけない。** `TopOut` で決着した 2 ゲームは **0 勝 2 敗**であり、現行 attax に勝ち越している根拠は「20 行視野を突いた」ことだけである。

扱いは **protocol §7.1(forfeit の扱い、v1.8)** で決定した。要点:

- forfeit は**勝ちとして数える**(分母を結果で変えない)
- ただし `reason` と `forfeit_cause` で**内訳を必ず記録**し、レポートで**勝ちを種類別に分ける**
- `vision` forfeit 勝ちがある場合、**「フル視野相手の勝率」を主張してはならない**
- フル視野相手の推定は `TopOut` 決着のみを別掲し、n と CI を併記して「参考値」と明記

**エビデンス側の不備(要修正)**: `scripts/evidence/m2-tuning/summary.json` の `notes` が **M0 の文言のままハードコード**されており、「the intetrigence side is the in-process M0 baseline stand-in, not the real bot」と書かれている。M2 は**本物の bot**(`think_median_ms = 96.9`、`--ms 100`)で走っているので、この注記は**事実と異なる**。`crates/arena/src/harness.rs` の `notes` が固定文字列であることが原因。最終レポートの前に必ず直すこと。

## 13. 改訂履歴

| 版 | 日付 | 変更 |
|---|---|---|
| v1 | 2026-11-13 | 初版。PLAN §3.1 全 15 行の裏取り、攻撃式・surge・All-Mini+・GSB・Opener・ゴミ・SRS+/180 の数値固定、計算例 15 件 + 機械検証 fixture、未確定 9 件。PLAN の「×2.5」「PC +2」の 2 点を訂正。 |
| v1.1 | 2026-11-13 | `crates/core` `07aaa24` の実装監査を反映。§2.2.1 を追加(spawn セルの絶対座標を表で固定。**最下段セルは row 21**)。§12 を監査結果に書き換え。S11(MochBot/fusion)を出典に追加。 |
| v1.2 | 2026-11-13 | S11(fusion)の S2 攻撃実装で §6.2 / §6.5 / §6.6 / §6.7 を三重裏取り。fixture 18 件を S11 のロジックと照合し、**イベント列・合計が 18/18 一致**。§5 の `combo` の定義を「式に入る値」と明示。§12.3 にフィールド意味表を追加。S11 の `AttackConfig` に残る S1 時代の古い値(`pc_b2b: 2`)への警告を追記。 |
| v1.3 | 2026-11-13 | **§11 の 9 件すべてを決定(暫定自決)**。各項目に「決定値・根拠・差し替え条件」を明記。§11.0 に決定の原則、§11.1 に決定一覧、§11.2 に反映先、§11.3 に差し替え手続きを追加。**U7(seed → RNG)を変更**: 旧案 `seed ^ (p * 0x9E3779B9)` は `p = 0` でピース袋と同一ストリームになるバグがあったため、MINSTD の前進による導出に差し替え。テストベクタを `docs/s2-rng-derivation.json` に追加。**U6 は到達不能を証明**。U9 にゴミ遅延 2 turn・margin time turn 900・最大倍率 1.1584 を明記。§0 / §0.1 / §3.1.2 / §3.2.2 / §4.3 / §6.5 / §6.7.3 / §7.4 / §7.5 / §7.7 / §12.2 の参照を「決定(暫定)」に更新。 |
| v1.3.1 | 2026-11-13 | **§4.3 の front corner 表の符号を修正**(bot-dev の指摘)。S7 の `cornerTable` は `Δy > 0` が下方向で、v1.3 までは転記時に反転し忘れていた。front が上下逆になり、**到達可能な TSD がすべて mini(base 1)**になる致命的な誤り。修正後の表は S7 生データの y 反転と完全一致し、全 4 回転で front が nub 側にあることを機械検証済み。回転ごとの front 2 隅の表と「符号の罠」の注意書き、TSD 検算を追加。`crates/core/src/board.rs` の `FRONT` 表と `crates/bot/src/spin.rs` の `T_FRONT_CORNERS` に影響。 |
| v1.3.2 | 2026-11-13 | 再発防止。**§0.2.1「座標表を転記するときの規約」を新設**(反転する / 規約を書く / 反転を検出するテストを書く)。**§10 に変異テストと穴の縦順のチェック項目を追加**。ルール値の変更は無し。 |
| v1.3.3 | 2026-11-13 | `crates/core` `dd20cda` / `08e883e` のコミットを受けて §12 を更新。FRONT 表・`insert_garbage` の穴順・`GARBAGE_SEED_MIX` 削除を「一致」に移動。**`cargo test -p intetrigence-core` = 151 passed を独立検証**。§7.5 に穴の縦順の検算例を追加。 |
| v1.3.4 | 2026-11-13 | `crates/bot` `7cff8c0` の監査を §12.4 に追加(spin 分類の core 一本化・独立オラクル・end-to-end TSD・変異テスト)。**`cargo test --workspace` = 168 passed を独立検証**。 |
| v1.3.5 | 2026-11-13 | §11-U7 の「袋は 1 本」という表現を修正。**seed は 1 本、袋はプレイヤーごと**であることを明記(hold で本人の袋だけ進む)。`docs/s2-referee-protocol.md` v1.1 と整合。ルール値の変更なし。 |
| v1.3.6 | 2026-11-13 | `crates/arena` + `crates/tbp` の監査を §12.5 に追加(適合 19 項目、仕様側の修正 2 件、movegen 二重実装のリスク 1 件)。§12.6 に繰り下げ。 |
| v1.3.7 | 2026-11-13 | §12.6 に M0 の実測裏付けを追加(attax の spin 不一致 12/730 はすべて非 T の `none → mini`、同 spin の攻撃不一致は 0)。All-Mini+ の差が実測で確認できた。 |
| v1.3.8 | 2026-11-13 | §12.5 の残件を更新。movegen 二重実装に加えて、**空の候補の 2 段判定(protocol §7 / v1.6)が未追随**であることを記録。 |
| v1.3.9 | 2026-11-13 | §12.5 の「空の候補の 2 段判定」を**未追随 → 到達不能(実装済み)**に更新(harness-dev `c50397d`)。`NoMoves` が到達しない証明を protocol §7 に追加。 |
| v1.3.10 | 2026-11-13 | §12.7 を追加。M2 ゲートの内訳(18 forfeit 勝ち / 2 top-out 負け)と、`summary.json` の `notes` が M0 の文言のままハードコードされている不備を記録。扱いは protocol §7.1(v1.8)で決定。 |
