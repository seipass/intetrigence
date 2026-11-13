# レフェリー実装メモ(`crates/arena` + `crates/tbp`)

**対象**: S2 レフェリー、TBP host、ベンチハーネス、replay 検証、監査ログ
**プロトコルの正**: [`docs/s2-referee-protocol.md`](s2-referee-protocol.md) v1
**ルールの正**: [`docs/s2-spec.md`](s2-spec.md) v1.3.4
**状態**: M0 ゲート通過(10 ゲーム replay 検証済み)/ M1 レフェリー完成
**最終更新**: 2026-11-14

---

## 0. できること

```text
cargo run --release -p intetrigence-arena --bin arena -- match \
  --seeds 1001,1002,1003,1004,1005 --sheets A,B --turns 1000 \
  --attax C:/Users/Hasilan/Projects/attax --out scripts/evidence/m0

cargo run --release -p intetrigence-arena --bin arena -- verify scripts/evidence/m0
cargo run --release -p intetrigence-arena --bin arena -- report scripts/evidence/m0
```

- `match`: seed × sheet の全組み合わせを実行し、1 ゲーム 1 ファイルの replay(JSONL)、
  監査 JSON、`summary.json` / `summary.md` を書く。各ゲームは書き終えた直後に
  `verify-replay` で再実行検証する。
- `verify`: replay を再実行して、spin / lines / events / sent / cancelled / 盤面ハッシュ /
  結果を記録と比較する。不一致があれば exit 1。
- `report`: 監査 JSON から summary を再生成する(ゲームを再実行しない)。
- `--no-attax`: 外部プロセスなしで baseline 同士を回す(オフラインの配管テスト)。

`match` は既定で 5 PPS(`frame = turn * 12`)、1000 turn cap、preview 5 + hold。

---

## 1. モジュール

| モジュール | 責務 |
|---|---|
| `crates/tbp/src/message.rs` | TBP の wire 型(`info` / `rules` / `ready` / `start` / `suggest` / `suggestion` / `play` / `new_piece` / `stop` / `quit`) |
| `crates/tbp/src/geometry.rs` | TBP / Cold Clear の `(orientation, 中心 x, y)` ↔ `crates/core` の `(rotation, ボックス左下 x, y)`。**セル集合の一致**で変換するので、ピース表が変わってもずれない |
| `crates/tbp/src/host.rs` | プロセス spawn、行区切り JSON、`info` タイムアウト、stderr 保存、think time、`quit`(チェックリスト A/B) |
| `crates/tbp/src/mirror.rs` | attax アダプタの queue/hold 遷移の写し。queue 長 6 の不変条件チェック(D5) |
| `crates/arena/src/movegen.rs` | 入力空間の BFS(左右・ソフトドロップ・SRS+ 回転 + 180°)。到達可能性検証と、core に「テレポート」API がないための**着手の再現パス**を返す |
| `crates/arena/src/engine.rs` | プレイヤー状態、着手適用、採点、相殺、送信、挿入、盤面ハッシュ |
| `crates/arena/src/referee.rs` | ペアドターン、席順、第 1 候補採用、replay 記録、監査集計 |
| `crates/arena/src/replay.rs` | replay レコードと `verify-replay` |
| `crates/arena/src/audit.rs` | think time(median / p95 / max)、first-choice、採点オーディット、Wilson CI |
| `crates/arena/src/baseline.rs` | M0 用の**暫定** in-process bot(本物ではない) |
| `crates/arena/src/tbp_agent.rs` | 外部 TBP bot(attax)の `Agent` 実装。`start` 再送・queue 補充・hold 推論・監査 |
| `crates/arena/src/harness.rs` | seed × sheet のマッチ実行、summary |

---

## 2. プロトコル対応表

| protocol | 実装 |
|---|---|
| §2.1 `frame(t) = t * 12` | `Engine::begin_turn`。core は frame を引数で受け取るだけ(内部時計なし) |
| §2.2 ピース供給 | 両者の `Game` を同じ `piece_seed` で初期化(下記 D1) |
| §2.3 席順(seat 1 → seat 2) | `Sheet::order()`。sheet A = `[0, 1]`、B = `[1, 0]` |
| §3.2 ラウンド進行 | `referee::run_round` |
| §4.1 提出形式 | `Candidate { piece, cells }`。TBP の `location` はセル集合に変換して照合 |
| §4.2 検証 | 合法性・接地・到達可能性・hold 整合・spin 再計算。拒否は forfeit |
| §4.3 第 1 候補採用 | `Engine::adopt`。`moves` を先頭から走査し、自前の到達可能集合に入る最初のもの |
| §5.1〜5.4 解決 | `Engine::apply`(core の `S2Scoring` / `GarbageQueue` / `OpenerPhase` を使用) |
| §6.1 トップアウト | `Game::lock` / `insert_garbage` の後 `Game::is_over()`。seat 1 が死んだらその turn の seat 2 は打たない |
| §6.3 turn cap | 受信ゴミ少 → 送信多 → 引き分け |
| §7 エラー | bot のエラーは forfeit、レフェリーの不変条件違反はマッチ中断(`RoundError::Referee`) |
| §8.1 replay | `replay.rs`。header / turn / garbage / hash / result |
| §8.2 盤面ハッシュ | 各 turn の終わりに両者の盤面(occupied + garbage)の FNV-1a |
| §8.3 verify-replay | `verify_replay`。着手を再適用し、spin / lines / events / sent / cancelled / hash / result を比較 |
| §8.4 監査 | `audit.rs` + `GameAudit` |

---

## 3. 実装上の決定

### D1. ピース列は「同一 seed の 2 本の袋」で、hold は使った本人だけを進める

protocol §2.2 は「共有 7-bag から `bag[t]` を取り、両者に同じピースを配る」と書く。
実装は **両者の `Game` を同じ `piece_seed` で初期化**する。したがって:

- hold を使わない限り、両者の現在ピース・preview は完全に一致する。
- hold は各プレイヤー独立(s2-spec §2.5)。hold を使った本人の袋だけが 1 個進む。
- これは `crates/core` の `Game::new(piece_seed)` / `Game::hold()` の意味と一致し、
  「hold / preview は各プレイヤーが独立に持つ」(protocol §2.2)を満たす唯一の読み方。
  単一の共有袋にすると、片方の hold が相手の将来のピースを変えてしまう。

**replay への影響**: protocol §8.1 の例は turn に 1 つの `piece` を持つが、hold 後は
両者のピースがずれる。そこで **placement ごとに `piece` を記録**し、verify は
placement の `piece` を使う(turn の `piece` は seat 1 の開始時ピースとして残す)。
これはスキーマへの追加であって、既存フィールドの意味は変えない。

> **要確認**: この読みでよいか s2-spec-lead / manager に確認中。protocol を
> 「単一の共有袋 + hold で共有袋を追加消費」に変える場合は、core に
> 「指定ピースを spawn する」API が必要になる(現状の `Game` には無い)。

### D2. 着手の適用は「BFS のパス再生」

`crates/core` の `Game` には任意位置へ置く API がない。レフェリーは
`movegen::generate` が返す**正規のアクション列**を `Game::apply` で再生してから
`Game::lock()` する。これにより:

- キック・180°・SRS+ は core の表をそのまま使う(レフェリー側に複製がない)。
- 到達可能性検証と適用が同じコードなので、検証を通った手は必ず適用できる。
- spin は「最後が回転」の経路が存在すれば最大のラベルを採る
  (`Board::spin_type(kind, rot, x, y, last_rot, fin_or_tst)`)。経路は決定的。

### D3. TBP の hold 推論

`moves[i].location.type != 現在ピース` なら hold(attax アダプタの `applyPlay` と同じ規則)。
レフェリーも同じ規則で hold を適用するので、アダプタの盤面とレフェリーの盤面は一致する。
hold スロットの中身が現在ピースと同じ種類のときは、両者とも「hold なし」と解釈する
(形は同じなので盤面は一致する)。

### D4. `start` 再送と `new_piece` 補充

- ゴミ挿入で盤面が変わったら **`start` を再送**(board / queue / hold / combo / b2b)。
  監査に理由・盤面ハッシュ・turn を記録(E3)。
- それ以外は `play` の後に `new_piece` で queue を 6 個に戻す(D5)。
  hold で空スロットを使った直後は 2 個送る。
- `crates/tbp::Mirror` がアダプタの遷移を写し、毎手「queue == 現在 + preview 5」を検証する。
  不一致は `queue_invariant_violations` に記録(0 でなければバグ)。

### D5. 採点オーディットの「申告値」

TBP の `suggestion` に攻撃量は無い。そこで **申告 spin ラベルから導いた攻撃量**を
「申告値」とし、レフェリーの再計算値と並べて記録する(同じ combo / B2B / multiplier /
lines / PC / ゴミ消しを使う)。attax は guideline 採点なので、All-Mini+ の非 T spin などで
ずれる。ずれは `scoring[slot].examples` に最大 20 件残す。

### D6. forfeit と中断

- `suggest` / `adopt` / `commit` の失敗 → そのゲームは forfeit(相手の勝ち)。
- `adopt` を通ったのに `apply` が失敗 → **レフェリーバグ**としてマッチ中断。
- 起動失敗(attax が spawn できない等)→ 0 turn の forfeit として記録し、replay も書く。

### D7. 同時トップアウト

protocol §6.1 は「seat 1 が死んだら seat 2 は打たない」なので、同一 turn の同時トップアウトは
**到達しない**。引き分けは turn cap の同点のみ。

### D8. ペーシング(P5)

実時間では待たない。`frame = turn * 12` が状態を決め、think time は `suggest` ごとに測る。
時間制限は課さない(protocol §0.5)。実時間の 5 PPS 演出が必要になったら
`run_round` に sleep を足すだけで、状態には影響しない。

### D9. 統計

- median: 偶数個は中央 2 つの平均。
- p95: nearest-rank(`sorted[ceil(0.95 n) - 1]`)。
- Wilson 95% CI: `z = 1.959964`。

### D10. ベースライン bot は M0 の暫定

`crates/bot` はまだ骨組みなので、M0 の intetrigence 側は `BaselineBot`
(自前 movegen + 単純なヒューリスティック)で回した。**本物の bot ではない**。
本物は同じ `Agent` trait に差し込む。M0 の勝敗は配管・監査・replay の機能証明であって
強さの主張ではない。

---

## 4. M0 ゲートの結果

- 実行: `scripts/evidence/m0/`(2026-11-14)
- seeds: **1001, 1002, 1003, 1004, 1005** × sheets **A, B** = 10 ゲーム
- attax: `7b38f51f2ba313bfdb2e7aa7b48d1a6bbcb4ac3b`、bun 1.4.0
- ルール: 5 PPS(12 frame/turn)、1000 turn cap、preview 5 + hold、S2 採点
- 相手: `BaselineBot`(M0 暫定)

| seed | sheet | 勝者 | 理由 | turns | replay | think median (ms) intetrigence / attax | first-choice | spin 一致 |
|---|---|---|---|---|---|---|---|---|
| 1001 | A | attax | TopOut | 51 | OK | 1.0 / 27.1 | 1.00 / 1.00 | 1.00 / 0.96 |
| 1001 | B | attax | TopOut | 51 | OK | 0.9 / 27.1 | 1.00 / 1.00 | 1.00 / 0.96 |
| 1002 | A | attax | TopOut | 58 | OK | 0.8 / 27.8 | 1.00 / 1.00 | 1.00 / 1.00 |
| 1002 | B | attax | TopOut | 58 | OK | 0.8 / 27.3 | 1.00 / 1.00 | 1.00 / 1.00 |
| 1003 | A | attax | TopOut | 48 | OK | 0.7 / 29.4 | 1.00 / 1.00 | 1.00 / 0.98 |
| 1003 | B | attax | TopOut | 48 | OK | 0.8 / 28.5 | 1.00 / 1.00 | 1.00 / 0.98 |
| 1004 | A | attax | TopOut | 104 | OK | 0.7 / 25.9 | 1.00 / 1.00 | 1.00 / 0.97 |
| 1004 | B | attax | TopOut | 94 | OK | 0.9 / 29.3 | 1.00 / 1.00 | 1.00 / 0.99 |
| 1005 | A | attax | TopOut | 131 | OK | 0.5 / 30.8 | 1.00 / 1.00 | 1.00 / 0.98 |
| 1005 | B | attax | TopOut | 92 | OK | 0.6 / 28.2 | 1.00 / 1.00 | 1.00 / 1.00 |

- **replay 検証: 10/10 成功**(`arena verify scripts/evidence/m0`)。
- 監査: 到達可能性拒否 0、queue 不変条件違反 0、hold 推論の不一致 0、
  `cellsAboveRow20` は全ゲーム false、`start` 再送は 1〜10 回/ゲーム(ゴミ挿入)。
- 採点オーディット: attax の spin 不一致は 0〜3 件/ゲーム。すべて
  「attax は `none`、S2 レフェリーは `mini`」(非 T の immobile spin)で、
  1 件は攻撃量が 0 → 1 行に変わった。guideline 採点との差が記録できている。
- **勝敗は参考値**: 0 勝 10 敗。ベースラインが攻撃を作れず(51 手で攻撃 6 行)、
  attax(23 行)にゴミで押し切られた。配管・監査・replay の証明が目的。
- 証拠: `scripts/evidence/m0/{summary.json,summary.md,audit/*.json,replays/*.jsonl,logs/*}`。

### 再現

```bash
# 前提: bun 1.4.0 が PATH、../attax が 7b38f51 で clean
scripts/m0-match.sh
```

---

## 5. 既知の制約・残件

1. **intetrigence 側は暫定ベースライン**。本物の bot(`crates/bot`)ができたら
   `Agent` trait に差し込む。M0 の勝敗を強さの根拠にしない。
2. **attax は guideline 採点 + 20 行視野**。S2 の All-Mini+ / surge / opener を
   知らないので、採点オーディットの不一致は想定内。`cellsAboveRow20` は監査で監視する。
3. **実時間ペーシングなし**(D8)。状態は frame のみで決まる。
4. **stderr はプロセス終了時にファイルへ書く**。長時間走行中の tail はできない。
5. **D1 の共有袋の読み**は s2-spec-lead の確認待ち。
6. `verify-replay` は着手列の再実行であって探索の再現ではない(PLAN §3.3 のとおり)。
7. forfeit は負けとして集計する(protocol §7 の P2 暫定)。
