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

- **ピース列は両シートで同一**(`piece_seed` は 1 本。各プレイヤーが自分の袋をその seed から作り、hold で分岐する。§2.2)。
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
- **実時間のペーシングはしない**(sleep を入れない)。レフェリーはボットの応答を待ち、`think_ms` を記録するだけ。
  - 理由: 両ボットとも 1 手の計算量が固定(intetrigence は `--ms 100`、attax は固定仕事量)なので、ペーシングの有無は**強さに影響しない**。実時間で待つと 100 ゲームの所要時間が伸びるだけで、得るものが無い。
  - 代わりに **1 手 200 ms(= 5 PPS の持ち時間)を超えた手を記録し、レポートに件数を出す**。超過が 0 件なら「公称 5 PPS と実測が一致している」と主張できる。
  - 実時間の制御を入れてもゲーム状態は変わらない(`frame` はターン番号から導出するため)。

### 2.2 ピース供給(プレイヤーごとの袋、同一 seed)

- **各プレイヤーが自分の 7-bag を持つ**。両者の袋は**同じ `piece_seed`** から作る(`Game::new(piece_seed)` を 2 つ)。
- したがって **hold を使うまで、両者の現在ピースと preview は完全に一致する**。
- **hold を使うと、そのプレイヤーの袋だけが 1 個進む**(相手の袋は不変)。以後その 2 人の列は 1 個ずれる。
- hold / preview は各プレイヤーが独立に持つ(s2-spec §2.5 / §2.6)。**hold はターンを消費しない**(1 turn に 1 ピース設置)。

**この設計を採る理由**(v1.1 で「単一の共有袋」から変更した):

1. **TETR.IO と同じ構造**。各プレイヤーが自分のキューを持ち、hold は自分のキューから消費する。
2. **hold が列に影響する**ので、hold の損得が TETR.IO と同じ意味を持つ。共有袋にすると hold が列に影響せず、hold が実質無コストになって TETR.IO より強くなる(評価関数の前提も変わる)。
3. 列のずれは**本人の選択だけ**で決まる。運の非対称は無い。
4. `crates/core` の `Game` がそのまま使える(任意ピースを spawn する API が不要)。

> **実装上の注意**: レフェリーは「この turn のピース」を**プレイヤーごとに自分の袋から**取る。共有の `bag[t]` を配ってはいけない。`crates/core` の `Game::hold()` は「hold が空のときだけ `bag.next_piece()` を呼び、埋まっていれば swap のみ」なので、この意味論と一致する。

### 2.3 1 turn の解決順序

**seat 1 の設置を完全に解決してから、seat 2 の設置を解決する。**

```
for t in 0..1000:
    for seat in [1, 2]:
        piece = プレイヤー(seat) の袋から 1 個(§2.2)
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
        for slot in order:                        # seat 1 → seat 2
            piece = players[slot].bag.next_piece()        # 各自の袋(§2.2)
            move = ask_bot(slot, snapshot(slot, piece))   # 応答を待つ(時間制限なし)
            verify(slot, move, piece)             # §4.2
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
5. **スピンラベル**: ボットの主張は**採用しない**。レフェリーが再計算する(s2-spec §4)。規則は §4.2.1。

> 3 は計算コストが高い。**本番では 1 ゲームおきに検証**し、全ゲームは replay 検証(§8.3)で担保する運用でもよい。ただし**拒否 0 件を記録**すること。

### 4.2.1 スピンラベルの決定規則(最大スピン)

**同じセル集合に複数の経路で到達できるとき、スピンラベルは「到達可能な経路のうち最強のもの」を採る**(`none` < `mini` < `full`)。

```
spin(cells) = max over paths p reaching cells of spin_type(cells, last_action(p), kick(p))
```

- **理由**: 本プロトコルはボットから**セル集合**を受け取り、キー列は受け取らない(TBP の `suggestion` にキー列が無い)。経路が不明な以上、決定的な規則が要る。
- **`last_action(p)`**: その経路の最後の操作が回転か(s2-spec §4.1)。回転で終わる経路が 1 本でもあればスピン候補になる。
- **`kick(p)`**: 5th kick を使ったか(s2-spec §3.3)。TST/FIN の昇格は経路依存。
- **実行する経路**: レフェリーは**最大スピンを達成する経路**を 1 本選んで `Game::apply` で再生する。最大スピン同士の経路のどれを選んでも**盤面と採点は同一**なので、同ラベル内の選択は実装依存でよい(ただし決定的であること)。
- **実装**: `crates/core` の movegen と `crates/arena` の movegen が同じ規則を実装している(BFS で各セル集合に対し最大スピンを保持)。

> **TETR.IO より寛容であることを明記する**: 実機では「実際に回して置いたか」で決まるが、本プロトコルは「回して置けたなら回したとみなす」。したがって**回転せずに置いた配置でもスピンとして採点され得る**。両者に同じ規則を適用するので公平ではあるが、TETR.IO のスコアと一致しない。レポートではこの点を明記すること。

> **落とし穴**: セル集合に畳むとき「最初に見つかった経路のラベル」を残すと、I/S/Z のように 180° 対称で同じセルを複数経路が持つピースで **`mini` を落とす**。必ず**最大**を残す。

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

**TBP への反映(`start` 再送)**: 挿入が起きたら、そのプレイヤーのアダプタに **`start` を再送**して盤面を同期させる(attax-integration §8-E1)。

- **挿入しなかった場合は再送しない**(combo blocking で `lines > 0`、または due なパケットが無い)。盤面は `play` で既に同期している。
- **forfeit / ラウンド終了時は再送しない**(ゲームが終わっている)。
- ゴミは**自分のロック時にしか自分の盤面に入らない**(§5.4)ので、相手の turn 中に盤面が変わることはない。再送のタイミングは「自分のロック直後」だけでよい。

---

## 6. 終了条件と勝敗

### 6.1 トップアウト

- 死亡条件は **block-out のみ**(s2-spec §2.3)。lock-out は無い。Clutch Clear は有効。
- 自分の設置を解決した後に、次のピースの spawn が衝突するかで判定する。
- **どちらかがトップアウトした瞬間にラウンド終了**。その turn の残りの設置は行わない。トップアウトした側の負け、相手の勝ち。
- したがって **「同一 turn で両者がトップアウト」は起こらない**(先に死んだ側で打ち切るため)。引き分けはターン上限(§6.3)でのみ発生する。

> **公平性の注意**: 解決順が seat 1 → seat 2 なので、**seat 1 が先に死ねば seat 2 はその turn を打たずに勝つ**。これは逐次解決に固有の非対称で、シートの入れ替え(§1.2)で相殺する。1 ゲーム単位では偏るが、100 ゲームでは各ボットが 50 回ずつ両方の席を経験する。

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
| **候補が空**(`suggestion` に合法手が無い) | 下の「空の候補」を参照 |
| プロセスが異常終了 | forfeit。stderr を保存(attax-integration §8-A5) |
| プロトコル違反(JSON 不正・必須フィールド欠落) | forfeit。理由を記録 |
| レフェリー自身のバグ(不変条件違反) | **マッチ全体を中断**。ゲームを無効として記録し、原因を修正して再実行 |

**空の候補(`suggestion` が空)の扱い**: レフェリーは**自分の movegen でも合法手を列挙して**から決める。

| レフェリーの movegen | 意味 | 扱い |
|---|---|---|
| 合法手が**ある** | ボットの movegen が壊れている | **forfeit**(ボットの失敗) |
| 合法手が**無い** | 盤面が詰んでいる(置ける場所が無い) | **トップアウト**(盤面の責任。相手の勝ち) |

- 両者は勝敗が同じでも**記録の意味が違う**。forfeit は「ボットが失敗した」、トップアウトは「盤面が死んだ」。レポートで混ぜないこと。
- `Game::is_over()`(次 spawn の衝突)だけでは「置ける場所が無い」を検出できないため、この 2 段判定が要る。

**1 設置につき `suggest` は 1 回だけ**。リトライしない(リトライすると `think_ms` の意味が壊れ、公平性も崩れる)。

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

{"type":"turn","t":0,
 "placements":[{"slot":0,"piece":"T","rotation":2,"x":3,"y":1,"hold":false,
                "spin":"full","lines":2,"events":[7],"sent":7,"cancelled":0,
                "think_ms":31.2,"candidate_index":0},
               {"slot":1,"piece":"T","rotation":0,"x":4,"y":0,"hold":false,
                "spin":"none","lines":0,"events":[],"sent":0,"cancelled":0,
                "think_ms":28.7,"candidate_index":0}]}

{"type":"garbage","t":0,"from":0,"to":1,"amount":7,"arrival_frame":0}

{"type":"result","winner":0,"reason":"TopOut","turns":87}
```

- **`placements` は seat 順**(解決順)に並べる。verify-replay はこの順で適用する。
- **`placement.piece` を必ず記録する**。袋はプレイヤーごとに独立しているので(§2.2)、hold の後は両者のピースが異なる。verify-replay は**この値**を使う。
- `turn.piece` は**置かない**(v1.1 で削除)。turn 単位の「その turn のピース」は per-player bag では定義できない。参考値が要るなら `placement.piece` を読む。
- `think_ms` と `candidate_index` は監査用。**再実行には使わない**。
- `events` は `compute_attack` の返り値。**再実行時に再計算して一致を検証する**(採点オーディット)。

### 8.2 盤面ハッシュ

各 turn の終わりに盤面のハッシュを記録する(`type:"hash"`)。verify-replay は再実行後のハッシュと比較する。**不一致は即エラー**。

**ハッシュの定義**(実装間で一致させること):

- 対象: **`occupied` 40 行 + `garbage` 40 行の両方**、両プレイヤー分。`garbage` を含めるのは**穴列の差を検出するため**(occupied だけだと穴の位置ずれを見逃す)。
- 関数: **FNV-1a**(64 bit)。暗号強度は不要で、verify の同一性検査には十分。
- タイミング: 各 turn の**両者の設置が終わった後**(§2.3 の解決順序の最後)。

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
| 両者の think time(median / p95 / p99 / max) | 計算資源の非対称を公開(PLAN §6) |
| **200 ms を超えた手の件数** | 公称 5 PPS との一致を主張するため(§2.1) |
| 第 1 候補採用率(両者) | attax のフォールバック 0 を確認(PLAN §3.3) |
| 非採用時のインデックス分布 | 同上 |
| 採点一致率(suggestion の主張 vs レフェリー実採点) | 採点オーディット(PLAN §3.3) |
| 到達可能性検証の拒否件数 | 0 であること |
| attax の `nodes` / `extra.ms` | attax の実仕事量(attax-integration §8-F2) |
| 両者の commit / 起動コマンド / Bun バージョン | 再現性(PLAN §5) |

---

**`think_ms` の定義**(レポートの数字の意味を固定する):

- **レフェリーが `suggest` を送信してから `suggestion` を受信するまでの実時間**(ホスト側で測定)。
- ボットの自己申告は使わない。attax の `move_info.extra`(自己申告 ms)は **`extra_ms` として別に記録**し、`think_ms` と混ぜない。
- **IPC の往復コストを含む**。attax は subprocess + 行区切り JSON なので、この分だけ `think_ms` が `extra_ms` より大きくなる。レポートでは両方を出し、差を「通信オーバーヘッド」として説明する。

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
| P5 | 5 PPS のペーシング方法 | **実時間で待たない**(§2.1)。`think_ms` を記録し、**200 ms 超過の手を数えてレポートに出す** | 決定済み(v1.3)。超過件数の集計だけ実装 |
| P6 | replay の圧縮・保存先 | 1 ゲーム 1 ファイル、無圧縮 | 実装時 |

---

## 11. 改訂履歴

| 版 | 日付 | 変更 |
|---|---|---|
| v1 | 2026-11-13 | 初版。マッチ構成・席/slot・ターンと frame・解決順序・提出検証・相殺/送信/挿入・終了条件・replay 形式・監査項目・エラー処理を規定。ルール値は s2-spec.md を参照。 |
| v1.1 | 2026-11-13 | **ピース供給を「単一の共有袋」から「プレイヤーごとの袋(同一 `piece_seed`)」に変更**(harness-dev の実装に合わせて確定)。§2.2 を書き換え、§1.2・§3.2 の擬似コード・§8.1 の replay(`placement.piece` を必須化、`turn.piece` を削除)を追随。理由: TETR.IO と同じ構造・hold が列に影響する・`crates/core` の `Game` がそのまま使える。 |
| v1.2 | 2026-11-13 | §6.1 を実装に合わせて修正。**「同一 turn で両者トップアウト → 引き分け」は到達不能**だった(先に死んだ側で打ち切るため)。「トップアウトした瞬間に終了、相手の勝ち」に統一し、逐次解決に固有の席非対称とその相殺方法を明記。 |
| v1.3 | 2026-11-13 | §2.1 にペーシングの方針を明記(**実時間で待たない**。`think_ms` を記録し 200 ms 超過を数える)。§10 P5 を決定済みに。実装(`2dad514`)に合わせた確定。 |
| v1.4 | 2026-11-13 | **§4.2.1「スピンラベルの決定規則(最大スピン)」を新設**。セル集合に複数経路があるとき最強のラベルを採る規則を明文化し、TETR.IO より寛容であることと、畳み方の落とし穴(`mini` を落とす)を明記。 |
| v1.5 | 2026-11-13 | §4.2.1 に「実行する経路」の規則を追加(最大スピンを達成する経路を再生。同ラベル内の選択は実装依存)。§5.4 に **TBP の `start` 再送規則**を追加(挿入時のみ、forfeit 時は送らない、ゴミは自分のロック時のみ入る)。 |
| v1.6 | 2026-11-13 | §7 に**空の候補の 2 段判定**(レフェリーの movegen に合法手があれば forfeit、無ければトップアウト)と「1 設置 1 `suggest`」を追加。§8.2 に **`board_hash` の定義**(occupied + garbage の FNV-1a)。§8.4 に **`think_ms` の定義**(ホスト側の送受信間、IPC 込み、自己申告は `extra_ms` として別記録)。 |
