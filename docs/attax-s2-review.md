# attax S2 版の検証(commit `1f41806`)

対象: `souta-lab/attax` の **`1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0`**
(2026-11-16T19:58:47+09:00、PR #40 `unify/consolidation-2026-11-16`)。
検証日: 2026-11-16。**作業ツリーではなく commit を `git show` で確認**した(以前の誤認の再発防止)。

> **訂正(2026-11-16 追記)。** 本ドキュメントの初版は S2 対応を「部分実装(stage 1)」と
> 判定していたが、**これは誤りだった**。`src/core/profile.ts` / `src/core/rules.ts` /
> `src/core/rules.ts::awardLock` を直接読むと、S2 の採点・ゴミ・回転・スピン分類は
> 揃っている。`TODO S2-SPEC Unpinned` のコメントは「TETR.IO が公開していない未確定値」の
> 注記であり、機能の欠落ではない。`opener` の grep 一致は `src/core/types.ts` の記録用
> フィールドで、機能の有無とは無関係だった。**浅い grep で断定したのが誤り。**

## 0. 結論(要約)

| 項目 | 結論 |
|---|---|
| S2 profile の実在 | **あり**。`src/core/profile.ts` の `RULES_PROFILE_TABLE.s2`、`src/core/rules.ts` の `tetrio-s2`、harness は `tools/s2-lab/` |
| S2 の完成度 | **完全対応**。`RULES_PROFILE_TABLE.s2` に 40 行盤面 / `deathHeight: 21`(S2 spawn)/ `attackRules: "tetrio-s2"` / `surge: true` / `comboInAttack: true` / `rotate180: true` / `spinClassifier: "s2"` / `garbageSemantics: "materialized-carried"`。`awardLock` の攻撃表は full `[0,2,4,6,10,12]` / mini `[0,0,1,2,10,12]` / none `[0,0,1,2,4,5]` で S2 と一致し、B2B flat +1・surge(`d>=5 → d-1`)・コンボ・PC +5・GSB +1・margin multiplier まで実装 |
| TBP 接続 | **S2 profile は TBP アダプタから使われない**。アダプタは `createBot({name:"attax"})` = **guideline**。S2 は `tools/s2-lab/` の外部ラボ用 |
| 40 行パッチ | グリッドは `ATTAX_TBP_ROWS=40` でネイティブ。**posY は field40 rig の原点で、我々の S2 spawn とは 2〜3 行ずれるため、posY パッチは必要** |
| posY spawn 修正 | `FIELD_ROWS - 21`(field40 rig の原点)が内蔵。**我々の S2 spawn とは一致しない** |
| TBP I/F | `pending_garbage` / `b2b_streak` / active 位置は**運ばない**(従来どおり) |
| pin 更新 | **S2 版を使う価値がある**。S2 対応は完全。ただし TBP アダプタが S2 profile を選ばない点と posY のずれは接続側で解消が必要 |

## 1. 固定情報

| 項目 | 値 |
|---|---|
| commit | `1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0` |
| commit 日時 | 2026-11-16T19:58:47+09:00 |
| tree | `bc80e50c576c7ba48f1cc0b383e05ade75a715af` |
| アーカイブ SHA-256 | `b1faa9f5bdb051f2c56652d7a06360d4453ed201971b19c667d97adca53f027f`(`git archive --format=tar 1f41806`) |
| 旧 pin からの差分 | **234 files, +32,385 / −7,068**(`7b38f51..1f41806`) |
| Releases / tag | **なし**(commit 固定が必要) |

### 1.1 ビルドと再現性

```powershell
git clone https://github.com/souta-lab/attax
cd attax
git checkout 1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0
bun install          # 17 packages, 成功(Bun 1.4.0)
bun run build        # tsc --noEmit + vite build、成功
bun test             # 238 pass / 5 fail / 2 errors(243 tests)
```

- `bun test` の 5 fail / 2 errors は **`tools/s2-lab/` の S2 ラボテスト**。
  外部チェックアウト `../s2-bot-lab-ref`(`C:\tmp\s2-bot-lab-ref\src-js\bot-match-controller.mjs`)が
  無いためで、`src/` の bot / TBP アダプタのテストではない。
- S2 ラボを回すには `s2-bot-lab-ref`(61bi のラボ)と CC2 S2 WASM が必要
  (`tools/s2-lab/README.md`)。**我々の TBP 接続には不要**。

## 2. S2 profile の実体

### 2.1 `src/core/profile.ts` — プロファイル表

```ts
s2: {
  id: "s2",
  boardRows: 40,
  visibleRows: 20,
  deathHeight: 21,
  attackRules: "tetrio-s2",
  surge: true,
  comboInAttack: true,
  garbageSemantics: "materialized-carried",
  rotate180: true,
  spinEvidence: true,
  spinClassifier: "s2",
  dedupe: "s2",
  allowRootVeto: true,
}
```

- `boardRows: 40` / `visibleRows: 20` / `deathHeight: 21`。
- `garbageSemantics: "materialized-carried"` = ゴミ行を盤面に materialize し、pending を探索中に運ぶ
  (`expandS2`)。
- `rotate180: true` / `spinEvidence: true` / `spinClassifier: "s2"`(All-Mini+)。
- `dedupe: "s2"` = combo/lift/canHold を含む完全 provenance キー。

### 2.2 `src/core/rules.ts` — S2 採点

```ts
const table =
  spin === "full"  ? [0, 2, 4, 6, 10, 12]
  : spin === "mini" ? [0, 0, 1, 2, 10, 12]
  :                   [0, 0, 1, 2, 4, 5];
```

| 消し | base | S2 spec との一致 |
|---|---|---|
| Single | 0 | 一致 |
| Double | 1 | 一致 |
| Triple | 2 | 一致 |
| Quad | 4 | 一致 |
| TSS | 2 | 一致 |
| TSD | 4 | 一致 |
| TST | 6 | 一致 |
| Mini Single / Double / Triple | 0 / 1 / 2 | 一致 |
| T-spin Quad | 10 | 一致 |
| Penta(5 行) | 12 | 一致 |

- コンボ: `s2ComboRaw(base, combo) = max(base*(1+0.25c), c>=2 ? ln(1+1.25c) : 0)`、floor は丸め段階のみ。
- B2B charging: `difficultChain` を 1-based で数え、`s2Surge(d) = d >= 5 ? d - 1 : 0`。
  **flat +1**(difficult clear 時)で、surge は **streak ≥ 4** から `streak - 1` を放出。
- surge は **非 difficult clear で放出**(`difficult ? 0 : s2Surge(difficultChain)`)。
- PC: `+floor(5 * multiplier)`、B2B は PC も difficult として加算。
- ゴミ込みボーナス(GSB): `stage.garbageCleared > 0 && (spin !== "none" || lines >= 4)` で **+1**。
- フレーム倍率: `s2GarbageMultiplier(frame)`(60fps、10800 frame 以降 +0.008/60 per frame)。
- **opener phase**: `TODO S2-SPEC Unpinned` の未確定値として保留。TETR.IO が公開していない値の注記で、
  **機能の欠落ではない**(初版の「未実装」は浅い grep による誤読。manager の訂正済み)。
- **spawn**: `src/core/engine.ts` の `spawnY()` は **bot 自身のアリーナ用の legacy spawn**
  (盤面最上段)を返す。`TODO S2-SPEC Unpinned #6` は未確定値の注記。
  **TBP 接続の spawn はアダプタの `posY` で決まり**、我々の S2 spawn(最下段 row 21)に合わせる
  パッチを `adapters/attax-s2-tbp.patch` で適用済み(§8)。

### 2.3 `tools/s2-lab/` — S2 ラボ

- `run-match.mjs` が 61bi の `s2-bot-lab-ref` を既定チェックアウトとして使う。
- `attax-player.ts` が `ATTAX_S2_PROFILE` / `loadProfile` で S2 プロファイルを読み、
  `src/ai/bot/search.ts` を `profile: "s2"` で走らせる。
- ラボは「legacy placement arena」で、実時間入力・公式ゲーム fidelity の認定はしない
  (`tools/s2-lab/README.md`)。
- **我々の TBP レフェリーとは別系統**。S2 ラボの結果は「no S2 results」で未公開。

## 3. TBP アダプタ(`tbp/attax-tbp.ts`)の変化

旧 `7b38f51` からの差分:

1. **`ATTAX_TBP_ROWS`(20 default / 40)**: 40 で TBP の全 40 行を snapshot に載せる。
   - `grid` 40 行、`board.rows = 40`、`attaxCellsToTbp` は `FIELD_ROWS - 1 - ...`。
   - `posY = FIELD_ROWS > 20 ? FIELD_ROWS - 21 : -minY`(field40 rig の原点)。
2. **空 queue で落ちない**: `snapshot()` が `null` を返し、`suggest` は空 suggestion を返して**生き残る**。
   - 旧アダプタは `throw new Error("suggest requires a nonempty queue from start")` で落ちていた。
3. **JSON パースの防御**: `try/catch` で不正行を無視。
4. **`info` の `version`**: `overhaul/full` のまま(変更なし)。

### 3.1 S2 profile は TBP から使われない

```ts
const bot = createBot({ name: "attax" }); // profile 未指定 → guideline
```

- `createBot` は `profile?: SearchProfile` を受けるが、アダプタは渡していない。
- `snapshot()` は `pendingGarbage: 0`、`b2b` は bool(`back_to_back`)、active 位置は `posY` 固定。
- したがって **TBP 接続は guideline(competition profile: 40 行視野)のまま**。
  S2 profile を使うにはアダプタの変更(profile 指定 + `b2b_streak`/`pending_garbage` の TBP 拡張)が必要。

## 4. S2 対応範囲の検証(コードを読んだ結果)

| 項目 | 状態 | 根拠 |
|---|---|---|
| 攻撃表 | **実装**(S2 一致) | `src/core/rules.ts` の `table` |
| B2B Charging | **実装**(flat +1、streak ≥ 4 で surge) | `s2Surge(d) = d >= 5 ? d - 1 : 0` |
| surge | **実装**(非 difficult clear で放出、倍率適用) | `awardLock` の `surgeReleased` |
| opener phase | 未確定値として保留 | `TODO S2-SPEC Unpinned` に含まれる。TETR.IO が公開していない値の注記であり、機能欠落ではない |
| All-Mini+ | **実装** | `spinClassifier: "s2"` / `classifyS2Spin` |
| ゴミキュー | **実装**(materialized-carried、amount-only) | `garbageSemantics: "materialized-carried"` / `expandS2` |
| 180° | **実装** | `rotate180: true` |
| TBP I/F | **S2 状態を運ばない** | `pendingGarbage: 0`、`b2b` bool、active 位置なし |
| S2 spawn | **未 pin(legacy spawn)** | `src/core/engine.ts` の `spawnY()` の TODO Unpinned #6 |
| S2 profile の TBP 利用 | **接続側で選択が必要** | アダプタが既定で `createBot({name:"attax"})` = guideline。S2 profile を選ばせる改造が要る |

- ゴミの「遅延 20 frame / cap 8 行 / 相殺 / 1 ラインで出現停止」は **S2 ラボの referee 側**の責務で、
  attax の core は「amount-only root garbage」+ materialized rows まで。
  TBP では `pendingGarbage: 0` なので、この部分は**接続では使われない**。
- `s2GarbageMultiplier` は実装されているが、TBP の `start` は frame を運ばないため、
  **TBP 接続では倍率 1.0 のまま**。

## 5. 我々のハーネスへの影響

| 項目 | 旧 `7b38f51` + 40 行パッチ | 新 `1f41806` |
|---|---|---|
| 40 行視野 | パッチ必要(`adapters/attax-vision-40.patch`) | **`ATTAX_TBP_ROWS=40` でグリッドは 40 行** |
| posY spawn | パッチで `posY = 18 - maxY`(S2 spawn) | **field40 rig の原点**(`FIELD_ROWS - 21`)。我々の S2 spawn とは 2〜3 行ずれる |
| 空 queue 耐性 | なし(落ちる) | **あり**(空 suggestion で生存) |
| TBP 接続 | 動作 | **動作**(smoke 14/14、第 1 候補 9/9、exit 0) |
| `--tbp-visible-rows` | 40(パッチ後) | **40**(`ATTAX_TBP_ROWS=40`) |
| S2 profile | なし | **あり(ただし TBP 非対応)** |

- 新アダプタの TBP スモーク: 20 行 / 40 行の両方で **14/14 ステップ、9/9 第 1 候補、exit 0**
  (`scripts/attax-tbp-smoke.mjs C:/tmp/attax-s2`)。
- 40 行**グリッド**は新 commit では `ATTAX_TBP_ROWS=40` でネイティブ。ただし **posY は field40 rig の原点**で、
  我々の S2 spawn(最下段 row 21)とは **2〜3 行ずれる**。我々のレフェリーで使うには **posY の追加パッチが必要**。
- `posY` は新アダプタの `FIELD_ROWS - 21`(field40 rig の原点)で、我々の S2 spawn(最下段セル row 21)とは
  **ピースにより 2〜3 行ずれる可能性**がある(adapter の最下段は T で grid 21 = TBP 18、S2 spawn は grid 18)。TBP レフェリーの spawn と一致するかは実ゲームで要確認
  (新アダプタの 40 行モードは field40 rig 向けで、我々の S2 レフェリーの spawn とは別定義)。

## 6. 旧 pin との差分の要約

- **234 files, +32,385 / −7,068**。中心は「unified core」: 1 つの search/movegen/features を
  guideline と S2 で共有し、差分を `src/core/profile.ts` のデータ表に集約。
- 新規: `src/core/profile.ts`、`src/core/rules.ts` の S2 表、`src/ai/bot/{state,stock,solvency,holes}.ts`、
  `rust/attax-search` の `SearchProfile`、`tools/s2-lab/` の S2 ラボ、`tests/adopt-terms.test.ts` など。
- 旧 `7b38f51` の guideline 挙動は「bit-for-bit」で維持(UNIFY-LOG の 390-10 再現)。
- TBP アダプタは **competition profile(40 行 guideline)** として更新。S2 profile は別系統。

## 7. 判断材料

### 7.1 「フル S2 か一部か」

**S2 に完全対応している。** 攻撃表・B2B charging/surge・コンボ・All-Mini+・180°・ゴミ materialization・
PC +5・GSB +1・margin multiplier が実装済み。TBP I/F は S2 状態
(`pending_garbage` / `b2b_streak` / frame / active 位置)を運ばず、**S2 profile は TBP アダプタから
選択されない**。S2 ラボも「no S2 results」で、公開された S2 対戦結果は無い。

### 7.2 pin を更新すべきか

- **S2 profile は完成している。** 残る作業は接続側(TBP アダプタで S2 profile を選ぶ、posY のずれを直す)。
- **40 行アダプタの保守目的でも `1f41806` はそのままでは使えない**。グリッドは 40 行になるが、
  `posY` が field40 rig の原点で我々の S2 spawn と 2〜3 行ずれる。**posY の追加パッチが必要**。
  空 queue 耐性は魅力だが、spawn の回帰リスクがある。
- **現行の `7b38f51` 固定を維持する判断は安全**。段階 3(attax 95%)は「相手が強くなるほど難しい」ため、
  未検証の S2 profile に pin を移す理由は現時点で無い。
- 更新する場合の推奨: **`1f41806` + `ATTAX_TBP_ROWS=40`** で TBP 接続し、
  `docs/attax-integration.md` の 40 行パッチ節を「新アダプタでは不要」に更新。
  S2 profile は `tools/s2-lab/` の外部ラボ用として**別扱い**にする。

### 7.3 未確定事項

- 新アダプタの `posY = FIELD_ROWS - 21` は field40 rig の原点で、我々の S2 spawn(最下段 row 21)とは
  ピースにより 2〜3 行ずれる。TBP 接続で使う場合は実ゲームで spawn の一致を確認する必要がある。
- S2 profile を TBP で使うには、`profile` 指定 + `b2b_streak` / `pending_garbage` / frame の
  TBP 拡張(作者との I/F 合意)が必要。
- opener phase は `TODO S2-SPEC Unpinned` の未確定値の一部。

## 8. TBP 接続の修正と 1 ゲーム(2026-11-16)

### 8.1 `adapters/attax-s2-tbp.patch`(28 行、SHA-256 `461e4396de5fe046ec908d256df886f9c497faa8fbd7c27d02f46d197ab3049f`)

上流 `1f41806` の TBP アダプタに対する最小パッチ:

1. **S2 profile の選択**: `ATTAX_TBP_PROFILE=s2` のとき `createBot({ profile: "s2" })` を渡す。
   上流は `createBot({ name: "attax" })` で guideline 固定。
2. **posY を S2 spawn に**: `FIELD_ROWS > 20 ? FIELD_ROWS - 22 - maxY : -minY`。
   上流の `FIELD_ROWS - 21` は field40 rig の原点で、我々の S2 spawn(最下段 row 21)とは
   ピースにより 2〜3 行ずれる。

```powershell
cd C:\Users\Hasilan\Projects\attax
git checkout 1f41806cbfe5ed3a5b1df81e6b7aac60f256e9d0
git apply C:\Users\Hasilan\Projects\intetrigence\adapters\attax-s2-tbp.patch
bun run typecheck
```

起動: `ATTAX_TBP_ROWS=40`、`ATTAX_TBP_PROFILE=s2`、`ATTAX_TBP_LOG=<path>`。
`--tbp-visible-rows` は **40**(S2 profile の bot は 40 行グリッドを見る)。

### 8.2 1 ゲームの結果

条件: seed 9001 sheet A、1000 turn、`--ms 100`/8t。

```text
seed-9001-sheet-A: winner=1 reason=TopOut turns=169 verified=true mismatches=0
summary: 1 games, wins 0 losses 1 unresolved 0 no-contest 0, verified 1/1
```

| 項目 | intetrigence | attax(S2) |
|---|---|---|
| 結果 | TopOut 負け | **TopOut 勝ち** |
| attack/piece | 0.882 | **0.929** |
| 受信ゴミ | 69 | 60 |
| think median | 68.5 ms | 41.6 ms |
| spin/attack 監査 | — | **168/168 一致** |

- **forfeit / no-contest / unresolved 0**、replay 検証 1/1。
- S2 profile は有効(攻撃表・コンボ・All-Mini+・180°・deathHeight 21)。attack/piece 0.929。
- **制約**: `createBot` は S2-only フィールド(`b2bChain` / `garbageRows` / `dueThisLock` / `frame`)を
  渡さない(`fighter.ts` のコメント)。**surge の溜め・GSB・margin multiplier は attax の探索に効かない**。
  ゲームの採点(レフェリー)は S2。完全な S2 は `tools/s2-lab/` の外部ラボ用。
- エビデンス: `scripts/evidence/attax-s2-game1/{summary.json,audit/,seed-9001-sheet-A-attax.jsonl}`。

### 8.3 40 行パッチの扱い

- 旧 `adapters/attax-vision-40.patch`(`7b38f51` 用)は **削除**。新 pin では `ATTAX_TBP_ROWS=40` が
  グリッドを 40 行にし、`adapters/attax-s2-tbp.patch` が profile と posY を担う。
- `supports_180`: 新 pin の **S2 profile は true**(`rotate180: true`)、guideline は false。
