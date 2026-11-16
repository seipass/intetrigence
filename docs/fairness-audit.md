# 対戦基盤の公平性監査 — 「S2 非対応の相手」以外の非対称性

- **監査日**: 2026-11-16
- **対象**: `HEAD 59c4e1f` の `crates/core` / `crates/arena` / `crates/tbp` / `adapters` / `scripts` / `docs`
- **目的**: 「S2 基準で Intetrigence 以外が公式に対応していない」以外に、**対戦基盤そのものが作っている不公平**がないかを調べる。
- **方法**: コード監査、仕様・プロトコルとの突き合わせ、既存 evidence の再読、`cargo test --workspace`、CC2 sheet B の 1 ゲーム再実行。

---

## 0. 結論

**「S2 非対応の相手」以外の不公平は存在する。** ただし、それは**ゲームルールの適用**ではなく、次の 3 層に集中している。

| 層 | 状態 | 主な問題 |
|---|---|---|
| **コアルール** | **ほぼ公平** | 攻撃表・採点・B2B・surge・opener・ゴミ・トップアウトは両者に同一。TurnCap の攻撃量勝ち、forfeit 勝ち、Stuck の TopOut 扱いは修正済み |
| **情報インターフェース** | **不公平** | Intetrigence だけが `pending_garbage` / `arrived_garbage` / 正確な `b2b_streak` / `sent_total` / `received_total` を受け取る。TBP の外部ボットにはこれらのフィールドが無い |
| **思考時間・能力設定・監査** | **不公平または不透明** | CC2/clys だけ空候補のリトライと 100 ms の遅延を与えられている。`supports_180` の既定値が仕様と逆。リプレイの rules/protocol 版・相手名・能力フラグ・パッチ SHA が不正確または欠落 |

**最重要の指摘**は **F1（情報非対称）** である。これは「相手が S2 を知らない」という相手側の制約ではなく、**プラットフォームが Intetrigence にだけ S2 の防御・surge に必要な状態を渡している**という、こちら側の設計問題である。

---

## 1. 要約表

| # | 分類 | 重大度 | 一言 |
|---|---|---|---|
| **F1** | 情報 | **高** | Intetrigence だけが受信ゴミ（到着前・到着済み）と正確な B2B streak を見る。外部ボットは見ない |
| **F2** | 思考時間 | **高** | CC2/clys だけ空候補を最大 2 秒リトライする。プロトコルは「1 設置 1 suggest、リトライ禁止」と明記 |
| **F3** | 能力設定 | **高** | CC2/clys の `supports_180` 既定値が `true`。仕様は `false`。forfeit 分類と verify に影響 |
| **F4** | 計算資源 | **中〜高** | Intetrigence は 8 threads / 100 ms。外部ボットの thread 数・suggest delay・retry はレポートに記録されない |
| **F5** | 監査メタデータ | **中** | replay header が古い rules/protocol 版を記録し、slot 1 を常に `attax` と書く。summary も相手を `attax`、TurnCap を `draw` と表示する |
| **F6** | パッチ開示 | **中** | CC2 パッチは hold 探索も変えるが README は「視野・spawn のみ」。CC2/clys のパッチ SHA は round report に記録されない |
| **F7** | 状態同期 | **中** | Clutch Clear で active が押し上がっても TBP の `start` に active 位置が無い。外部ボットは spawn 前提のまま |
| **F8** | プロセス | **中** | `--seeds` override が事前確定 seeds file と併用でき、file の commit を記録したまま別 seeds で走れる。resume の原因分類も強制されない |
| **F9** | ゲート | **中** | 段階 3 は no-contest を 2 件まで許すが、`forfeit-other` は「ラウンド不成立」という別の規定と矛盾。コードは前者に従う |
| **F10** | RNG | **低** | slot 0 と slot 1 のゴミ穴列は独立ではなく、同じ MINSTD 列を 1 draw ずらしたもの |
| **F11** | スピン | **低** | 最大スピン規則は両者同一だが、「Intetrigence には保守的」という文書の説明は誤り。実際はセル提出側（attax）が余分に得をする |
| **F12** | 文書 | **低** | `docs/arena.md` に旧 TurnCap 勝者決定規則が残る。`docs/s2-rule-audit.md` は「実装は未追随」のまま。プロトコル本文にも「TurnCap を勝ちに数える」旧記述が残る |
| **F13** | S2 忠実性 | **低（既開示）** | ゴミ挿入後の active を「押し上げ」ず標準 spawn に再スポーンする。両者同一なので勝敗の公平性には影響しないが S2 ではない |

---

## 2. 公平と確認できた点

先に、**問題が無いと確認できた**ものを列挙する。ここを混ぜると「何が不公平か」がぼやける。

### 2.1 コアルールは両者同一

`crates/core` の純粋エンジンを両者が共有する。

- 攻撃表・combo 乗数・B2B Charging・surge・PC・GSB・All-Mini+（`crates/core/src/scoring.rs`）
- ゴミ cap 8 / 遅延 20 frame / 相殺 / opener 2 倍 / change-on-attack（`crates/core/src/garbage.rs`）
- 7-bag・hold・SRS+・180° kick・Clutch Clear・block-out（`crates/core/src/{bag,hold,kick,game}.rs`）
- トップアウトは block-out のみ。lock-out は無効（`crates/core/src/game.rs`）

レフェリーは**両者の設置を同じ `Engine::apply` で処理**し、スピンも攻撃も再計算する（`crates/arena/src/engine.rs`）。外部ボットの申告 spin/attack は監査用で、ゲーム進行には使わない。

### 2.2 勝者決定に「発明規則」は残っていない

- `TurnCap` は `winner = None`、`reason = "TurnCap"`（`crates/arena/src/referee.rs`）
- `Forfeit` は `winner = None`、no contest
- `Stuck` は `winner = None`、`reason = "Stuck"`
- 「攻撃量が多い方を勝ち」「forfeit を勝ちに数える」「Stuck を TopOut と呼ぶ」は**コードから削除済み**

`docs/s2-rule-audit.md` の F1〜F8 のうち、F2/F3/F4/F5/F7/F8 はコード側で修正されている（文書が未追随。F12 参照）。

### 2.3 seed・シート・ターン条件は対称

- 1 seed = 1 ゲーム、両シートで同じ `piece_seed`（`crates/core/src/rng.rs::derive_seeds`）
- ゴミ穴 seed は slot 固定（slot 0 = Intetrigence、slot 1 = 相手）。シートを変えても同じボットは同じ穴列
- シート A/B で seat 1/2 を入れ替える（`crates/arena/src/engine.rs::Sheet::order`）
- 両者とも 1000 turn cap、12 frame/turn、preview 5 + hold
- 開始盤面は `official` では空（`Profile::Official`）

### 2.4 監査・再現の仕組みは強い

- replay は seed / sheet / seats / slots / bots / rules / protocol / turn_cap / frames / preview / profile を記録
- `verify-replay` は着手列を再実行し、spin / lines / events / sent / cancelled / hash / result を照合
- 評価 seeds は事前コミット必須。`check_seed_discipline` が build 祖先・seeds commit・clean tree を検査
- `measurement-lock.sh` で同時測定を排他
- 相手の decision log / transcript の内容を検査（`measurement_complete`）

### 2.5 相手の盤面・キューは Intetrigence にも見えない

`View` は自分の盤面・自分の active・自分の hold・自分の preview・自分のゴミ queue だけを持つ。相手の盤面や相手のゴミ穴は含まない（`crates/arena/src/engine.rs::View`）。この点は公平。

---

## 3. 不公平・不整合の詳細

### F1. 情報非対称 — Intetrigence だけが S2 の防御・surge 状態を受け取る（高）

#### 事実

`View` は Intetrigence に次のフィールドを渡す（`crates/arena/src/engine.rs:79-107`）。

| フィールド | 意味 | TBP `StartPayload` にあるか |
|---|---|---|
| `pending_garbage` | 受信ゴミ合計（到着前を含む） | **無い** |
| `arrived_garbage` | すでに到着し、次のロックで挿入され得る行数 | **無い** |
| `b2b_streak` | 正確な B2B streak（surge の量と放出条件に使う） | **bool の `back_to_back` だけ** |
| `sent_total` | 累計送信行数 | **無い** |
| `received_total` | 累計受信行数 | **無い** |
| `pieces_placed` | 設置数（opener 判定） | **無い**（自分で数えられる） |
| `multiplier` | frame 由来のゴミ倍率 | **無い**（turn から推測可能） |

TBP の `StartPayload` は `board` / `queue` / `hold` / `combo` / `back_to_back` / `randomizer` だけ（`crates/tbp/src/message.rs:170-180`）。`PlayPayload` は move、`NewPiecePayload` は piece だけで、ゴミや B2B streak を運ぶメッセージは無い。

Intetrigence 側はこの情報を**実際に使っている**。

- `BotAgent::context` が `view.pending_garbage` / `view.arrived_garbage` / `view.b2b_streak` / `view.sent_total` / `view.received_total` を `EvalContext` に入れる（`crates/arena/src/bot_agent.rs:90-99`）。
- `EvalContext` は `incoming` / `arrived` を防御・生存判定に、`b2b_streak` を surge charge と B2B 攻撃に、`sent_total - received_total` をレース項に使う（`crates/bot/src/eval.rs:508-548, 679-704`）。
- 探索は `incoming` / `arrived` を相殺分だけ減らしながら先読みする（`crates/bot/src/eval.rs:590-605`）。

外部ボット側の実装も、この情報を持たないことを認めている。

- attax: `pendingGarbage` は常に 0（`docs/attax-integration.md` §5.2）。
- CC2 / clys: TBP の `back_to_back` bool と `combo` だけ。ゴミ queue は無い（`docs/cc2-integration.md` / `docs/clys-integration.md`）。

#### 影響

これは **S2 の勝ち筋そのもの**に効く。

- **防御**: 到着済みゴミと到着前ゴミを区別できると、「今消すべきか、次のピースまで待つか」を正しく選べる。外部ボットは到着前ゴミが見えないため、同じ判断ができない。
- **surge**: S2 の surge は「B2B streak が 4 以上で charge、5 以上で break 時に放出、量は `N_before - 1`」。bool だけでは**いつ・どれだけ放出するか**を計画できない。Intetrigence は正確な streak で探索できる。
- **レース**: `sent_total - received_total` を評価に入れられる。外部ボットは正確な累計を持たない。

特に **sheet B（Intetrigence が seat 2）** では、seat 1 の攻撃が同じ turn のうちに `pending_garbage` として見える。Intetrigence はそれを見て相殺する手を選べるが、外部ボットは見えない。プロトコルは「seat 2 がわずかに有利」とだけ書いている（`docs/s2-referee-protocol.md:280`）が、**Intetrigence だけがその有利を情報として使える**。シート入れ替えは seat の有利を相殺するが、この情報有利は相殺しない。

#### 判定

**プラットフォーム起因の不公平。** 「相手が S2 を知らない」こととは別で、**S2 を知っていても TBP では受け取れない状態**を Intetrigence だけが受け取っている。

#### 修正案

1. **TBP を拡張**して `start` に `pending_garbage` / `arrived_garbage` / `b2b_streak` / `multiplier` / `pieces_placed` / `sent_total` / `received_total` を追加する（外部ボットが無視しても、少なくとも仕様としては同等）。
2. または、**公式マッチでは Intetrigence の `View` を TBP 相当に制限**する（`pending_garbage` / `arrived_garbage` / 正確な `b2b_streak` / `sent_total` / `received_total` を渡さない）。強さは落ちるが、情報条件は揃う。
3. どちらを選ぶかをプロトコルに明記し、レポートに「情報条件は同等か、非対称か」を書く。

---

### F2. CC2/clys だけ空候補をリトライする（高）

#### 事実

`TbpAgent::suggest` は `retry_empty_suggest_ms` の間、空の `suggestion` を繰り返し要求する（`crates/arena/src/tbp_agent.rs:275-290`）。

```rust
let retry_ms = self.config.retry_empty_suggest_ms;
let deadline = Instant::now() + Duration::from_millis(retry_ms);
let payload = loop {
    let payload = host.suggest()?;
    if !payload.moves.is_empty() || retry_ms == 0 || Instant::now() >= deadline {
        break payload;
    }
    sleep(5ms);
};
```

既定値:

- CC2: `retry_empty_suggest_ms = 2000`（`crates/arena/src/bin/round.rs:94`、`scripts/run-round.sh` の `--cc2-retry-empty-ms` は未指定でも 2000）
- clys: `2000`（`scripts/run-round.sh:79`、`scripts/tuning-smoke.sh:55`）
- attax: `0`（`AttaxConfig` / `TbpAgentConfig::new` の既定）

プロトコルは明確に禁止している。

> **1 設置につき `suggest` は 1 回だけ**。リトライしない（リトライすると `think_ms` の意味が壊れ、公平性も崩れる）。
> — `docs/s2-referee-protocol.md:976`

別の節では「`suggestion` は 1 turn に 1 回（リトライは内部）」と書いており（`docs/s2-referee-protocol.md` §8.3.1.2）、**プロトコル自身が矛盾**している。

#### 影響

- CC2/clys は、空候補という**失敗状態**から追加の `suggest` で復帰できる。attax は同じ状況で forfeit になる。
- リトライは transcript に `suggest` として残らない（host 側のメッセージは記録されない）。`measurement_complete` は `suggestion` の件数だけを見るので、**リトライの有無を監査できない**。
- `think_ms` には全リトライ分が入るが、レポートには `retry_empty_suggest_ms` が記録されない。読み手は「なぜ CC2 の think が 100 ms なのか」「リトライがあったのか」を判断できない。
- コミット `18b32d2` は「`TbpAgent::begin_round` が throwaway suggest を送る」と説明しているが、**そのコードは存在しない**（`git show 18b32d2:crates/arena/src/tbp_agent.rs` に `begin_round` 内の `suggest` は無い）。実際の復帰機構はこのリトライループである。コミットメッセージとコードが一致していない。

#### 判定

**プラットフォーム起因の不公平。** CC2/clys の start race を避けるための互換措置としては理解できるが、**プロトコルの公平性規則を破っており、attax には同じ救済が無い**。また、その設定がレポートに残らない。

#### 修正案

1. **公式ラウンドではリトライを禁止**し、代わりに `start` 後に 1 回だけ「捨て suggest」を送る warm-up を全 TBP ボットに共通実装する（コミットメッセージが意図した形）。warm-up も `suggest` 回数に含めるなら、プロトコルに明記する。
2. リトライを残すなら、**全ボットに同じポリシー**を適用し、`retry_empty_suggest_ms` を replay header / round report に記録する。
3. `docs/s2-referee-protocol.md` §6.3 と §8.3.1.2 の矛盾を解消する。

---

### F3. CC2/clys の `supports_180` が仕様と逆（高）

#### 事実

仕様・統合ドキュメントは、3 相手すべてが 180° を持たないと明記している。

- `docs/cc2-integration.md` §13: **`supports_180 = false`**
- `docs/clys-integration.md` §13: **`supports_180 = false`**
- `docs/attax-integration.md` §14: **`supports_180 = false`**
- `PLAN.md:28`: 「attax・CC2・clys のいずれも 180° 回転を持たない（`supports_180 = false`）」

しかしコードの既定値は逆。

- `crates/arena/src/harness.rs:96` `TbpOpponentConfig::new` → `supports_180: true`
- `crates/arena/src/bin/round.rs:92` `cc2_supports_180 = true`
- `crates/arena/src/bin/round.rs:102` `clys_supports_180 = true`
- `scripts/run-round.sh:48-49` `CC2_SUPPORTS_180` / `CLYS_SUPPORTS_180` の既定 `true`
- `scripts/tuning-smoke.sh` は `--tbp-supports-180` を渡さないため、`arena match` の既定 `true`（`crates/arena/src/bin/arena.rs:79`）
- `crates/arena/src/bin/arena.rs:638` `reclassify` の既定 `true`

既存 evidence もこの誤設定を記録している。

- `scripts/evidence/cc2-sheet-b-check/replays/seed-9001-sheet-B.jsonl` の header:
  `"bots":{"1":{"name":"Cold Clear 2",...,"supports_180":true}}`
- `scripts/evidence/cc2m-det8w96/replays/seed-9001-sheet-A.jsonl` も同じ。

#### 影響

`supports_180` は `classify_forfeit` と `verify_result` の `has_legal_move` に使われる（`crates/arena/src/referee.rs:737-771`、`crates/arena/src/replay.rs:745-756`）。

- 180° でしか到達できない可視配置がある場合、`supports_180=true` だと「合法手がある」と判定され、`forfeit-other` になる。`false` なら `forfeit-vision` になる。
- `forfeit-other` は「アダプタ故障 = ラウンド不成立」、`forfeit-vision` は「不戦」なので、**プロトコル上の扱い（不戦か不成立か）が変わる**。
- `verify_replay` は header の `supports_180` で `forfeit_cause` を再計算する。誤設定の replay は、正しい設定で再分類すると verify mismatch になり得る。
- ゲームの勝敗そのものは変わらない（どちらも `winner = None`）が、**記録の意味とラウンドの扱い**が変わる。

#### 判定

**設定バグ。意図的な有利ではないが、監査上は高リスク。** 既存 evidence の `supports_180` は信用できない。

#### 修正案

- `TbpOpponentConfig::new` の既定を `false` にする（能力不明の TBP ボットは保守側に倒す）。
- `round.rs` / `run-round.sh` / `tuning-smoke.sh` / `arena match` / `reclassify` の CC2/clys 既定を `false` にする。
- 仕様とコードの一致をテストで固定する（例: `docs/cc2-integration.md` の `supports_180 = false` と CLI 既定を突き合わせる）。

---

### F4. 思考時間・スレッド数・リトライの条件が記録されず、実効条件が揃っていない（中〜高）

#### 事実

| ボット | 実効 think budget | スレッド | リトライ | レポート記録 |
|---|---|---|---|---|
| Intetrigence | 100 ms（deterministic d8 w96 は max 79.2 ms） | **8** | なし | `threads` / `deterministic` / `depth` / `width` は記録 |
| CC2 | `suggest_delay_ms = 100` + 自前探索 | 実質 1（外部プロセス） | **2000 ms** | **delay/retry/threads は記録なし** |
| clys | adapter が `suggest` を 100 ms 遅延 | 実質 1（外部プロセス） | **2000 ms** | **delay/retry/threads は記録なし** |
| attax | 自前探索（実測 median ~30 ms） | 実質 1（Bun） | なし | `attax.command_line` / `bun_version` は記録。threads は無い |

`BotRecord` / `Header` / `StageReport` / `MatchSummary` には、**相手の `suggest_delay_ms` / `retry_empty_suggest_ms` / `threads` を入れる場所が無い**（`crates/arena/src/replay.rs:99-111`、`crates/arena/src/round.rs:84-120`、`crates/arena/src/harness.rs:444-489`）。`Header.threads` は Intetrigence の探索スレッド数だけ（`crates/arena/src/harness.rs:1570`）。

#### 影響

- Intetrigence は 8 スレッドで 100 ms 使える。外部ボットは実質 1 スレッド。**同じ 100 ms でも仕事量が違う**。
- CC2/clys は 100 ms の遅延 + 最大 2 秒のリトライ。attax は追加予算なし。**相手ごとに実効条件が違う**。
- レポートには Intetrigence の `threads` と `deterministic` は出るが、相手の条件は出ない。読み手は「同じ条件で戦った」と誤解し得る。
- プロトコルは「思考時間・スレッド数を毎ゲーム記録・公開」としている（`PLAN.md:216`）が、実装は相手側を記録していない。

#### 判定

**条件の非対称。** 100 ms cap は Intetrigence 側の上限としては守られているが、**相手の実効予算・スレッド数・リトライ**は揃っていないし、開示もされていない。

#### 修正案

- `BotRecord` に `threads` / `suggest_delay_ms` / `retry_empty_suggest_ms` / `deterministic` を追加し、`StageReport` / `summary.json` / `round.md` に出す。
- 公式ラウンドでは、全ボットに同じ `suggest_delay_ms` / `retry` ポリシーを適用するか、**相手ごとに違うことを明示**する。
- Intetrigence のスレッド数を相手に合わせる（1 スレッド）選択肢も検討する。8 スレッドを維持するなら、その旨を結論に明記する。

---

### F5. 監査メタデータが古い・相手名を間違える（中）

#### 事実

**(a) rules / protocol の版が古い**

`MatchConfig::new` / `RoundConfig::new` は次の固定値を持つ。

- `rules: "docs/s2-spec.md@v1.3.11"`（`crates/arena/src/harness.rs:167`、`crates/arena/src/referee.rs:55`）
- `protocol: "docs/s2-referee-protocol.md@v1.9"`（`crates/arena/src/harness.rs:168`、`crates/arena/src/referee.rs:56`）

現在の文書は **spec v1.3.17 / protocol v1.53**。既存 evidence の header も `v1.3.11` / `v1.9` のまま（例: `scripts/evidence/cc2-sheet-b-check/replays/seed-9001-sheet-B.jsonl`）。

**(b) slot 1 を常に `attax` と記録する**

`header()` は `slots` をハードコードする（`crates/arena/src/referee.rs:543-546`）。

```rust
slots.insert("0".to_string(), "intetrigence".to_string());
slots.insert("1".to_string(), "attax".to_string());
```

CC2/clys の replay でも `"slots":{"1":"attax"}` になる。`bots[1].name` は正しく `Cold Clear 2` なのに、`slots` だけ嘘になる。`render.rs` は `header.slots` を勝者名に使う（`crates/arena/src/render.rs:886-899`）ので、動画にも `attax` と出る。

**(c) summary の勝者ラベル**

`MatchSummary::to_markdown` は `Some(1)` を常に `"attax"` と表示する（`crates/arena/src/harness.rs:1234`）。CC2 が勝ったゲームは `| 9001 | A | attax | TopOut |` になる（`scripts/evidence/cc2-after-search-fix/summary.md`）。

**(d) TurnCap / Forfeit を `draw` と表示する**

`None` を `"draw"` と表示する（`crates/arena/src/harness.rs:1236`）。実際の TurnCap ゲームが `| 9010 | B | draw | TurnCap |` と出ている（`scripts/evidence/cc2m-det8w96/summary.md:32`）。プロトコルは「`draws` という名前を使わない」と明記している（`docs/s2-referee-protocol.md:1080, 1477`）。

**(e) deterministic の note が常に「非決定論」**

`run_match` は常に次の note を追加する（`crates/arena/src/harness.rs:1651-1654`）。

> The intetrigence search is non-deterministic (anytime, 8 threads): the same seed can produce different games; the replay is the record.

しかし `cc2m-det8w96` の header は `"deterministic":true`。summary の note と header が矛盾する。`threads` も固定文字列 `8` で、実際の `--threads` を反映しない。

**(f) round report の `W-L-D` と `bun 1.4.0`**

- `round.rs` の表ヘッダは `W-L-D`（`crates/arena/src/round.rs:976`）。プロトコルは `draws` を禁止している。
- 環境行は `bun 1.4.0` をハードコード（`crates/arena/src/round.rs:966`）。実際の Bun 版を測っていない。
- `win breakdown` は `forfeit_unknown` を出さない（`crates/arena/src/round.rs:764`）。プロトコルは `forfeit_unknown` を要求（`docs/s2-referee-protocol.md:1080`）。

#### 影響

ゲームの勝敗には影響しないが、**監査の信頼性**に影響する。

- rules/protocol 版が古いと、どのルールで走ったゲームか分からない。v1.9 と v1.53 では TurnCap/forfeit/deterministic の扱いが大きく違う。
- slot 1 が `attax` だと、CC2/clys の replay を機械的に集計するとき相手を誤認する。
- `draw` は「未決着」と「不成立」を混同する。プロトコルが最も避けたい表現。
- deterministic の note が逆だと、公式ラウンドの再現性の主張が崩れる。

#### 判定

**監査バグ。** ゲームの公平性そのものではないが、公平性を**検証できない**状態を作る。

#### 修正案

- rules/protocol の版を文書から読むか、少なくとも現在の版に更新する。
- `header.slots` を `bots[slot].name` から作る。
- summary の勝者ラベルを `self.bots[1].name` にする。
- `None` は `reason` に応じて `unresolved` / `no contest` と表示する。
- note を `deterministic` / `threads` に応じて切り替える。
- `W-L-D` を `W-L (U, NC)` に直す。`bun 1.4.0` を実測値にする。`forfeit_unknown` を出す。

---

### F6. パッチ開示が不正確・不完全（中）

#### 事実

**(a) README の「視野・spawn のみ」は CC2 では誤り**

README は「いずれも視野・spawn 位置のプロトコル互換修正のみで、探索・評価・重みには触れません」と書く（`README.md:112`）。しかし `adapters/cc2-tbp-compat.patch` は空 hold の表現のために **`Freestyle` の hold 候補列挙と eval の cutout** を変更する（パッチの `src/bot/freestyle.rs` / `src/data.rs`）。これは探索・評価に触れている。`docs/cc2-integration.md` §1.2 は正しく「空 hold の表現 + spawn」と書いているが、README の要約が誤っている。

**(b) CC2/clys のパッチ SHA が round report に残らない**

`round.rs` は CC2/clys の `patch_sha256` を常に `None` にする（`crates/arena/src/bin/round.rs:468, 477, 496, 505`）。`run-round.sh` にも `--cc2-patch-sha256` / `--clys-patch-sha256` が無い。プロトコルはパッチの SHA-256 を必須にしている（`docs/s2-referee-protocol.md` §7.3 / §8.5）。

**(c) CC2/clys の commit / command / config SHA が既定で空**

`run-round.sh` は `CC2_COMMIT` / `CLYS_COMMIT` をユーザーが設定しない限り空で渡す。`StageReport.commit` は `None` になる。プロトコルは相手のビルドコミットと起動コマンドを必須にしている（`docs/s2-referee-protocol.md` §8.5）。CC2 の `src/default.json` SHA、clys の `config.json` SHA も round report には入らない。

#### 影響

パッチが探索・評価に触れているかどうかは、公平性の主張の根拠になる。README の誤要約は、**パッチの方向性を読み違えさせる**。SHA が無いと、どのパッチで走ったか再現できない。

#### 判定

**開示の不備。** パッチ自体は「空 hold の表現」という許容範囲（プロトコル §7.3）だが、説明と記録が不正確。

#### 修正案

- README の「視野・spawn のみ」を「視野・spawn・空 hold の表現」に直す。
- `run-round.sh` / `round.rs` に `--cc2-patch-sha256` / `--clys-patch-sha256` を追加し、既定値をパッチファイルの SHA にする。
- CC2/clys の commit を `git -C` で自動検出する（attax と同じ）。
- config.json / default.json の SHA を `StageReport` に追加する。

---

### F7. Clutch Clear の active 位置が TBP 外部ボットに伝わらない（中）

#### 事実

S2 の Clutch Clear は、直前のライン消去後に標準 spawn が衝突すると、active を 1 行ずつ上へ押し上げる（`crates/core/src/game.rs:208-247`）。`Game::lock` は `clear.lines > 0` のとき `spawn(next, true)` を呼ぶ。

TBP の `start` は board / queue / hold / combo / back_to_back / randomizer だけで、**active の位置を持たない**（`crates/tbp/src/message.rs:170-180`）。`TbpAgent::commit` は `garbage_inserted` のときだけ `start` を再送し（`crates/arena/src/tbp_agent.rs:355`）、Clutch では再送しない。外部ボットの `snapshot()` は常に標準 spawn を仮定する（`docs/attax-integration.md` §5.2）。

Intetrigence 側は `View.active` で実際の位置を受け取り、検索が標準 spawn で失敗したら `append_fallback` で実際の active から合法手を生成する（`crates/arena/src/bot_agent.rs:250-260`）。

#### 影響

Clutch が起きたゲームでは、外部ボットの内部盤面とレフェリーの active 位置がずれる。外部ボットが標準 spawn 前提で出した手が、レフェリーの到達可能性検証で拒否される可能性がある。拒否されれば `forfeit-other`（ラウンド不成立）になる。既存 evidence では `bot_fallbacks` は 0 件だが、これは Intetrigence 側のカウンタであり、外部ボットの Clutch 遭遇を保証しない。

#### 判定

**TBP の構造的制約。** 「外部ボットが S2 非対応」の一部とも言えるが、**プラットフォームが active 位置を送らない**ことで増幅している。Clutch は S2 のルールなので、S2 対応を名乗るなら状態を運ぶ必要がある。

#### 修正案

- TBP の `start` に `active`（kind / rotation / x / y）を追加する。外部ボットが無視しても、少なくとも送る。
- または、TBP 外部ボットとの公式マッチでは Clutch を無効化し、そのことをプロトコルに明記する（S2 忠実性は落ちる）。
- `clutch_fallbacks` を外部ボット側にも記録できるようにする。

---

### F8. `--seeds` override と resume 原因分類の抜け道（中）

#### 事実

**(a) seeds override**

`cmd_run` は `check_seed_discipline(seeds_file, build, !dry_run)` で seeds file を検証した後、`seeds_override` があれば**その値を使う**（`crates/arena/src/bin/round.rs:378-386`）。

```rust
let seeds_commit = check_seed_discipline(seeds_file.as_deref(), build_commit.as_deref(), !dry_run)?;
let (seeds, seeds_source) = match (seeds_override, &seeds_file) {
    (Some(seeds), _) => (seeds, "<--seeds override>".to_string()),
    ...
};
```

`--seeds-file` と `--seeds` を同時に渡すと、**file の commit を記録したまま、別の seeds で本番ラウンドを走らせられる**。`seeds_source` は `<--seeds override>` になるが、`seeds_commit` は file の commit のまま。プロトコルは「評価 seeds は実行前にコミットで確定」「確定後に選び直さない」としている（`docs/s2-referee-protocol.md` §1.5）。

**(b) resume 原因**

`RoundReport.resume_reason` は自由文字列で、`resume_cause` の enum が無い。`check_resume_compatibility` は build / seeds / seeds_commit だけを検査する（`crates/arena/src/round.rs:1088-1115`）。プロトコルは「referee-defect のラウンドは再開不可」「external のみ再開可」としている（`docs/s2-referee-protocol.md` §1.4）。コードはこの分類を強制しない。

#### 影響

どちらも**結果を見てから都合の良い seeds / 再開を選ぶ**余地を作る。ゲーム中の不公平ではないが、**評価の公平性**を損なう。

#### 判定

**プロセスの抜け道。** 監査可能性は `seeds_source` / `resume_reason` で部分的に残るが、強制力が無い。

#### 修正案

- strict mode では `--seeds` override を**禁止**する（dry run のみ許可）。
- `resume_cause` を enum にし、`referee-defect` では resume を拒否する。
- `seeds_commit` が override と一致しない場合はエラーにする。

---

### F9. 段階 3 の no-contest 規定が `forfeit-other` 不成立規定と矛盾（中）

#### 事実

- 段階 3 は `wins_topout >= 48` かつ `turncap == 0`。**no-contest は 2 件まで許す**（`docs/s2-referee-protocol.md:104, 109`）。
- コードの `gate_check` は index 2 で `topout_wins >= gate` だけを見る（`crates/arena/src/round.rs:403-410`）。`no_contest` の内訳は見ない。
- 一方、プロトコルは「**`forfeit-other` が 1 件でもあればラウンド不成立**」と明記している（`docs/s2-referee-protocol.md:127, 137, 1084, 1218-1220`）。

したがって、**48 TopOut wins + 2 forfeit-other** はコードでは通過するが、プロトコルの別節では不成立。`forfeit-vision` と `forfeit-other` を区別せず `no_contest` として扱っているのが原因。

#### 影響

相手のアダプタ故障（`forfeit-other`）が 2 件あっても、48 勝していれば段階 3 が通る。これは「50 ゲームの実力測定」としての妥当性を損なう。`forfeit-vision` なら不戦として許容できるが、`forfeit-other` はゲーム自体が無効。

#### 判定

**ゲートの抜け。** プロトコル内の矛盾をコードが一方に寄せている。

#### 修正案

- 段階 3 でも `forfeit_other == 0` を要求する（`forfeit_vision` のみ 2 件まで許す）。
- または、`forfeit-other` を許すならプロトコル §7.1.4 / §7.4 を改訂し、その理由を明記する。

---

### F10. ゴミ穴 RNG の slot 0 / slot 1 が独立でない（低）

#### 事実

`derive_seeds` は 1 本の MINSTD を前進させて 3 つの seed を作る（`crates/core/src/rng.rs:95-104`）。

```rust
let mut base = MinStd::new(seed);
let piece_seed = base.state();          // seed
let garbage_seed_0 = base.next_u32();   // a * seed
let garbage_seed_1 = base.next_u32();   // a^2 * seed
```

`GarbageQueue::new(garbage_seed)` は `MinStd::new(seed)` で state を初期化し、`next_float()` のたびに `a` を掛ける。したがって:

- slot 0 の 1 回目 = `a^2 * seed`
- slot 1 の 1 回目 = `a^3 * seed`
- slot 0 の 2 回目 = `a^3 * seed`

つまり **slot 0 の列は slot 1 の列を 1 draw ずらしたもの**。実際に seed 1 / 12345 / 30001 で確認した。

#### 影響

両者の穴列は独立ではなく、同じ系列の位相違い。分布は一様なので**系統的な有利不利ではない**が、統計的には相関している。両者が同じタイミングでゴミを消費すると、似た穴列を受け取る可能性がある。ゲームの公平性への影響は小さいが、「独立なランダム」と主張するなら正確ではない。

#### 判定

**設計上の注意点。** 不公平とまでは言えないが、RNG の独立性を主張しない方がよい。

#### 修正案

- slot ごとに別の導出（例: `SplitMix64(seed ^ 0x...slot)`）を使う。ただし replay 形式と既存 evidence が変わるので、プロトコル版を上げて移行する。
- 現状を維持するなら、`docs/s2-rng-derivation.json` に「slot 0 と slot 1 は同じ MINSTD 列の位相違い」と明記する。

---

### F11. スピン最大ラベルの説明が逆（低）

#### 事実

レフェリーはセル集合ごとに**到達可能な経路の最大スピン**を採る（`crates/arena/src/movegen.rs` の `unique_by_cells`、`docs/s2-referee-protocol.md` §4.2.1）。`claimed_spin` は監査用で、採点には使わない（`crates/arena/src/engine.rs::apply`）。

`docs/arena.md:139-140` は「Intetrigence は spin を回転終端の入力列で裏付けるので、寛容さの恩恵を受けるのはセルだけを提出する attax 側で、我々には保守的」と書く。しかし**レフェリーは Intetrigence の claimed spin も使わず、同じ最大スピン規則を適用する**。Intetrigence の cells がスピン経路で到達可能なら、同じようにスピンとして採点される。実際、既存 evidence では Intetrigence の spin mismatch は 0、attax/CC2 は `none → mini` の不一致が出ている（例: `scripts/evidence/cc2m-det8w96/summary.md` の CC2 94/5585）。これは「Intetrigence が保守的」ではなく「CC2/attax の movegen がスピンを過小申告している」という違い。

#### 影響

ゲームの勝敗には影響しないが、**どちらに有利かを読み違える**。文書の説明を信じると「Intetrigence はスピンで損している」と誤解する。

#### 判定

**文書の誤り。** 規則自体は両者同一で公平。

#### 修正案

- `docs/arena.md` の「我々には保守的」を削除し、「レフェリーは両者に同じ最大スピン規則を適用する。セルだけを提出するボットは、自分の movegen がスピンを過小評価していると余分に得をする」と書き直す。

---

### F12. 文書の旧規則・旧実装状態が残っている（低）

#### 事実

- `docs/arena.md:72`: 「§6.3 turn cap | 受信ゴミ少 → 送信多 → 引き分け」— **削除済みの旧規則**。現在は `winner = None`。
- `docs/s2-rule-audit.md:18, 31, 144`: 「`End::Forfeit` は `winner = Some`」「実装は未追随」— 現在は `None`。
- `docs/s2-referee-protocol.md:23`: 「forfeit を勝ちとして数え、内訳を出す」— 現在は no-contest。
- `docs/s2-referee-protocol.md:983, 1034`: 「勝ちに数えるのは `TopOut` と `TurnCap` だけ」— 現在は TurnCap も数えない。
- `README.md:112`: CC2 パッチを「視野・spawn のみ」と要約 — 実際は空 hold も変更。
- `docs/attax-integration.md:106`: `attax-40-row.patch` — 実際のファイル名は `attax-vision-40.patch`。
- `docs/cc2-integration.md:65`: `cc2-tbp-empty-hold.patch` — 実際は `cc2-tbp-compat.patch`。
- `docs/clys-integration.md:68`: `C:UsersHasilanProjects...` — パス区切りが欠落。
- `docs/s2-referee-protocol.md` v1.23 の履歴: 「attax は 40 行視野、視野拡張パッチは不要」— 現在の §7.3.1 と矛盾。

#### 影響

読み手が「旧 TurnCap の攻撃量勝ちがまだ残っている」「forfeit が勝ちに数えられている」と誤解する。公平性監査の前提を壊す。

#### 判定

**文書バグ。** コードは修正済みだが、文書が追いついていない。

#### 修正案

- `docs/arena.md` の旧 TurnCap 行を現在の `winner = None` に直す。
- `docs/s2-rule-audit.md` の結論に「v1.2 の決定は実装済み。現在の状態はコードを参照」と追記する。
- プロトコルの TOC / §7.1.3 の「TurnCap を勝ちに数える」を削除する。
- README の CC2 パッチ要約を直す。パッチファイル名・パスを直す。
- v1.23 の履歴に「後に §7.3.1 で訂正」と注記する。

---

### F13. ゴミ挿入後の active 再スポーンは S2 ではない（低・既開示）

#### 事実

`Game::insert_garbage` は、ゴミ挿入後に active を**標準 spawn に再スポーン**する（`crates/core/src/game.rs:456-490`）。S2 の「押し上げ」ではない。`docs/core-rule-audit.md:222-224` が「我々の取り決め」として開示している。

#### 影響

両者に同じ規則なので、勝敗の公平性には影響しない。ただし、高スタックで標準 spawn が衝突すると、S2 なら押し上げで生存できる局面でもトップアウトする。S2 の再現ではない。

#### 判定

**S2 忠実性の問題。公平性の問題ではない。** 既に開示済み。

#### 修正案

- 現状維持なら、レポートの「S2 との差」に明記する。
- S2 に合わせるなら、active を押し上げる実装に変え、TBP 外部ボットには active 位置を送る（F7 と同じ）。

---

## 4. 検証

### 4.1 テスト

`cargo test --workspace` は全テスト成功（2026-11-16、HEAD 59c4e1f）。

### 4.2 CC2 sheet B の再実行

現在のコードで、CC2 sheet B を 1 ゲーム実行した。

```sh
./target/release/arena.exe match \
  --seeds 9001 --sheets B --turns 1000 --bot search --ms 100 --threads 8 \
  --out /tmp/cc2b \
  --tbp-program /c/Users/Hasilan/Projects/cold-clear-2/target/release/cold-clear-2.exe \
  --tbp-cwd /c/Users/Hasilan/Projects/cold-clear-2 \
  --tbp-visible-rows 40 --tbp-suggest-delay-ms 100 --tbp-retry-empty-ms 2000 \
  --tbp-supports-180 false --tbp-name cc2
```

結果: `winner=Some(0) reason=TopOut turns=531 verified=true`。transcript は 531 suggestions / 531 plays で、リトライは発生していない。`supports_180=false` で正常に動作する。

これは「CC2 sheet B が常に forfeit する」という P0 が**現在のコードでは再現しない**ことを示す。ただし、コミット `18b32d2` が主張する warm-up はコードに無く、実際の復帰機構は F2 のリトライである。

### 4.3 既存 evidence の確認

- `scripts/evidence/cc2-sheet-b-check/replays/seed-9001-sheet-B.jsonl`: `slots[1] = "attax"`、`bots[1].name = "Cold Clear 2"`、`supports_180 = true`、`deterministic = true`。
- `scripts/evidence/cc2-sheet-b-check/summary.md`: 「The intetrigence search is non-deterministic (anytime, 8 threads)」— header は deterministic。
- `scripts/evidence/cc2m-det8w96/summary.md`: TurnCap ゲームが `draw`、note が非決定論。
- `scripts/evidence/cc2-after-search-fix/summary.md`: CC2 の勝ちが `attax`。
- `scripts/evidence/cc2m-det8w96/summary.md`: CC2 の spin mismatch 94/5585、Intetrigence 0/5576。

---

## 5. 推奨修正（優先順）

1. **F1（情報非対称）**: TBP を拡張するか、公式マッチで Intetrigence の `View` を TBP 相当に制限する。どちらかをプロトコルに明記する。
2. **F2（リトライ）**: 公式ラウンドではリトライを禁止し、warm-up を全ボット共通にする。残すなら全ボット同一ポリシー + レポート記録。
3. **F3（supports_180）**: CC2/clys の既定を `false` にし、仕様との一致テストを追加する。
4. **F4（思考時間）**: 相手の `threads` / `suggest_delay_ms` / `retry_empty_suggest_ms` / `deterministic` を `BotRecord` / `StageReport` / `summary` に記録する。
5. **F5（監査メタデータ）**: rules/protocol 版、slot 名、勝者ラベル、`draw`、deterministic note、`W-L-D`、`bun 1.4.0`、`forfeit_unknown` を修正する。
6. **F6（パッチ開示）**: README を直し、CC2/clys のパッチ SHA・commit・config SHA を記録する。
7. **F7（Clutch）**: TBP `start` に active を追加するか、公式マッチで Clutch を無効化する。
8. **F8（seeds/resume）**: strict mode で `--seeds` override を禁止し、`resume_cause` を enum 化する。
9. **F9（段階 3）**: `forfeit_other == 0` を要求するか、プロトコルを改訂する。
10. **F10〜F13**: RNG の位相、スピン説明、旧文書、S2 忠実性の注記を直す。

---

## 6. 最終判定

**「S2 基準で Intetrigence 以外が公式に対応していない」以外の不公平は、ある。**

- **コアルールは公平**。攻撃表・採点・B2B・surge・opener・ゴミ・トップアウトは両者に同じ。TurnCap の攻撃量勝ち、forfeit 勝ち、Stuck の TopOut 扱いは修正済み。
- **情報インターフェースは不公平**。Intetrigence だけが受信ゴミ（到着前・到着済み）と正確な B2B streak を受け取る。外部ボットは TBP の `back_to_back` bool しか持たない。これは「相手が S2 を知らない」こととは別の、**プラットフォーム側の非対称**。
- **思考時間・能力設定は不公平または不透明**。CC2/clys だけ空候補リトライと 100 ms 遅延。`supports_180` の既定が仕様と逆。相手の threads / delay / retry はレポートに残らない。
- **監査メタデータは不正確**。rules/protocol 版が古く、slot 1 を常に `attax` と書き、TurnCap を `draw` と表示する。これでは公平性を検証できない。

したがって、**「不公平は S2 非対応の相手側の制約だけ」という主張は、現状のコードと文書では支持できない**。コアルールの公平性は高いが、情報条件と監査条件を揃えるか、少なくとも非対称を明示する必要がある。
