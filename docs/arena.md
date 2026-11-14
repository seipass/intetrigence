# レフェリー実装メモ(`crates/arena` + `crates/tbp`)

**対象**: S2 レフェリー、TBP host、ベンチハーネス、replay 検証、監査ログ
**プロトコルの正**: [`docs/s2-referee-protocol.md`](s2-referee-protocol.md) v1.9
**ルールの正**: [`docs/s2-spec.md`](s2-spec.md) v1.3.11
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
| `crates/arena/src/movegen.rs` | `intetrigence_bot::movegen` のアダプタ。BFS は bot 側の 1 実装を共有し、セル集合キーの `MoveSet` と着手の再現パスを返す |
| `crates/arena/src/engine.rs` | プレイヤー状態、着手適用、採点、相殺、送信、挿入、盤面ハッシュ |
| `crates/arena/src/referee.rs` | ペアドターン、席順、第 1 候補採用、replay 記録、監査集計 |
| `crates/arena/src/replay.rs` | replay レコードと `verify-replay` |
| `crates/arena/src/audit.rs` | think time(median / p95 / max)、first-choice、採点オーディット、Wilson CI |
| `crates/arena/src/baseline.rs` | M0 用の**暫定** in-process bot(本物ではない) |
| `crates/arena/src/bot_agent.rs` | 本物の `intetrigence_bot::Searcher` を `Agent` に接続(`--ms 100`、8 スレッド) |
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
| §8.1 replay | `replay.rs`。header / turn / garbage / hash / result。`placement.piece` 必須(v1.1 以降) |
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

**replay への影響**: per-player bag では turn 単位のピースが定義できないので、
**placement ごとに `piece` を記録**し、verify は placement の `piece` を使う。
protocol v1.1(2026-11-14、s2-spec-lead)で §2.2 が per-player bag に改訂され、
§8.1 の `turn.piece` は削除・`placement.piece` が必須になった。v1.2 では §6.1 の
「同一 turn の同時トップアウト → 引き分け」が到達不能と確認され、「トップアウトした瞬間に
終了、相手の勝ち」に統一された(実装は元からその挙動)。v1.3 では §2.1 のペーシングが
「実時間で待たない・200 ms 超過を数える」に確定した(下記 D8)。s2-spec §11-U7 の
「ピース袋は 1 本」も「`piece_seed` は 1 本、袋のインスタンスはプレイヤーごと」に
修正された(v1.3.5、ルール値の変更なし。v1.3.6 は §12 の監査追記のみ)。

### D1.1. `View` のフィールドの意味(bot 側と合わせる)

| フィールド | 意味 |
|---|---|
| `combo` | **この設置で式に入る値**(設置前の combo。0 = チェインなし) |
| `b2b_streak` | 連続 difficult clear 数 `N`(**0 始まり**。表示 B2B x4 = `N = 5`) |
| `pending_garbage` | 自分のキュー全体(**到着前のパケットを含む**) |
| `preview` | 5 枚(現在ピースは含まない) |
| `active` | 現在ピースの完全な位置(ボックス左下) |
| `multiplier` | その turn の frame での攻撃倍率(逐次加算値) |

### D2. 着手の適用は「BFS のパス再生」

`crates/core` の `Game` には任意位置へ置く API がない。レフェリーは
`movegen::generate` が返す**正規のアクション列**を `Game::apply` で再生してから
`Game::lock()` する。これにより:

- キック・180°・SRS+ は core の表をそのまま使う(レフェリー側に複製がない)。
- 到達可能性検証と適用が同じコードなので、検証を通った手は必ず適用できる。
- spin は「最後が回転」の経路が存在すれば最大のラベルを採る
  (`Board::spin_type(kind, rot, x, y, last_rot, fin_or_tst)`)。経路は決定的。
- 合法手集合は**セル集合で 1 個**に畳む。I/O/S/Z は同一セルを複数 rotation で表し、
  回転由来かどうか(`spin_entry`)が異なることがあるため、**最強の spin ラベルを残す**
  (最初の BFS 順で残すと Mini を落とす。bot-dev の 27edb70 と同じ規則)。
- **BFS は bot と共有**(`intetrigence_bot::movegen`)。`crates/arena/src/movegen.rs` は
  アダプタだけで、`(sorted cells, spin)` の集合一致を
  `crates/arena/tests/movegen_crosscheck.rs` で恒久的に検証する(fixture 5 種 + ランダム 32 盤面)。
  protocol v1.4 §4.2.1 で `spin(cells) = max over paths p of spin_type(cells, last_action(p), kick(p))`
  として明文化された。
- **TETR.IO との差(重要)**: この「経路の最大 spin」規則は、実機の「実際に回して置いたか」より
  **寛容**。回転せずに置いた配置でも、回転経由の経路が存在すればスピンとして採点され得る。
  両者に同じ規則を適用するので**強さの比較は公平**だが、**TETR.IO のスコアとは一致しない**。
  最終レポートにはこの解釈差を明記する(protocol v1.7 §4.2.1)。
- **どちらに不利か**: intetrigence(bot-dev)側は `placement.spin != None ⇒ spin_entry` を
  テストで固定しており(`assert_spin_flags`)、提出する spin ラベルは常に回転終端の入力列に
  裏付けられている。したがって**寛容さを利用するのはセルだけを提出する相手(attax)側**で、
  我々には保守的な方向に働く。この非対称も最終レポートに書く。

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

### D8. ペーシング(P5、protocol v1.3 で確定)

実時間では待たない。`frame = turn * 12` が状態を決め、think time は `suggest` ごとに測る。
時間制限は課さない(protocol §0.5)。代わりに **1 手 200 ms(5 PPS の持ち時間)を超えた手**を
`ThinkStats.over_budget` に数え、レポートに出す。0 件なら「公称 5 PPS と実測が一致」と
主張でき、超過があれば正直に書ける。実時間の 5 PPS 演出が必要になったら `run_round` に
sleep を足すだけで、状態には影響しない。

### D8.1. 計測の定義(レポートで使う)

| 項目 | 定義 |
|---|---|
| `think_ms` | レフェリーが `suggest` を送信してから `suggestion` を受信するまでの**実時間**(ホスト側)。ボットの自己申告ではない |
| `extra_ms`(attax) | `move_info.extra` の自己申告値。参考として別に記録し、`think_ms` とは混ぜない |
| `board_hash` | 各 turn 終了時の盤面。**occupied 40 行 + garbage 40 行**の両方を FNV-1a でハッシュ(穴列の差も検出) |
| `suggestion` が空 | 2 段判定(protocol v1.6 §7): レフェリーの movegen に合法手がある → **forfeit**(ボットの movegen が壊れている)/ 無い → **トップアウト**(盤面が詰んでいる)。`AdoptError::NoLegalCandidate` / `NoMoves` で区別 |
| リトライ | 1 設置につき `suggest` は **1 回だけ**。同じ turn で同じボットに再質問しない |
| `start` 再送 | 自分のロックでゴミが実際に挿入されたときだけ。combo blocking / due なし / forfeit 時は送らない |

### D9. 統計

- median: 偶数個は中央 2 つの平均。
- p95: nearest-rank(`sorted[ceil(0.95 n) - 1]`)。
- Wilson 95% CI: `z = 1.959964`。

### D9.1. 本物の bot の接続(M2)

`crates/arena/src/bot_agent.rs` が `intetrigence_bot::Searcher` を `Agent` に接続する。

- `View` → `RootState`: `current = active.kind`、`preview = 5 preview`、`hold`、
  `EvalContext { b2b_streak, combo, pieces_placed, incoming = pending_garbage, t_slot = 0 }`。
- `--ms 100`(既定)、スレッド 8(`MAX_THREADS`)。
- 検索は**標準 spawn 前提**。ゴミ挿入で active が押し上げられた場合や Clutch では、
  検索が `None` を返すことがある。その場合は**実際の active からの movegen** で
  合法手を生成し、簡易評価で並べて候補の末尾に追加する(フォールバック)。
  フォールバック回数は監査の `bot_fallbacks` に記録する。
- 検索候補は `SearchResult.candidates`(best 先頭)をそのまま優先順にする。レフェリーは
  到達可能な最初の候補を採用するので、検索の best が押し上げ後の active から到達不能でも
  次の候補・フォールバックで合法手が残る。

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
| 1001 | A | attax | TopOut | 51 | OK | 0.9 / 29.0 | 1.00 / 1.00 | 1.00 / 0.96 |
| 1001 | B | attax | TopOut | 51 | OK | 0.9 / 28.9 | 1.00 / 1.00 | 1.00 / 0.96 |
| 1002 | A | attax | TopOut | 58 | OK | 0.8 / 29.5 | 1.00 / 1.00 | 1.00 / 1.00 |
| 1002 | B | attax | TopOut | 58 | OK | 0.8 / 27.7 | 1.00 / 1.00 | 1.00 / 1.00 |
| 1003 | A | attax | TopOut | 48 | OK | 0.7 / 30.4 | 1.00 / 1.00 | 1.00 / 0.98 |
| 1003 | B | attax | TopOut | 48 | OK | 0.8 / 28.8 | 1.00 / 1.00 | 1.00 / 0.98 |
| 1004 | A | attax | TopOut | 104 | OK | 0.7 / 27.8 | 1.00 / 1.00 | 1.00 / 0.97 |
| 1004 | B | attax | TopOut | 94 | OK | 0.9 / 28.5 | 1.00 / 1.00 | 1.00 / 0.99 |
| 1005 | A | attax | TopOut | 131 | OK | 0.5 / 30.3 | 1.00 / 1.00 | 1.00 / 0.98 |
| 1005 | B | attax | TopOut | 92 | OK | 0.6 / 28.8 | 1.00 / 1.00 | 1.00 / 1.00 |

- **replay 検証: 10/10 成功**(`arena verify scripts/evidence/m0`)。
- 監査: 到達可能性拒否 0、queue 不変条件違反 0、hold 推論の不一致 0、
  `cellsAboveRow20` は全ゲーム false、`start` 再送は 1〜10 回/ゲーム(ゴミ挿入)。
- 採点オーディット: attax の spin 不一致は 0〜3 件/ゲーム。すべて
  「attax は `none`、S2 レフェリーは `mini`」(非 T の immobile spin)で、
  1 件は攻撃量が 0 → 1 行に変わった。guideline 採点との差が記録できている。
  `ScoringAudit` は **spin 不一致**と**spin が一致したうえでの攻撃量不一致**を別カウンタで持つ
  (`attack_mismatches_same_spin`)。最終レポートではボット別にこの 2 つを分けて出す。
  **解釈**(s2-spec-lead と確認済み): 本 10 ゲームでは、レフェリーと attax の採点差は
  **spin 分類に限られ**、攻撃式の差は観測されなかった。12 件中 2 件だけ攻撃量が変わるのは、
  mini single の base 0 と none single の base 0 が同じで、**B2B が乗っている局面だけ**
  S2 側に +1 が乗るため。S2 版 attax では spin 不一致も 0 になる**見込み**(未実測)。
- **200 ms 超過**: 全 10 ゲームで 0 件(ベースライン < 2 ms、attax ~27 ms)。
  `ThinkStats` は median / p95 / **p99** / max を持つ(1 手だけ遅いケースを検出するため)。
- **ボット別集計**(10 ゲーム合計):
  - intetrigence(baseline): 735 配置、spin 不一致 **0**、攻撃不一致 **0**、200 ms 超過 **0**
  - attax: 730 配置、spin 不一致 **12**、攻撃不一致 **2**(うち **spin が一致したうえでの攻撃不一致 0**)、200 ms 超過 **0**
  - attax の 12 件はすべて「attax `none` → S2 `mini`」の All-Mini+ 非 T spin。攻撃の 2 件も
    その spin 差に由来する。同じ spin での攻撃量は一致している。
- **勝敗は参考値**: 0 勝 10 敗。ベースラインが攻撃を作れず(51 手で攻撃 6 行)、
  attax(23 行)にゴミで押し切られた。配管・監査・replay の証明が目的。
- 証拠: `scripts/evidence/m0/{summary.json,summary.md,audit/*.json,replays/*.jsonl,logs/*}`。
- header の版: この証拠は `docs/s2-spec.md@v1.3.7` / `docs/s2-referee-protocol.md@v1.4`。
  `arena match` は既定で `+<short HEAD>` を付ける(`git show <commit>:<path>` で当時の文書を復元)。
  この証拠は統合前の worktree で生成したため、dangling hash を避けて版のみを記録している。
  統合コミット後に `scripts/m0-match.sh` を再実行すると `+<commit>` 付きになる。

### 再現

```bash
# 前提: bun 1.4.0 が PATH、../attax が 7b38f51 で clean
scripts/m0-match.sh
```

---

## 4.1 M2 チューニングゲート(2026-11-14)

本物の `intetrigence_bot::Searcher`(`--ms 100`、8 スレッド)を接続し、tuning seeds
**9001–9010 × A/B = 20 ゲーム**を attax `7b38f51` と実施した。詳細は
[`docs/m2-tuning-report.md`](m2-tuning-report.md)、証拠は `scripts/evidence/m2-tuning/`。

- **18 勝 2 敗 0 分**(勝率 90.0%、Wilson 95% CI 69.9%–97.2%)、replay 検証 **20/20**
- 勝ちの内訳(protocol v1.8 §7.1): **TopOut 0 / forfeit-vision 18 / forfeit-other 0**。
  負けは **TopOut 2**(9006-A downstack、9010-A buried。いずれも sheet A)。
  intetrigence の forfeit は **0**、`other` の forfeit も **0**(無効ゲームなし)
- 18 勝はすべて attax の **vision forfeit**: ゴミで attax の可視 20 行が埋まり、
  `moves` が空になった。40 行のトップアウトまで到達した 2 ゲームは **0 勝 2 敗**
- think median / p95: intetrigence **97.1 / 98.9 ms**、attax **37.0 / 46.7 ms**、200 ms 超 0
- attack/piece: intetrigence **1.000**、attax **0.876**。受信ゴミ/ゲーム: **72.0 / 105.0**
- 採点オーディット: intetrigence spin 0 / attack 0、attax spin 78(All-Mini+ 非 T)/ same-spin attack 0
- **解釈**: 生の勝率は M2 ゲート(5 割)を超えるが、その大半は attax の 20 行視野による forfeit。
  フル 40 行視野の相手に対する強さはまだ測れていない。M3 は防御・downstack と
  (可能なら)フル視野の相手での評価が課題。

## 5. 既知の制約・残件

1. **intetrigence 側は暫定ベースライン**。本物の bot(`crates/bot`)ができたら
   `Agent` trait に差し込む。M0 の勝敗を強さの根拠にしない。
2. **attax は guideline 採点 + 20 行視野**。S2 の All-Mini+ / surge / opener を
   知らないので、採点オーディットの不一致は想定内。`cellsAboveRow20` は監査で監視する。
3. **実時間ペーシングなし**(D8)。状態は frame のみで決まる。
4. **stderr はプロセス終了時にファイルへ書く**。長時間走行中の tail はできない。
5. **movegen の一本化**(bot-dev 提案): `crates/bot::Movegen` に `Placement.actions` /
   `generate_from` / `reachable_states` / `unique_by_cells` があり、`crates/arena::movegen`
   を差し替え可能。**I/O/S/Z は同一セルを複数 rotation で表す**ので、クロスチェックは
   両者を **`(cells, spin)` の集合**に畳んでから比較する(`(rotation, x, y)` の集合で比べると
   個数が食い違う)。spin を含めるのは、セルが同じでも回転由来かどうかで採点が変わるため。
   差し替え後もクロスチェックは残す(FRONT 表バグと同じ二重実装リスクの検出)。
6. `verify-replay` は着手列の再実行であって探索の再現ではない(PLAN §3.3 のとおり)。
7. forfeit は負けとして集計する(protocol §7 の P2 暫定)。
8. **スピン判定は TETR.IO より寛容**(protocol v1.7 §4.2.1)。セル集合に到達する経路のうち
   最大の spin ラベルを採るため、回転せずに置いた配置もスピンになり得る。両者に同じ規則なので
   強さの比較は公平だが、TETR.IO のリプレイ・スコアとは一致しない。intetrigence 側は spin を
   常に回転終端の入力列で裏付ける(bot-dev の `assert_spin_flags`)ので、寛容さの恩恵を受けるのは
   セルだけを提出する attax 側であり、我々には保守的。
