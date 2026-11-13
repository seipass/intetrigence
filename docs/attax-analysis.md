# attax 敵分析 — 探索・評価・思考時間の全量

対象: [souta-lab/attax](https://github.com/souta-lab/attax) の bot 実装 `src/ai/bot/`。
本書は**ソースを直接読んだ事実**と、**こちらで実測した数値**、そして**S2 対応に関する推測**を分けて記録する。
推測はすべて「推測」と明記する。TBP 接続仕様は [attax-integration.md](attax-integration.md) を参照。

- attax commit: **`7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`**(`main`, 2026-11-13)
- 読解対象: `src/ai/bot/{movegen,features,weights,search,fighter}.ts`(計 934 行)、
  および採点・T-spin の `src/core/{garbage,tspin,srs,pieces}.ts`
- 実測環境: Windows 11, AMD Ryzen 7 3700X, Bun 1.4.0
- 実測スクリプト: 一時プローブ(`bun` で `createBot().findBestMove` を直接呼ぶ)+ `scripts/attax-tbp-smoke.mjs`
- ライセンス: AGPL-3.0(作者許可済み)。**本体 bot へのコード・重みのコピーは方針禁止**(§10)

---

## 0. 要約(数値)

| 項目 | 値 | 出典 |
|---|---|---|
| 探索 | beam search | `search.ts` |
| beam 幅 | **48**(デフォルト) | `fighter.ts:22` |
| 探索長 | 現在ピース + **5 preview** = 6 ピース | `fighter.ts:22`, `search.ts:92` |
| 実効深さ | **6**(hold 取得後)/ **5**(hold 空で root hold を使う場合) | 実測 + `search.ts` |
| hold | あり(root と各 ply で使用可) | `search.ts:190-215` |
| 割引 | **0.97 / ply** | `weights.ts:61` |
| 評価特徴量 | **11 項** | `features.ts:10-29` |
| 報酬項 | attack + 9 種の clear + b2bBreak + wastedT | `weights.ts:47-60` |
| 重み | 手設定の固定 1 セット(学習なし) | `weights.ts:33` |
| 思考時間 | **時間制限なし**。beam 幅で決まる固定仕事量 | `search.ts` 全体 |
| 実測 think time | median **31.5 ms** / p95 41.3 ms(TBP スモーク、9 手) | `summary.json` |
| 実測ノード数 | 8,168〜10,809 / 手(median 9,124) | `summary.json` |
| 1 手の合法配置数(空盤面) | I 17, O 9, T 34, S 17, Z 17, J 34, L 34 | 実測 |
| 受信ゴミ | **root の 1 手目にしか効かない**(`pending` はスカラー) | `search.ts:241` |
| 盤面視野 | **下 20 行のみ** | `movegen.ts:20` |
| bag 認識 | **なし**(preview の列をそのまま使う) | `search.ts:92` |
| 相手モデル | **なし**(相手の盤面・攻撃タイミングを見ない) | 全体 |

---

## 1. 探索(beam search)

### 1.1 アルゴリズム

`search.ts:90` の `search()`:

1. `sequence = [現在ピース, ...preview]`(`search.ts:92`)。preview は fighter が 5 個に切る。
2. root ノードを作り、beam = [root]。
3. 各 ply で、beam の全ノードを**現在ピースの全合法配置**と**hold を使う場合の全合法配置**で展開する。
4. 子を `value` の降順にソートし、**上位 `width` 個**を残す(`dedupe`、`search.ts:287`)。
5. ピースが尽きたノード(`pieceOptions` が空)は `finished` に入れる。
6. 全ラインが top-out した場合は、最深の生存ノード(`deepest`)を葉として使う。
7. `finished`(または `deepest`)の中で最大 `value` の葉を選び、その根までの手順を返す。
8. `candidates` は「root の各第 1 手」ごとに最良葉の value を集めて降順ソートしたもの。

- **transposition table なし**。dedupe は同一 ply 内の完全一致のみ。
- **反復深化なし**。anytime ではない。
- **マルチスレッドなし**(単一 JS スレッド)。
- **相手モデルなし**。相手の盤面・ゴミ・攻撃タイミングは一切見ない。

### 1.2 dedupe キー

```ts
`${node.next}|${node.hold}|${node.b2b ? 1 : 0}|${node.board.join(",")}`
```

(`search.ts:291` 付近)

**`combo` と `lift` はキーに含まれない**。同じ盤面・hold・次ピース・b2b でも combo や受信ゴミ量が違うノードは
同一視され、value の高い方だけが残る。これは近似であり、コンボ狙いやゴミ量の違いを潰す可能性がある(事実)。

### 1.3 価値関数

```
value(node) = Σ_{i<d} 0.97^i * reward_i  +  0.97^d * boardScore_d
```

- `scale = 0.97^parent.depth`(`search.ts:252`)
- `accumulated = parent.reward + scale * reward`(`search.ts:253`)
- `value = accumulated + scale * 0.97 * dot(features, w)`(`search.ts:254`)

つまり盤面評価は「その深さの割引」を受ける。**深いほど盤面スコアの重みが下がる**。

### 1.4 深さの決まり方(実測)

`sequence` の長さ = 1 + preview 数。デフォルトは 6。

- hold を使わないライン: 6 手まで置ける → depth 6。
- **hold スロットが空**の状態で root で hold を使うライン: `next` が 2 進む(現在ピース + 次のピースを消費)ため
  5 手まで → depth 5。
- hold スロットに既存ピースがある場合の hold: `next` が 1 進むだけ → depth 6。

実測(`createBot().findBestMove`、空盤面、preview 5):

| hold | preview | sequence 長 | 報告 depth |
|---|---|---|---|
| null | 3 | 4 | 3 |
| null | 4 | 5 | 4 |
| null | 5 | 6 | **5** |
| `T` | 3 | 4 | 4 |
| `T` | 4 | 5 | 5 |
| `T` | 5 | 6 | **6** |

TBP スモークでも、hold が空の序盤 3 手は depth 5、hold 取得後 6 手は depth 6 だった
(`summary.json` の `depths.perMove = [5,5,5,6,6,6,6,6,6]`)。

> README の「depth 6 on 98.7% of moves」は、hold がほぼ常に埋まっていることと整合する。
> 例外は各ゲームの初手(hold 空)など。**推測**: 1.3% は初手 + hold を最後まで使わないゲーム。

### 1.5 実測: beam 幅と仕事量

空盤面・hold=`T`・preview 5 で `width` を変えた測定(各 5 回の中央値):

| width | nodes | median ms | min ms | max ms |
|---|---|---|---|---|
| 12 | 2,777 | 10.7 | 7.5 | 15.0 |
| 24 | 5,477 | 15.0 | 14.1 | 16.6 |
| **48** | **10,778** | **29.2** | 28.0 | 29.8 |
| 96 | 19,583 | 52.1 | 50.3 | 53.2 |
| 192 | 36,940 | 100.0 | 97.9 | 103.2 |

- ノード数は width にほぼ比例(最初の ply の展開数を除く)。
- 時間はノード数にほぼ比例。**時間制限はどこにもない**。
- width 48 で約 29 ms。README の「median 35 ms」と整合する。

---

## 2. movegen(完全 BFS)

`movegen.ts:132` の `generatePlacements()`。

### 2.1 入力空間

エンジンの実入力(左移動・右移動・ソフトドロップ・CW/CCW 回転 + SRS kick)上の BFS。
状態は `(rotation, x, y)`。`rotation` 4 × `y` (-4..27) × `x` (-3..12)。

- 接地(`collides(board, kind, rotation, x, y+1)`)した状態を「休息配置」として記録。
- 回転は **CW / CCW の 2 方向のみ**。180° は無い。
- kick は「最初にフィットした kick を採用」(エンジンの `tryRotate` と同じ)。`lastSrs = i === 0 ? 1 : i + 1`。
- O ピースは回転 1 種のみ。

### 2.2 開空間ショートカット

ピース全体がスタックより上にある間は壁だけが制約なので、スタックの 3 行上から全 (rotation, x) を seed する
(`movegen.ts:160` 付近)。これで開空間の全状態を spawn から歩かない。

### 2.3 spin 判定

- 休息状態に**回転を最後の入力として**入れる場合、`lastSrs > 0` を記録。
- 同じ休息状態に複数の回転経路で入れる場合、**最大の `lastSrs` を保持**する(`movegen.ts:207-209`)。
  これは「そのラベルを再現する入力列が存在する」ことを意味する(テストで実エンジンにより検証済み)。
- T のみ `classifyTspin` で mini/full を判定(`movegen.ts:226`)。
- **T 以外のスピンは判定しない**(`tspin: "none"` 固定)。

### 2.4 重複排除

- T 以外: セル座標キーで重複排除(同じセルに複数の回転で到達しても 1 つ)。
- **T: 重複排除しない**。同じセルでも spin ラベル(none/mini/full)が違えば別配置。

### 2.5 実測: 空盤面の合法配置数

| 種 | 配置数 |
|---|---|
| I | 17 |
| O | 9 |
| T | 34 |
| S | 17 |
| Z | 17 |
| J | 34 |
| L | 34 |

T/J/L が 34 と多いのは、4 回転・非対称で休息位置が多いため。T は spin ラベル違いを重複排除しないので、
同じセルでも none/mini/full が別配置として数えられる(I/S/Z は 17、O は 9)。

### 2.6 ロックと消去

`lockPlacement()`(`movegen.ts:273`): セルを書き、全 10 列が埋まった行を消し、生存行を下に詰め、上に空行を足す。
`perfectClear` は「1 行以上消して、盤面が完全に空」。

---

## 3. 評価特徴量(全 11 項)

`features.ts:10-29` の `BOARD_FEATURES`。盤面は「配置後・消去後」の 20 行盤面。
`lift` は「そのライン中にスタックの下に着地するゴミ行数」。

| # | 特徴量 | 定義 | 重み |
|---|---|---|---|
| 1 | `maxHeight` | 最も高い列の高さ `h_max` + `lift` | **-6** |
| 2 | `danger` | `max(0, h_max + lift - 10)^2`(10 超えの二乗) | **-40** |
| 3 | `holes` | 同じ列の上に埋まったセルがある空セル数 | **-60** |
| 4 | `holeCover` | 各 hole の上にある埋まりセル数(上限 4)の合計 | **-12** |
| 5 | `holeRows` | hole を 1 つ以上含む行数 | **-40** |
| 6 | `bumpiness` | 隣接列の高さ差の絶対値の和(メイン well の隣接ペアを除く) | **-8** |
| 7 | `bumpinessSq` | 同じく二乗和 | **-3** |
| 8 | `rowTransitions` | 各行の埋まり/空きの変化数(壁は埋まり扱い) | **-6** |
| 9 | `wellDepth` | メイン well の深さ(`min(4, min(左右) - h[well])`) | **+18** |
| 10 | `extraWells` | メイン well 以外で、両隣より 3 以上低い列の数 | **-40** |
| 11 | `tSlot` | 最良の T-spin スロットが消せる行数(0/1/2) | **+60** |

補足(ソースの実装):

- **メイン well** = 最も低い列(同点なら左端)。`wellDepth` は壁を高さ 20 として扱う。
- `bumpiness` / `bumpinessSq` は「メイン well に隣接するペア」を除外する。
- `rowTransitions` は**空でない行のみ**数える。壁を埋まりとして両端に 1 を足す。
- `holes` / `holeCover` / `holeRows` は上から走査し、`covered` マスク(上に埋まりがある列)で判定。
- `holeCover` は hole ごとに「上にある埋まりセル数」を **4 で上限クリップ**。
- `SAFE_HEIGHT = 10`(`features.ts:39`)。`danger` は高さ 10 を超えた分の二乗。
- `tSlot` は `bestTSlot()`(`features.ts:151`)で計算。条件:
  - stem セルが空、左右の下角が両方埋まり、
  - 平らな 3 セル(stem の 1 行上)が空、
  - 上角が**片方だけ**屋根付き(overhang)でもう片方が開いている、
  - 開いている側の列の高さが届く範囲。
  - 消せる行数は stem 行と flat 行の 2 行まで(0/1/2)。

---

## 4. 報酬項

`weights.ts:33-62` の `DEFAULT_WEIGHTS`(固定 1 セット)。

### 4.1 盤面重み

```ts
board: {
  maxHeight: -6,      danger: -40,      holes: -60,
  holeCover: -12,     holeRows: -40,    bumpiness: -8,
  bumpinessSq: -3,    rowTransitions: -6,
  wellDepth: 18,      extraWells: -40,  tSlot: 60,
}
```

### 4.2 配置報酬

| 項 | 値 | 意味 |
|---|---|---|
| `attack` | **+90** | 発生させたゴミ 1 行あたり(相殺前) |
| `clear.single` | **-60** | Single の追加整形 |
| `clear.double` | **-40** | Double |
| `clear.triple` | **-30** | Triple |
| `clear.quad` | **+60** | Quad |
| `clear.tspinMini` | **-20** | T-spin Mini |
| `clear.tspinSingle` | **+40** | TSS |
| `clear.tspinDouble` | **+120** | TSD |
| `clear.tspinTriple` | **+160** | TST |
| `clear.perfectClear` | **+400** | PC |
| `b2bBreak` | **-90** | B2B を通常消しで切ったとき |
| `wastedT` | **-50** | T-slot があるのに T をスピンなしで置いたとき |
| `discount` | **0.97** | ply ごとの割引 |

- `clear` 報酬は attack 報酬に**上乗せ**される。例: B2B 中の Quad = `4*90 + 1*90 + 60 = 510`。
  通常の Quad = `4*90 + 60 = 420`。
- Single は `0*90 - 60 = -60`、Double は `1*90 - 40 = +50`、Triple は `2*90 - 30 = +150`。
  → **Single は明確に嫌い、Double 以上を好む**。
- `wastedT` は「親盤面の `tSlot > 0`」かつ「T を `tspin: none` で置く」場合のみ。

### 4.3 攻撃テーブル(guideline)

`src/core/garbage.ts` の `attackForClear()`:

| 消し | base |
|---|---|
| Single | 0 |
| Double | 1 |
| Triple | 2 |
| Quad | 4 |
| T-spin Mini Single | 0 |
| T-spin Mini Double | 1 |
| TSS | 2 |
| TSD | 4 |
| TST | 6 |

- B2B: difficult clear(Quad または T-spin 系)のとき **+1**。
- コンボ: `comboBonus(combo)` = 0,1,1,2,2,3,3,4,4,4,5…(combo 0 始まり)。
- パーフェクトクリア: **+10**。
- 合計 = base + b2b + combo + pc。

> これは **guideline**(PPT/TEC 風)であり、TETR.IO S2 の攻撃テーブルではない。
> S2 は Single 0 / Double 1 / Triple 2 / Quad 4 / TSM 0 / TSS 2 / TSD 4 / TST 6 と base は近いが、
> **Mini Triple 2 / Mini Quad 10 / T-Spin Quad 10** が追加され(attax は Mini Triple 0)、
> **コンボが乗算**(base × (1 + 0.25 × combo)、base 0 は ln 項)、**B2B が charging/surge**、
> **PC +5**、**ゴミ込みボーナス +1**、**opener phase ×2** が異なる([s2-spec.md](s2-spec.md) §6)。

---

## 5. T-spin 判定

`src/core/tspin.ts` の `classifyTspin()`。

1. `lastSrs === 0` → `none`(回転で入っていない)。
2. T の 3×3 ボックスの四隅(TL, TR, BR, BL)のうち **3 つ以上**が埋まっていなければ `none`。
   - 盤外: `x < 0 || x >= 10 || y < 0` は**埋まり扱い**(上は壁)。
   - 床より下(`y >= 20`)は**空き扱い**。
3. 回転ごとの「前側 2 隅」:
   - rot0(上向き): TL, TR
   - rot1(右向き): TR, BR
   - rot2(下向き): BR, BL
   - rot3(左向き): BL, TL
4. 前側 2 隅が**片方だけ埋まり**、かつ `lastSrs !== 5` → **mini**。それ以外 → **full**。
   - `lastSrs === 5` は 5 番目の kick(T-spin triple 用)で、無条件 full。

> これは Hoiko バイナリのルール(`corners[r] != corners[r+1]`)であり、TETR.IO の
> 3-corner / 2-corner + 5th kick ルールとは細部が異なる。S2 の All-Mini+ とは別物。

---

## 6. danger と incoming garbage の扱い

### 6.1 実装

`search.ts:241-242`:

```ts
const pending = node.depth === 0 ? rootPending : 0;
const lift = node.lift + Math.max(0, pending - attack);
```

- `pending` は **root の 1 ply 目にしか適用されない**。2 ply 目以降は常に 0。
- `lift` は「そのライン中に着地するゴミ行数」。`max(0, pending - attack)` を加算。
- `features = boardFeatures(outcome.board, lift)` で `maxHeight` と `danger` に反映。
- `pending` は**スカラー**(行数の合計)のみ。キュー・タイマー・穴列・change-on-attack は持たない。

つまり:

- ゴミは「最初の 1 手で相殺しきれなかった分が、そのラインの盤面の下に積まれる」と近似される。
- 2 手目以降に追加で降るゴミは考慮されない。
- 相手の攻撃タイミング・ゴミの穴列・遅延は考慮されない。
- 相殺は「自分の攻撃で pending を減らす」だけ。ゴミキューへの送信は考えない。

### 6.2 実測: pending の効果

空盤面・hold=`T`・preview 5 で `pendingGarbage` を変えたときの最良 value:

| pending | value | 最良手 | depth |
|---|---|---|---|
| 0 | **-28.7** | T(hold) | 6 |
| 4 | **-48.7** | T(hold) | 6 |
| 8 | **-68.7** | I@6,18 | 6 |
| 12 | **-672.0** | I@6,18 | 6 |

hold 空の場合:

| pending | value | 最良手 | depth |
|---|---|---|---|
| 0 | -39.5 | I@6,18 | 5 |
| 4 | -60.1 | I@6,18 | 5 |
| 8 | -115.1 | I@6,18 | 5 |
| 12 | -693.2 | **I@5,18**(手が変わる) | 5 |

- pending 4 の悪化(hold=T で -20.0、hold 空で -20.6)は、主に `maxHeight` の +4
  (`-6 * 4 = -24` を `0.97^6` / `0.97^5` で割り引いたもの)。
- hold=T では pending 8 でも `danger` は 0(高さ 10 以下)。hold 空では pending 8 で `danger` が 1 程度効く。
- pending 12 では両方で `danger` が大きく効き(高さ 10 超え)、hold 空では手が変わる。
  **危険度は二次関数的に効く**。

> 重要な帰結: ゴミは**1 手目の評価にしか入らない**。S2 の「遅延挿入・キュー・change-on-attack」では
> この近似は粗くなる。attax の S2 版がゴミキューを見るようになると、ここが最大の強化点になる(PLAN §2)。

---

## 7. 思考時間の決まり方(固定仕事量)

### 7.1 時間制限は存在しない

`search.ts` / `fighter.ts` / `attax-tbp.ts` のどこにも時間制限・打ち切り・反復深化はない。
仕事量は以下で決まる:

```
仕事量 ≈ Σ_{ply=0..5} (beam 内ノード数) × (そのピースの合法配置数 + hold の場合の配置数)
```

- beam 幅 48、preview 5、hold あり。
- 合法配置数は空盤面で 9〜34(§2.5)。盤面が高くなると減る。
- したがって**時間は盤面に依存して変動する**が、上限は beam 幅で決まる。

### 7.2 実測

| 条件 | nodes | ms |
|---|---|---|
| 空盤面・hold=T・width 48 | 10,778 | 29.2(median) |
| TBP スモーク 9 手 | 8,168〜10,809 | 25.5〜38.4(adapter 自己申告) |
| TBP スモーク 9 手(ホスト往復) | — | 27.0〜41.3(median 31.5) |
| README の実マッチ | — | median 35 |

- `move_info.nps` は**常に 0**(計測していない)。`extra` に `attax {ms}ms` が入る。
- ponder なし。相手の手番中に考えることはしない。
- 単一スレッド。並列化なし。

### 7.3 こちらの視点

PLAN §2 の「35ms 固定 vs 100ms anytime + マルチスレッド」は、実測でも整合する。
attax の仕事量は width 48 × depth 6 で固定されており、**時間を増やしても強くならない**
(作者が S2 版で時間予算を導入しない限り)。こちらは 100ms を使い切れるため、実効 3〜10 倍の計算量が見込める。

---

## 8. 探索の性質と弱点(事実)

1. **固定仕事量**: 時間を増やしても探索は深くならない。ANYTIME ではない。
2. **bag を無視**: preview の列をそのまま使い、7-bag の制約をモデル化しない。
3. **下 20 行しか見ない**: row 20 以上のスタックを見ない(`movegen.ts:20`)。
4. **受信ゴミはスカラー・root のみ**: キュー・タイマー・穴列なし(§6)。
5. **相手モデルなし**: 相手の盤面・攻撃・ゴミを見ない。
6. **固定重み 1 セット**: 状況に応じた重み切り替えなし。
7. **transposition なし**: 同一盤面の再利用なし。dedupe は同一 ply のみ。
8. **combo / lift が dedupe キーに無い**: コンボ狙いの違いが潰れる可能性。
9. **深さの異なるラインを比較**: hold 空の root hold は 5 手で終わるため、6 手のラインと直接比較される。
   盤面スコアの割引が深さで変わるため、**短いラインが有利になりうる**(実測で depth 5 が選ばれた)。
10. **T 以外のスピンなし**: All-Spin 系の評価ができない。
11. **180° 回転なし**: SRS+ の 180° を使えない。

---

## 9. S2 対応に必要な構造変更(推測)

> 以下は**推測**であり、attax 作者の実装方針を保証するものではない。S2 版のリリース時に必ず再確認する。

### 9.1 攻撃・採点

- `src/core/garbage.ts` の `attackForClear()` が唯一の採点入口。S2 化はここに集中する。
- 必要: コンボ乗算(`base × (1 + 0.25 × combo)`、切り捨て)、B2B charging(streak + surge)、
  PC +5、ゴミ込みボーナス +1、opener phase ×2。
- S2 の正確な base テーブル・B2B charging・コンボ乗算・PC +5 は [s2-spec.md](s2-spec.md) §6 を参照。
- **推測**: 探索側は `attack` を reward と `lift` にしか使っていないため、採点関数の差し替えだけでは
  surge の「溜め」と「放出」のタイミングを評価できない。`Node` に surge 状態が必要。

### 9.2 探索状態

- 現在: `Node.b2b: boolean`、`Node.combo: number`、`Node.lift: number`。
- S2 で必要(推測): `b2bStreak`、`surge`、`openerRemaining`、`garbageQueue`。
- `dedupe` キーにこれらを追加する必要がある(現状は combo/lift すら入っていない)。
- **推測**: 状態が増えると beam の多様性が下がるため、width 48 では S2 の評価が粗くなる可能性。

### 9.3 ゴミキュー

- 現在: `pending: number` を root のみ。
- S2 で必要(推測): 行数・タイマー・穴列を持つキュー。`lift` の代わりに「いつ・どの穴で降るか」を
  シミュレートする必要がある。
- **推測**: change-on-attack(攻撃内は同列、攻撃間でばらける)をモデル化しないと、downstack の評価がずれる。

### 9.4 All-Spin / 180°

- 現在: `Placement.tspin: "none" | "mini" | "full"` は T のみ。movegen は CW/CCW のみ。
- S2 で必要(推測): 全ピースの immobile 判定(All-Mini+)、180° 回転 + 独自 kick、SRS+ の I kick 修正。
- **推測**: movegen の BFS に 180° を追加すると状態数が増え、固定仕事量の時間が増える。
  attax が時間制限を持たない場合、1 手あたりの ms が増える可能性がある。

### 9.5 盤面視野

- 現在: 20 行。S2 の内部 40 行・ゴミバッファを扱うには視野拡張が必要。
- **推測**: TBP アダプタの 20 行写像(`rows[19 - r]`)を拡張しない限り、S2 版でも 20 行のまま。

### 9.6 評価関数

- 現在: 11 特徴量 + 固定重み。
- S2 で必要(推測): surge 溜め/放出、opener 相殺、combo multiplier、PC 狙いの特徴量。
- **推測**: 重みは手設定のまま 1 セットで、S2 用に再調整される可能性が高い。
  我々は重みをコピーしない(§10)。

### 9.7 総合的な見立て(推測)

- S2 版は「ゴミキューが見える」「surge を評価できる」ことで、guideline 版より明確に強くなる。
- ただし「20 行視野」「bag 無視」「固定仕事量」「単一重み」は構造的に残る可能性が高い。
- こちらの勝ち筋は PLAN §2 の仮説どおり:
  計算量(100ms anytime × 8 スレッド)、S2 ネイティブ評価、ゴミキュー込みの生存評価、探索の質。

---

## 10. 方針(ライセンスと利用範囲)

- attax は **AGPL-3.0**。作者から AGPL 範囲内での利用許可済み(ソース参照・引用可)。
- **本体 bot(`crates/bot`)への attax コード・重みのコピーは禁止**(方針)。
  - 重みの数値をそのまま移植しない。
  - 特徴量の実装をそのまま写さない。
  - 探索アルゴリズムのコードをそのまま写さない。
- **ハーネスでの利用は可**:
  - TBP アダプタを subprocess として実行する(改変なし)。
  - 検証スクリプト(`scripts/`)で attax を起動・計測する。
  - 本書のように仕様を分析・引用する。
- attax 由来のコードをハーネスに含める場合は、intetrigence 全体を **AGPL-3.0** で公開し、出典を明記する。
- チューニングは tuning seeds のみ。held-out seeds では一切最適化しない(PLAN §2, §6)。

---

## 11. 参照箇所

| 内容 | ファイル:行 |
|---|---|
| beam 幅・preview のデフォルト | `src/ai/bot/fighter.ts:22` |
| fighter の snapshot → search | `src/ai/bot/fighter.ts:54-102` |
| search 本体 | `src/ai/bot/search.ts:90-181` |
| sequence の作り方 | `src/ai/bot/search.ts:92` |
| hold の選択肢 | `src/ai/bot/search.ts:190-215` |
| 報酬・lift・割引 | `src/ai/bot/search.ts:227-254` |
| dedupe キー | `src/ai/bot/search.ts:287-297` |
| 11 特徴量の一覧 | `src/ai/bot/features.ts:10-29` |
| SAFE_HEIGHT | `src/ai/bot/features.ts:39` |
| 特徴量の計算 | `src/ai/bot/features.ts:56-129` |
| T-slot 判定 | `src/ai/bot/features.ts:151-172` |
| 重み一式 | `src/ai/bot/weights.ts:33-62` |
| movegen BFS | `src/ai/bot/movegen.ts:132-240` |
| spin 到着の記録 | `src/ai/bot/movegen.ts:203-210` |
| T-spin 分類 | `src/ai/bot/movegen.ts:244-264` |
| ロック・消去 | `src/ai/bot/movegen.ts:273-300` |
| 攻撃テーブル | `src/core/garbage.ts:23-46` |
| コンボ表 | `src/core/garbage.ts:50-58` |
| difficult clear | `src/core/garbage.ts:71-73` |
| attackForClear | `src/core/garbage.ts:75-88` |
| T-spin 判定 | `src/core/tspin.ts:26-60` |
| SRS kick 表 | `src/core/srs.ts` |
| ピース形状 | `src/core/pieces.ts` |

---

## 12. 再現方法

```powershell
# attax の取得とビルド
cd C:\Users\Hasilan\Projects
git clone https://github.com/souta-lab/attax
cd attax
git rev-parse HEAD        # 7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b
bun install
bun run build
bun test

# TBP 経由の実測(我々のリポジトリ)
cd C:\Users\Hasilan\Projects\intetrigence
node scripts/attax-tbp-smoke.mjs
# → scripts/evidence/attax-tbp-smoke/summary.json に
#    think time / nodes / depth / 第 1 候補採用率が記録される
```

探索パラメタを直接測る場合は、attax リポジトリ内で `createBot({ width })` を呼ぶ一時スクリプトを使う
(本書 §1.5 / §2.5 / §6.2 の数値はこの方法で取得)。一時スクリプトはリポジトリに残さない。
