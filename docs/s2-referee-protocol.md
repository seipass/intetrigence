# S2 マッチ・レフェリープロトコル(凍結版)

**対象**: `crates/arena`(レフェリー)、`crates/tbp`(接続)、attax アダプタ、replay 検証ツール
**ルールの正**: [`docs/s2-spec.md`](s2-spec.md)(S2 ルール仕様)。本書は**ルールをどう運用するか**だけを定める。
**接続の実装詳細**: [`docs/attax-integration.md`](attax-integration.md)(TBP アダプタの挙動)
**凍結日**: 2026-11-13 / **版**: v1

本書は **TETR.IO の再現ではなく、我々のマッチのプロトコル**である。ルール値は s2-spec.md が唯一の正で、本書はそれを「ターン」「frame」「提出」「記録」に落とす。

---

## 0. 原則

1. **決定論**: `(seed, 席, 両者の着手列)` から全ラウンドが完全再現できること。verify-replay の前提。
2. **対称性**: 席による有利不利は、シートの入れ替えで相殺できる形にする(§2.3)。
3. **ルールは s2-spec.md のみ**: 本書にルール値(攻撃表・cap・遅延など)を書かない。参照で済ませる。
4. **監査可能性**: 思考時間・第 1 候補採用率・採点一致を毎ゲーム記録する(PLAN §3.3)。
5. **飢死させない**: ボットの応答を待つ。時間制限は課さない(PLAN §3.3)。

---

## 1. マッチ構成

| 項目 | 値 | 出典 |
|---|---|---|
| ゲーム数 | **100**(seed 50 × シート 2) | PLAN §3.1 |
| 合格条件 | **90 勝以上**(点推定 ≥ 90%)+ Wilson 95% CI 併記 | PLAN §3.1 |
| ペース | 5 PPS = **12 frame / turn** | PLAN §3.1、s2-spec §11-U9 |
| ターン上限 | **1000 turn** | PLAN §3.1 |
| preview | **5** + hold | s2-spec §2.6 |
| 思考時間 | intetrigence `--ms 100`、attax は S2 版デフォルト | PLAN §3.1 |
| seed | 両プロジェクトの `docs/benchmark` 未使用の held-out 50 個 | PLAN §3.1 |

### 1.1 seed の扱い

- 1 seed = 1 ゲーム(シートごとに 1 回)。**同じ seed を 2 回プレイする**(シート A / B)。
- seed は `1 ≤ s ≤ 2_147_483_646` の整数。
- seed から RNG への導出は **s2-spec.md §11-U7** のとおり。レフェリーは `derive_seeds(s)` を 1 回だけ呼び、`piece_seed` と `garbage_seeds[slot]` を得る。
- **ラウンド開始時に毎回、導出から初期化し直す**。ラウンド間で RNG を引き継がない。

### 1.2 席(sheet)と slot

| 概念 | 意味 | シートで入れ替わるか |
|---|---|---|
| **席(seat)** | 1 turn 内の解決順序。seat 1 → seat 2 | **入れ替わる** |
| **slot** | ゴミ穴 RNG の割り当て。`slot 0 = intetrigence`、`slot 1 = attax` | **入れ替わらない** |

| シート | seat 1 | seat 2 |
|---|---|---|
| **A** | intetrigence | attax |
| **B** | attax | intetrigence |

- **ピース列は両シートで同一**(`piece_seed` は 1 本、両者に同じ列を配る)。
- **ゴミ穴列は bot ごとに固定**(slot が identity なので、シートを変えても同じ bot は同じ穴列)。
- したがって 2 シートの違いは **1 turn 内の解決順序だけ**になる(§3.2)。これが席バイアスを相殺する。

---

## 2. ターンと frame

### 2.1 定義

```
turn t = 0, 1, 2, ... 999
frame(t) = t * 12                # 5 PPS、60 fps 固定
```

- 1 turn に**両者が 1 ピースずつ**置く。
- `frame` は**ターン番号から導出**する。実時間は使わない(決定論のため)。
- 実時間の制御(5 PPS のペーシング)はレフェリーの責務だが、**ゲーム状態には影響しない**。

### 2.2 ピース供給

- 共有 7-bag から `bag[t]` を取り、**両者に同じピース**を配る。
- hold / preview は各プレイヤーが独立に持つ(s2-spec §2.5 / §2.6)。
- hold は「その turn で使ったか」を各プレイヤーが持つ。**hold はターンを消費しない**(1 turn に 1 ピース設置)。

### 2.3 1 turn の解決順序

**seat 1 の設置を完全に解決してから、seat 2 の設置を解決する。**

```
for t in 0..1000:
    piece = bag[t]
    for seat in [1, 2]:
        p = ボット(seat) の着手を取得(§4)
        検証(§4.2)
        設置を適用 → 消去 → 採点 → 相殺 → 送信(§5)
        ゴミ挿入(§5.4)
        if トップアウト: ラウンド終了(§6)
```

**この順序が意味を持つ理由**: seat 1 が turn `t` に送った攻撃は、**同じ turn の seat 2 の相殺で消せる**(キューに入っているため)。逆に seat 2 の攻撃は seat 1 の同じ turn の相殺には間に合わない。つまり **seat 2 がわずかに有利**。シートの入れ替えでこの有利を相殺する。

> 相殺は到着(frame)を見ない(s2-spec §7.3)。キューにあるものは全部削れる。20 frame の遅延が効くのは**挿入**だけ(§5.4)。

---

## 3. ラウンド進行

### 3.1 状態

各プレイヤーが持つ状態(s2-spec §9 のとおり):

```
board, combo, b2b_streak, pieces_placed, sent_total,
garbage_queue, garbage_rng, last_column, has_changed_column,
hold, hold_available, next[5]
```

レフェリーが追加で持つもの:

```
turn, frame, seed, sheet, seat_of[slot], log(§8)
```

### 3.2 擬似コード

```
fn run_round(seed, sheet) -> RoundResult:
    d = derive_seeds(seed)                       # s2-spec §11-U7
    players = [Player::new(d.piece_seed, d.garbage_seeds[0]),   # slot 0 = intetrigence
               Player::new(d.piece_seed, d.garbage_seeds[1])]   # slot 1 = attax
    order = match sheet { A => [0, 1], B => [1, 0] }            # seat 順

    for t in 0 .. 1000:
        frame = t * 12
        piece = bag_next()                        # 共有 bag
        for slot in order:                        # seat 1 → seat 2
            move = ask_bot(slot, snapshot(slot, piece))   # 応答を待つ(時間制限なし)
            verify(slot, move)                    # §4.2
            apply(slot, move, frame)              # §5
            if topped_out(slot): return result(slot)
    return timeout_result()                       # §6.3
```

---

## 4. 着手の提出と検証

### 4.1 提出形式

ボットは「どのピースを、どの回転で、どの位置に、hold を使ったか」を返す。TBP では `suggestion` の `moves` 配列(優先順)で届く(attax-integration §5.3)。

**レフェリーが要求する最小情報**:

| フィールド | 意味 |
|---|---|
| `piece` | 置くピース種(hold 使用時は hold されていたピース) |
| `rotation` | 0..3(SRS 状態。0 = spawn) |
| `x`, `y` | **バウンディングボックス左下**(s2-spec §2.2.1 と同じ基準点) |
| `hold` | その turn で hold を使ったか |

> **座標の基準点は「ボックス左下」で固定**。回転中心や最下段セルではない。`crates/core` の `Board::spin_type` / `t_spin_corners` もこの基準点を取る。

### 4.2 検証(拒否したらそのゲームは forfeit、§7)

1. **合法性**: セルが `0 ≤ x ≤ 9` / `0 ≤ y ≤ 39`、既存ブロックと重ならない。
2. **接地**: 1 行下に動かせない(空中で止めていない)。
3. **到達可能性**: `crates/bot` の movegen で spawn から到達できること(PLAN §3.3 の監査)。
4. **hold の整合**: hold を使ったなら `hold_available` が真で、ピースが hold の中身と一致。
5. **スピンラベル**: ボットの主張は**採用しない**。レフェリーが `Board::spin_type` で再計算する(s2-spec §4)。

> 3 は計算コストが高い。**本番では 1 ゲームおきに検証**し、全ゲームは replay 検証(§8.3)で担保する運用でもよい。ただし**拒否 0 件を記録**すること。

### 4.3 第 1 候補の採用

TBP の `moves` は優先順に並ぶ。レフェリーは**先頭から走査し、自前の合法手集合に入る最初のもの**を採用する(attax-integration §8-D3)。

- 採用したインデックスを記録する(0 なら「第 1 候補採用」)。
- **第 1 候補採用率**を毎ゲーム記録(PLAN §3.3)。attax が 1.0 でない場合、その理由を残す。

---

## 5. 1 設置の解決

順序は **s2-spec §6.8 の擬似コードに完全に従う**。レフェリーが独自解釈をしてはいけない。

### 5.1 手順

```
1. ピースを盤面に置く
2. ライン消去を判定(lines, g_cleared)
3. スピン判定(spin)を §4.2-5 のとおり再計算
4. パーフェクトクリア判定(pc)
5. compute_attack(lines, spin, combo, b2b_streak, pc, g_cleared, multiplier(frame))
   → events(送信イベント列)
6. events を順に相殺(§5.2)
7. 残りを相手のキューへ送る(§5.3)
8. ライン消去なしなら、自分のキューから挿入(§5.4)
```

### 5.2 相殺

**イベントごとに** `GarbageQueue::cancel_with_opener(amount, opener)` を呼ぶ(s2-spec §7.3 / §7.4)。

- イベントの順序は `compute_attack` が返す順(surge → 通常攻撃 → PC)。
- `sent_total` は「実際に相手へ飛んだ行数」を累積する(相殺で消えた分は含まない)。
- Opener Phase の判定は `pieces_placed + 1 <= 14` かつ `queue_total >= sent_total`(s2-spec §11-U5)。

### 5.3 送信

残った行数を**1 パケット**として相手のキューに積む:

```
opponent.garbage_queue.receive(remaining, arrival_frame = frame)
```

- `arrival_frame` は**送信した turn の frame**(= `12t`)。
- イベントが複数あった場合、**イベントごとに別パケット**にする(s2-spec §7.2)。合算してはいけない。
- 送信量 0 のイベントはパケットを作らない。

### 5.4 挿入

`lines > 0` なら挿入しない(combo blocking)。`lines == 0` なら:

```
holes = self.garbage_queue.take_insertable(0, frame)   # 最大 8 行、arrival+20 <= frame
self.board.insert_garbage(&holes)                      # 穴の縦順は s2-spec §7.5
```

- **`take_insertable` が返す順序 = 挿入順**。`holes[0]` が**最上段**になる(s2-spec §7.5 の検算例)。
- 遅延: turn `t` に送られたパケットは `frame >= 12t + 20` で挿入可能 → **turn `t+2` 以降**(s2-spec §11-U9)。

---

## 6. 終了条件と勝敗

### 6.1 トップアウト

- 死亡条件は **block-out のみ**(s2-spec §2.3)。lock-out は無い。Clutch Clear は有効。
- 自分の設置を解決した後に、次のピースの spawn が衝突するかで判定する。
- **seat 1 がトップアウトしたら、その turn の seat 2 の設置は行わずにラウンド終了**(seat 2 の勝ち)。
- **同一 turn で両者がトップアウトしたら引き分け**(seat 1 が先に死に、seat 2 も同じ turn で死ぬケース)。

### 6.2 引き分けの扱い

引き分けは**勝ちに数えない**。合格条件は「100 ゲームで 90 勝以上」なので、引き分けは保守側に倒れる。

### 6.3 ターン上限(1000 turn)

1000 turn を終えてもトップアウトしなければ、次の順で決める(**プロトコル上の決定**。TETR.IO のルールではない):

1. **累計受信ゴミ行数が少ない方の勝ち**(`received_total`。挿入された行数の累計)
2. 同数なら **累計送信行数が多い方の勝ち**(`sent_total` の合計)
3. それも同数なら**引き分け**

> 1000 turn = 200 秒。5 PPS で S2 の攻撃量なら、両者が生き残ることはまれ。順位付けは「生き残りに近い方」を選ぶ設計にした。

### 6.4 記録する結果

```
RoundResult {
    winner: Option<slot>,        // None = draw
    reason: TopOut | TurnCap,
    turns: u32,
    final_frame: u64,
    per_player: { sent_total, received_total, attack_events, max_combo, max_b2b, surge_released },
}
```

---

## 7. エラーと forfeit

| 事象 | 扱い |
|---|---|
| ボットの応答が返らない | **待つ**(時間制限なし)。ただし `--timeout` を超えたら forfeit(既定 30 秒) |
| 非合法な着手を返した | **そのゲームは forfeit**。理由と盤面・着手を記録 |
| プロセスが異常終了 | forfeit。stderr を保存(attax-integration §8-A5) |
| プロトコル違反(JSON 不正・必須フィールド欠落) | forfeit。理由を記録 |
| レフェリー自身のバグ(不変条件違反) | **マッチ全体を中断**。ゲームを無効として記録し、原因を修正して再実行 |

- forfeit は**勝ちに数えない**。100 ゲームのうち forfeit が出た場合、その seed を差し替えるか、負けとして集計するかを**レポートに明記**する。
- **レフェリーのバグで無効になったゲームは、seed を再利用して再実行する**(結果を見てから seed を選ぶことはしない)。

---

## 8. replay と監査

### 8.1 replay の記録項目

1 ゲーム 1 ファイル(JSON Lines)。**verify-replay はこのファイルだけで再実行できる**こと。

```
{"type":"header","schema":"intetrigence/s2-replay/1","seed":12345,"sheet":"A",
 "slots":{"0":"intetrigence","1":"attax"},
 "seats":{"1":0,"2":1},
 "rules":"docs/s2-spec.md@v1.3.4",
 "bots":{"0":{"name":"intetrigence","commit":"...","args":["--ms","100"]},
         "1":{"name":"attax","commit":"...","args":[...]}},
 "started_at":"..."}

{"type":"turn","t":0,"piece":"T",
 "placements":[{"slot":0,"rotation":2,"x":3,"y":1,"hold":false,
                "spin":"full","lines":2,"events":[7],"sent":7,"cancelled":0,
                "think_ms":31.2,"candidate_index":0},
               {"slot":1,"rotation":0,"x":4,"y":0,"hold":false,
                "spin":"none","lines":0,"events":[],"sent":0,"cancelled":0,
                "think_ms":28.7,"candidate_index":0}]}

{"type":"garbage","t":0,"from":0,"to":1,"amount":7,"arrival_frame":0}

{"type":"result","winner":0,"reason":"TopOut","turns":87}
```

- **`placements` は seat 順**(解決順)に並べる。verify-replay はこの順で適用する。
- `think_ms` と `candidate_index` は監査用。**再実行には使わない**。
- `events` は `compute_attack` の返り値。**再実行時に再計算して一致を検証する**(採点オーディット)。

### 8.2 盤面ハッシュ

各 turn の終わりに盤面のハッシュを記録する(`type:"hash"`)。verify-replay は再実行後のハッシュと比較する。**不一致は即エラー**。

### 8.3 verify-replay

```
verify(replay) -> Result<(), Mismatch>:
    header から seed / sheet / seats / slots を復元
    derive_seeds(seed) で RNG を初期化
    for each turn record in file order:
        for each placement in seat order:
            合法性・到達可能性を再検証
            apply して events / lines / spin を再計算
            記録された値と比較(events, sent, cancelled, spin, lines)
        hash を比較
    result を比較
```

- **探索の再現は不要**(PLAN §3.3)。着手列が記録されているので盤面進行は完全再現できる。
- 全 100 ゲームで verify が通ることを合格条件に含める。

### 8.4 監査記録(毎ゲーム)

| 項目 | 用途 |
|---|---|
| 両者の think time(median / p95 / max) | 計算資源の非対称を公開(PLAN §6) |
| 第 1 候補採用率(両者) | attax のフォールバック 0 を確認(PLAN §3.3) |
| 非採用時のインデックス分布 | 同上 |
| 採点一致率(suggestion の主張 vs レフェリー実採点) | 採点オーディット(PLAN §3.3) |
| 到達可能性検証の拒否件数 | 0 であること |
| attax の `nodes` / `extra.ms` | attax の実仕事量(attax-integration §8-F2) |
| 両者の commit / 起動コマンド / Bun バージョン | 再現性(PLAN §5) |

---

## 9. 実装の分担(参考)

| モジュール | 責務 |
|---|---|
| `crates/core` | ルール(s2-spec.md)。**frame は引数で受け取る**(内部に時計を持たない) |
| `crates/arena` | ターン管理・席順・提出検証・ゴミの受け渡し・replay 記録・監査集計 |
| `crates/tbp` | 行区切り JSON の送受信、`info`/`rules`/`ready`/`start`/`suggest`/`play`/`new_piece` の状態機械 |
| `tools/` | verify-replay、レポート生成、Wilson CI |

**`core` に frame を持たせない理由**: `GarbageQueue::take_insertable(lines, frame)` は既に frame を引数に取る。レフェリーが `frame = turn * 12` を決めて渡せば、core は純粋なまま決定論を保てる。

---

## 10. 未確定(実装時に決める)

| # | 項目 | 暫定 | 決めるタイミング |
|---|---|---|---|
| P1 | 到達可能性検証を全ゲームでやるか、間引くか | 全ゲーム(コストが許せば) | `arena` 実装時。movegen が 14µs/手なので 1000 turn × 2 = 28ms/ゲーム。**全ゲームで問題ない** |
| P2 | forfeit が出たゲームの集計 | 負けとして集計し、レポートに明記 | 最初の forfeit 発生時 |
| P3 | ターン上限の順位付け(§6.3) | 受信ゴミ少 → 送信多 → 引き分け | 実装時。1000 turn に到達するゲームが出たら見直す |
| P4 | 同時トップアウトの判定(§6.1) | 引き分け | 実装時 |
| P5 | 5 PPS のペーシング方法(実時間で待つか、ターンを回すだけか) | 実時間で待つ(attax の思考時間を測るため) | `arena` 実装時 |
| P6 | replay の圧縮・保存先 | 1 ゲーム 1 ファイル、無圧縮 | 実装時 |

---

## 11. 改訂履歴

| 版 | 日付 | 変更 |
|---|---|---|
| v1 | 2026-11-13 | 初版。マッチ構成・席/slot・ターンと frame・解決順序・提出検証・相殺/送信/挿入・終了条件・replay 形式・監査項目・エラー処理を規定。ルール値は s2-spec.md を参照。 |
