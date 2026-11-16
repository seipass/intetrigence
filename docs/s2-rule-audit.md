# S2 ルール監査 — 「我々が発明した規則」の全数洗い出し

**目的**: 実装と文書に含まれる規則を **S2 準拠 / 我々のプロトコル / 誤り** に 3 分類し、**勝者を決めるロジックを全部列挙**する。
**契機**: `TurnCap` の「攻撃量で勝者決定」が referee と verify に複製されていた(S2 に存在しない規則)。同じ種類の穴を全数洗い出す。
**監査日**: 2026-11-15 / **対象**: `crates/arena` / `crates/core` / `docs/s2-referee-protocol.md`
**参照**: [`docs/s2-spec.md`](s2-spec.md)(S2 ルールの正)/ [`docs/s2-referee-protocol.md`](s2-referee-protocol.md)(運用)

---

## 0. 結論(要約)

| 分類 | 件数 | 要点 |
|---|---|---|
| **[S2 準拠]** | 24 | core の採点・スピン・ゴミ・opener は一次ソースと一致 |
| **[我々のプロトコル]** | 11 | 明示されているが、**「S2 のルール」と読める書き方が残っている** |
| **[誤り / S2 に存在しない]** | **6** | うち **勝者決定に関わるものが 3 件** |

**最重要**: `End::Stuck` が **`TopOut` として記録**され、`draws` が **未決着と引き分けを混同**し、`wins_forfeit_other` が **cause 未設定を other に数える**。この 3 件は「勝敗の記録」を歪める。

---

## 1. 勝者を決めるロジックの全列挙(最重要)

`crates/arena/src/referee.rs` の `run_round` が `End` を確定し、`(winner, reason, detail, forfeit_cause)` を組み立てる。**勝者を作る経路は 4 つだけ**。

| # | 経路 | 実装 | 分類 | 判定 |
|---|---|---|---|---|
| W1 | `End::TopOut { loser }` | `winner = Some(1 - loser)` | **[S2 準拠]** | TETR.IO の top-out(block-out)で相手の勝ち。正しい |
| W2 | `End::Stuck { loser }` | `winner = None`, `reason = "Stuck"`, `detail = "no legal placement"` | **[誤り（修正済み）]** | **「合法手なし」は S2 の top-out ではない**。S2 の top-out は block-out(スポーン衝突)と lock-out(設置不能)。`Stuck` は**ボットが置けない**状態であり、ルール上の決着ではない。修正済み |
| W3 | `End::TurnCap` | `winner = None`, `detail = "unresolved: turn cap reached"` | **[我々のプロトコル]** | 修正済み(以前は攻撃量で勝者を決めていた)。S2 に時間切れの規則は無い |
| W4 | `End::Forfeit { slot }` | `winner = None`, `reason = "Forfeit"` | **[我々のプロトコル]** | **決定: `winner = None`(no contest)**。protocol §7.1.3.1。修正済み |

### 1.1 集計側の勝者判定

`crates/arena/src/harness.rs` の `summarize`:

| # | 集計 | 実装 | 分類 | 判定 |
|---|---|---|---|---|
| A1 | `wins` | `winner == Some(0)` | **[我々のプロトコル]** | **forfeit 勝ちを含む**。「裁定上の勝ち」であることを明示すべき |
| A2 | `losses` | `winner == Some(1)` | 同上 | forfeit 負けを含む |
| A3 | `draws` | `winner.is_none()` | **[誤り]** | **未決着(TurnCap)と引き分けが混ざる**。S2 に「引き分け」は無い(両者同時 top-out は起こらない) |
| A4 | `wins_topout` | `winner == 0 && reason == "TopOut"` | **[S2 準拠]** | 正しい |
| A5 | `wins_forfeit_vision` | `cause == Some("vision")` | **[S2 準拠]** | 正しい |
| A6 | `wins_forfeit_other` | `cause != Some("vision")` | **[誤り]** | **`cause == None` も other に数える**。`Some("other")` のみにすべき |
| A7 | `wins_turncap` | 集計フィールドとして残るが常に 0 | **[誤り（修正済み）]** | ゲートを `wins_topout == 50` に変更して解消。`wins_turncap` は追加しない |
| A8 | `losses_topout` | `winner == 1 && reason == "TopOut"` | **[S2 準拠]** | 正しい |
| A9 | `losses_forfeit` | `winner == 1 && reason == "Forfeit"` | [我々のプロトコル] | 正しい(forfeit 負け) |
| A10 | `turncap` | `reason == "TurnCap"` | [我々のプロトコル] | 正しい |

### 1.2 verify 側の複製

`verify_replay` は `ResultRecord` の `winner` / `reason` を**再計算して照合**する。TurnCap の「攻撃量で勝者決定」が referee と verify に複製されていたのはこの構造のため。**勝者決定ロジックは 1 箇所に集約すべき**(現在は `referee.rs` の `match &end` のみが正で、verify は記録と照合する)。

---

## 2. ゲーム終了条件

| # | 条件 | 実装 | 分類 | 判定 |
|---|---|---|---|---|
| E1 | block-out(スポーン衝突) | `Game::spawn` が失敗 → `topped_out` | **[S2 準拠]** | S2 の top-out |
| E2 | lock-out | **実装されていない**(`nolockout: true`) | **[S2 準拠]** | S2 TL デフォルトは lock-out 無効 |
| E3 | Clutch Clear | `spawn(kind, clutch = 直前が消去)` で押し上げ探索 | **[S2 準拠]** | Beta 1.5.0 |
| E4 | 合法手なし(`Stuck`) | `AdoptError::NoMoves` → `End::Stuck` | **[誤り]** | S2 に無い終了条件。**到達不能**(生きている active は定義上 spawn から到達可能)なので実害は無いが、**`TopOut` というラベルは誤り** |
| E5 | ターン上限 1000 | `config.turn_cap` | **[我々のプロトコル]** | ハーネスの安全弁。S2 に無い |
| E6 | ボットの 0 候補 | `AdoptError::NoLegalCandidate` → `End::Forfeit` | **[我々のプロトコル]** | S2 に無い(相手の実装制約) |
| E7 | プロセス異常終了・タイムアウト | `End::Forfeit` | **[我々のプロトコル]** | S2 に無い |

---

## 3. レフェリーの定数・挙動

| # | 項目 | 値 / 挙動 | 分類 | 出典・根拠 |
|---|---|---|---|---|
| R1 | `frames_per_turn` | 12(= 5 PPS) | **[我々のプロトコル]** | PLAN §1。S2 は 60fps の実時間 |
| R2 | `turn_cap` | 1000 | **[我々のプロトコル]** | 安全弁 |
| R3 | 席順(seat 1 → seat 2) | 逐次解決 | **[我々のプロトコル]** | S2 は同時進行。シート入替で相殺 |
| R4 | `visible_rows` | 相手のソースから自動検出 | **[我々のプロトコル]** | S2 に概念が無い(全員 40 行) |
| R5 | `forfeit_cause` | vision / other / unknown | **[我々のプロトコル]** | S2 に無い |
| R6 | スピンラベル = 経路の最大 | `max over paths` | **[我々のプロトコル]** | S2 は実入力で決まる。我々はセル集合しか受け取らない |
| R7 | ゴミ挿入は「ライン消去なしのロック」のみ | `lines == 0` | **[S2 準拠]** | combo blocking |
| R8 | 相殺はイベント順、1 行ずつ | `cancel_with_opener` をイベント毎 | **[S2 準拠]** | s2-spec §7.3 |
| R9 | 送信はイベント毎に別パケット、`arrival_frame = frame` | `receive(sent, frame)` | **[S2 準拠]** | s2-spec §7.2 |
| R10 | 挿入は `arrival + 20 <= frame`、最大 8 行 | `take_insertable` | **[S2 準拠]** | s2-spec §7.5 |
| R11 | Profile(official / defense-stress / garbage-pressure / composite) | 開始盤面の違いのみ | **[我々のプロトコル]** | S2 に無い。診断用 |
| R12 | ペーシングしない(実時間で待たない) | sleep 無し | **[我々のプロトコル]** | S2 は実時間 |

---

## 4. `crates/core` のルール定数

> **`crates/core` の詳細監査は [`docs/core-rule-audit.md`](core-rule-audit.md)(core-engine 担当)が正**。本節は横断確認のための要約で、重複を避けるため詳細はそちらを参照する。core-engine の監査で **F1(0 ライン設置の combo ln 攻撃)** が見つかり、修正済み。仕様側も §6.3 / §6.8 を修正した。

すべて [`docs/s2-spec.md`](s2-spec.md) と一致することを確認した。

| # | 定数 | 値 | 分類 | 出典 |
|---|---|---|---|---|
| C1 | `BASE_ATTACK` | none `[0,0,1,2,4]` / mini `[0,0,1,2,10]` / full `[0,2,4,6,10]` | **[S2 準拠]** | s2-spec §6.1(S7 `garbageData`) |
| C2 | `B2B_ATTACK_BONUS` | 1 | **[S2 準拠]** | s2-spec §6.2 |
| C3 | `B2B_CHARGE_AT` | 4 | **[S2 準拠]** | s2-spec §6.7.1(S6 `b2bcharge_at`) |
| C4 | `B2B_CHARGE_BASE` | 3 | **[S2 準拠]** | s2-spec §6.7.1(S6 `b2bcharge_base`) |
| C5 | `PC_GARBAGE` | 5 | **[S2 準拠]** | s2-spec §6.5(Patch Beta 1.2.0) |
| C6 | `PC_B2B` | 1 | **[S2 準拠]** | s2-spec §6.5(S6 `allclear_b2b: 1`) |
| C7 | `GARBAGE_CAP` | 8 | **[S2 準拠]** | s2-spec §7.1(S6 `garbagecap`) |
| C8 | `GARBAGE_SPEED_FRAMES` | 20 | **[S2 準拠]** | s2-spec §7.1(S6 `garbagespeed`) |
| C9 | `GARBAGE_HOLE_SIZE` | 1 | **[S2 準拠]** | s2-spec §7.1 |
| C10 | `MESSINESS_CHANGE` | 1.0 | **[S2 準拠]** | s2-spec §7.1 |
| C11 | `MESSINESS_WITHIN` | 0.0 | **[S2 準拠]** | s2-spec §7.1 |
| C12 | `GARBAGE_MULTIPLIER_INIT` | 1.0 | **[S2 準拠]** | s2-spec §7.1 |
| C13 | `GARBAGE_MARGIN_FRAMES` | 10800 | **[S2 準拠]** | s2-spec §7.1(S6 `garbagemargin`) |
| C14 | `GARBAGE_INCREASE_PER_FRAME` | 0.008/60 | **[S2 準拠]** | s2-spec §7.8(逐次加算) |
| C15 | `OPENER_PIECES` | 14 | **[S2 準拠]** | s2-spec §7.4 |
| C16 | `ALLCLEAR_CHARGES_SURGE` | true | **[未確定(§11-U3)]** | S6 `allclear_charges: false` と矛盾。暫定で S7 に合わせた |
| C17 | `OPENER_QUEUE_GE_SENT` | true | **[未確定(§11-U5)]** | S1 の散文は `>`。暫定で実装(`>=`)に合わせた |
| C18 | `GARBAGE_DISCARD_TOP_ROWS` | true | **[S2 準拠]** | s2-spec §11-U6(到達不能を証明) |
| C19 | スピン判定(非 T = immobile、T = 3-corner/2-corner + 5th kick) | §4.2 / §4.3 | **[S2 準拠]** | s2-spec §4(S2 / S7) |
| C20 | 7-bag | Fisher-Yates + MINSTD | **[S2 準拠]** | s2-spec §2.4 |
| C21 | hold | 1 ピース 1 回、ロックで回復 | **[S2 準拠]** | s2-spec §2.5 |
| C22 | 盤面 40 行 / 可視 20 行 | `HEIGHT = 40` | **[S2 準拠]** | s2-spec §2.1 |
| C23 | spawn 最下段セル row 20〜21 | `spawn_position` | **[S2 準拠]** | s2-spec §2.2.1 |
| C24 | lock-out 無し | 判定が存在しない | **[S2 準拠]** | s2-spec §2.3 |

---

## 5. 文書の混同(`docs/s2-referee-protocol.md`)

| # | 箇所 | 問題 | 分類 |
|---|---|---|---|
| D1 | §1「マッチ構成」の表 | 「合格条件」など**プロトコル**が S2 の表と同じ体裁で並ぶ | **[我々のプロトコル]** — 見出しに「プロトコル」と明記すべき |
| D2 | §6「終了条件と勝敗」 | TurnCap を「勝敗」の節に置いている | **[我々のプロトコル]** — S2 の勝敗は top-out のみ |
| D3 | §7.1「forfeit の扱い」 | 「裁定としては相手の負け」と書くが、`winner` に記録されることを明示していない | **[我々のプロトコル]** — 追記すべき |
| D4 | §8.5 レポート要件 | `draws` を「引き分け」と書いているが、実装では未決着も含む | **[誤り]** — 「未決着(ターン上限)」と「引き分け」を分ける |
| D5 | 全体 | 「S2 準拠」と「我々のプロトコル」の**境界が節ごとに書かれていない** | **[我々のプロトコル]** — 各節の冒頭に分類を書くべき |

---

## 6. 誤りの一覧と修正案

| # | 誤り | 影響 | 修正案 | 優先度 |
|---|---|---|---|---|
| **F1** | `End::Stuck` を `reason = "TopOut"` で記録 | 「合法手なし」が top-out として集計され、`wins_topout` に混入し得る | `reason = "Stuck"` を新設し、**ゲートでは未決着扱い**。到達不能なので実害は無いが、ラベルを正す | 中（修正済み） |
| **F2** | `draws = winner.is_none()` | 未決着(TurnCap)と引き分けが混ざる | `draws` を「両者 top-out 同時」(S2 では到達不能)に限定し、**`unresolved` を別カウント** | **高（修正済み）** |
| **F3** | `wins_forfeit_other = cause != Some("vision")` | `cause == None` が other に数えられる | `cause == Some("other")` のみに。`None` は `unknown` として別カウント | **高（修正済み）** |
| **F4** | `wins_turncap` が存在しない | ゲート(`wins_topout + wins_turncap >= 45`)が計算できない | ゲートを `wins_topout == 50` に変更して解消（`wins_turncap` は追加しない） | **高（修正済み）** |
| **F5** | `End::Forfeit` が `winner = Some(1 - slot)` | 相手の 0 候補が「勝ち」として記録される | **決定: B(`winner = None`)**。protocol §7.1.3.1 に明文化。実装は修正済み | **高（修正済み）** |
| **F6** | 文書 §8.5 の `draws` の記述 | 実装と食い違う | 文書を実装に合わせる(または実装を文書に合わせる) | 中 |
| **F7** | **仕様 §6.3 / §6.8 の combo 乗数**(spec 側の誤り) | 0 ライン設置で直前の combo が残り、`ln` 項が攻撃を作る | **修正済み**: `lines > 0` ガードを追加。fixture に `no-clear-combo5-no-attack` を追加 | **高(修正済)** |
| **F8** | `crates/core` の F1(0 ライン combo) | 同上の実装側 | **修正済み**(core-engine) | **高(修正済)** |

---

## 7. 決定済みの項目(2026-11-15 確定)

### 7.1 forfeit の `winner` — **決定: B(`winner = None`、no contest)**

| 案 | 内容 | 長所 | 短所 |
|---|---|---|---|
| A(却下) | `winner = Some(相手)`、`reason = "Forfeit"` | TETRA LEAGUE の「切断は負け」に近い。ゲームに結果がある | 「相手のバグで勝ち」が記録に残る |
| **B(採用)** | `winner = None`、`reason = "Forfeit"` | 「勝負が成立していない」ことが記録から明らか | TETRA LEAGUE の切断規則と異なる |

**決定: B**(manager 承認、2026-11-15)。理由: 我々の forfeit は**相手の実装制約**であり、TETRA LEAGUE の切断(プレイヤーの意思・回線)とは性質が違う。ゲートが既に forfeit を勝ちに数えていないので、`winner` だけ A のままにしておくと**記録と主張が食い違う**。

`forfeit-vision`(20 行視野で置けない)は**相手の制約**、`forfeit-other`(アダプタ故障)は**相手のバグ**。前者は「不戦」、後者は「無効ゲーム」で、**両方 `winner = None`**。

**protocol への反映**: §7.1.3.1(`winner` の記録規則)と §7.1.3(勝ちに数えない)に明文化済み。**TETRA LEAGUE との差はレポートで開示する**(§8.5)。

### 7.2 `Stuck` の扱い — **決定: `reason = "Stuck"`、未決着扱い**

到達不能(証明済み)なので、**`reason = "Stuck"` にして未決着扱い**にするのが正しい。`TopOut` と呼ぶのは S2 の用語の誤用。protocol §6.4 の `reason` enum に追加済み。

### 7.3 ゲートの数値 — **決定: goal に合わせて厳格化**(protocol v1.27)

| 段階 | ゲート |
|---|---|
| 1(attax) | `wins_topout == 50` かつ `losses_topout == 0` かつ `turncap == 0` かつ `forfeit == 0` |
| 2・3 | `wins_topout >= 48` かつ `turncap == 0` |

**`TurnCap` は勝ちに数えない**(決定 A)。ゲートには `wins_turncap` を使わない（集計フィールドは互換のため残るが常に 0）。

### 7.4 `turn_cap` / `frames_per_turn` の文書分離 — **未着手**

protocol §1 の表を **「S2 のルール」と「計測プロトコル」の 2 表に分ける**ことを推奨。現在は 1 つの表に混在している。

---

## 8. 監査の方法と限界

- **コードは `git show HEAD:<path>` で読んだ**(作業ツリーは他担当のパッチが当たっている可能性があるため)。ただし本監査は main の作業ツリーで実施。
- **S2 準拠の判定は `docs/s2-spec.md` の出典(S1〜S11)に依拠**。一次ソースを再取得していない項目は、spec の確度表記を継承する。
- **未確定(§11-U3 / U5)は「S2 準拠」と断定できない**。暫定決定であり、差し替え条件が spec にある。
- **`crates/bot` は監査対象外**(ルールではなく探索・評価)。

---

## 9. 改訂履歴

| 版 | 日付 | 変更 |
|---|---|---|
| v1.1 | 2026-11-15 | `docs/core-rule-audit.md` への参照を追加。**F7 / F8(0 ライン設置の combo ln 攻撃)** を追加(仕様・実装とも修正済み)。fixture を 19 件に。 |
| v1.2 | 2026-11-15 | **決定を反映**。F5 = B(`winner = None`、no contest)、`Stuck` = `reason = "Stuck"`、`TurnCap` = 勝ちに数えない(A)、ゲート = `wins_topout == 50`(段階 1)/ `>= 48`(段階 2・3)。§7 を「判断が必要な点」から「決定済みの項目」に変更。**実装は v1.27 時点で追随済み**。 |
| v1.3 | 2026-11-16 | 実装追随を反映(Stuck / Forfeit / TurnCap / ゲート)。F11(最大スピン説明)と F12(旧 TurnCap 記述)の文書修正を反映。 |
| v1 | 2026-11-15 | 初版。勝者決定 4 経路 + 集計 10 項目 + 終了条件 7 項目 + レフェリー 12 項目 + core 24 項目 + 文書 5 項目を 3 分類。誤り 6 件(うち勝者決定に関わるもの 3 件)を特定。forfeit の `winner` について A/B の判断を依頼。 |
