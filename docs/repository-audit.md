# intetrigence リポジトリ監査

調査基準は 2026-11-16 JST の `main`、コミット `4b2e103263c97aff7c42636107d52d2c07e94bb9`。ローカルと GitHub の HEAD が一致した。作業開始時に `s2lab_bridge.rs` に探索設定を環境変数で指定する未コミット差分があり、動作検証ではその差分を含むバイナリを使用した。対象は Rust 4 crates、外部 AI アダプター、ベンチと採点、公開資料、追跡済み計測データ、GitHub Issues。権利関係は調査対象外。

監査開始時の状態を番号付きで記録した。現在の検証結果と残件は [修正後の検証](#修正後の検証) を参照。

## GitHub Issues 全件

`gh api --paginate 'repos/seipass/intetrigence/issues?state=all&per_page=100'` で PR を除く全件を取得した。結果はオープン 3 件、クローズ済み 0 件。本文と全コメントを確認した。

| Issue | 指摘 | 現行状態 |
| --- | --- | --- |
| [#1 s2-bot-lab 対応](https://github.com/seipass/intetrigence/issues/1) | TBP で S2 Attax の状態を落としたまま対戦している | Lab への接続は成立した。公式ラウンドは旧 TBP 経路を使い続け、Lab でも選択した回転手が変わる。01、02、05 を参照。 |
| [#2 PLAN.md の肥大化](https://github.com/seipass/intetrigence/issues/2) | 計画と履歴の混在で context rot が起きている | 実測で 1,934 行、約 121 kB、見出し 218 個。先頭の現状と末尾の撤回が衝突する。24–28 を参照。 |
| [#3 Attax の構成と計測](https://github.com/seipass/intetrigence/issues/3) | 幅 48、Φ 無効、キック修正前、合法手フィルター欠落、実データ未公開、調整 seeds 使用 | 38.9% は撤回済み。フィルター復元後も採用手の照合が壊れ、設定の証明と計測データが不足する。旧 TS の `8891e9e` と、[追記](https://github.com/seipass/intetrigence/issues/3#issuecomment-6919255496) で指定された Rust の `c83d339` の両方で再現した。01、03、04、06–09 を参照。 |

## 対戦で相手が選んだ手と強さを保てない

| ID | 優先度 | 問題と根拠 |
| --- | --- | --- |
| 01 | P0・再現 | **選択した Attax の T-spin mini が通常シングルになる。** `acceptRoot` はピース・hold・最終回転・占有セルの組で受理する。採用する `witnessFor` は占有セルだけで最初の候補を選び直す。seed 9001 の第25手では同一セルの回転あり・なしがあり、実際の採用結果は攻撃 0、Attax が選んだ回転ありの結果は攻撃 1。Intetrigence でも第2手で回転フラグが落ちる事例を確認。同じピースの current / hold が競合する場合にも照合キーに hold が効かない。 [run-match.mjs](../adapters/s2lab/run-match.mjs#L59) |
| 02 | P0・コード確認 | **Lab へ移行しても Intetrigence の S2 状態が欠ける。** `lockTime.piecesPlaced` が提供されるのに `pieces_placed=0` を渡し、毎手 opener の加点と倍の相殺を見込む。盤面の `G` セルを通常ミノにするので GSB とゴミ行評価を失う。実フレームと送受信累計も探索に入らない。累計は amount-only decision の公開フィールドにないため、再構築できる範囲を決める必要がある。 [bridge](../crates/arena/src/bin/s2lab_bridge.rs#L45) [評価](../crates/bot/src/eval.rs#L517) |
| 03 | P0・上流照合 | **指定された現行 Attax Rust の通常の対戦経路を使わない。** `c83d339` の上流 runner は `await decideAsync(...)`、`engineIdentity()`、終了時 `close()` を使用。こちらは `decide(...)` を呼ぶので Rust は毎手 `--oneshot`、常駐サーバー側も閉じない。Windows では上流の既定の実行ファイルパスで ENOENT となり、`ATTAX_RUST_CLI` にビルド済み exe のパスを指定して接続した。 [現行 runner](../adapters/s2lab/run-match.mjs#L147) [Attax 側の実装](https://github.com/souta-lab/attax/blob/c83d3397d458f4af9cfd6fe3ba3468df48bb2877/tools/s2-lab/attax-player.ts#L651) |
| 04 | P0・再現 | **表示された設定から実行した Attax を証明できない。** `ATTAX_TREE` は任意の自己申告値。Φ・solvency は環境変数の文字列を記録し、プロファイルから決まる実効値や solvency mode を記録しない。クリーンな `8891e9e` と `c83d339` の JSON はどちらも `attaxWidth:null`。Lab も手元のコピーは Git 管理情報がなく、champion profile の hash は Lab 実装全体を識別しない。自機のバイナリ、重み、実効探索設定、Rust エンジン ID も出力から確定できない。 [run-match.mjs](../adapters/s2lab/run-match.mjs#L232) |
| 05 | P0・コード確認 | **公式ラウンドの Attax は旧 TBP のまま。** ラッパー既定は `7b38f51`、Rust の段階 3 は Attax を TBP に接続する。B2B は bool に圧縮され、滞留ゴミと到着済みゴミ、正確なフレームが対戦側に届かない。現行 Attax の Lab と公式ゲートがつながっていない。 [ラッパー](../scripts/run-round.sh#L40) [ラウンド](../crates/arena/src/round.rs#L533) [TBP start](../crates/arena/src/tbp_agent.rs#L134) |
| 06 | P1・コード確認 | **Lab で一方だけ合法ルートを探索中に絞る。** Attax は `acceptRoot` で不一致を除ける。Intetrigence は最善手1件のみ返すため、キック順などで witness がないと合法な次善手があっても試合停止。Lab の `holdAvailable=false` は空 hold に変換され、探索が空 hold を使えると解釈する。 [runner](../adapters/s2lab/run-match.mjs#L133) [bridge](../crates/arena/src/bin/s2lab_bridge.rs#L111) [探索](../crates/bot/src/search.rs#L690) |
| 07 | P1・再現 | **Lab が起動失敗で止まり、予定試合を欠落させる。** 存在しない bridge で ENOENT、出力 0 バイト、Bun が残留したため監査プロセスのみ終了した。初期化は `try` の外にあり、子プロセス応答にタイムアウトがなく、`error`・stdin エラー経路で pending request を確実に解決しない。試合中のエラーを JSONL に残す経路でもバッチの exit code は 0。 [runner](../adapters/s2lab/run-match.mjs#L97) [decider](../adapters/s2lab/intetrigence-decider.mjs#L8) |
| 08 | P1・コード確認 | **Lab の勝敗ファイルは試合を再生できない。** 保存は集計と最後の receipt が中心で、全ターンの判断・採用 witness・状態・計時がない。JSONL は追記式で重複を検査せず、席替えだけの同一ゲームや調整 seeds を正式な標本から外す仕組みもない。非数・負数・無限などの CLI 入力を拒否しない。 [runner](../adapters/s2lab/run-match.mjs#L31) [出力](../adapters/s2lab/run-match.mjs#L200) |
| 09 | P1・証拠照合 | **38.9% 撤回後の適格な対戦結果がない。** 旧ローカル `games20.jsonl` は 7 勝・11 敗・2 エラーで、PLAN の「未決着」と異なる。 `s2lab-20-attax96phi.jsonl` は 9001–9004 の調整 seed 4 レコードのみで、Attax 全勝、幅 `null`、旧 TS。両ファイルとも Git 非追跡で 01 の影響も切り分けられない。 [PLAN.md](history/plan-2026-11-16.md#L1908) [.gitignore](../.gitignore) |
| 10 | P1・コード確認 | **ラボと S2 Rust レフェリーは別の実験条件。** Lab は current 込み 14 個、1 PPS、ゼロ配達遅延、legacy LCG、上部20行への進入で終了を判定。Rust は current + preview 5 個、別 seed 導出・遅延、spawn を使う TopOut 判定。ラボでの配置勝率、時間上限付きの Rust 対戦、実操作を伴う S2 での強さはそれぞれ実験が必要。 [Lab 設定](../adapters/s2lab/run-match.mjs#L84) [Rust レフェリー](../crates/arena/src/referee.rs#L215) [プロトコル](s2-referee-protocol.md#L98) |
| 11 | P1・パッチ確認 | **CC2 の比較は独自の空 hold 契約に依存。** 互換パッチは current のずれを修正する一方、空 hold 時に次ピースを置く候補を通常の hold 候補として実装せず、hold 候補や T-slot 評価に手を加えている。上流素の強さや完全な空 hold 対応と同一視できない。CC2 と clys は短い疎通で両シートを検証できたが、S2 固有状態を TBP で受け取れない。 [CC2 パッチ](../adapters/cc2-tbp-compat.patch) [CC2 統合](cc2-integration.md#L44) |
| 12 | P1・コード確認 | **CC2 / clys の実効思考予算は自機と非対称で報告値ともずれる。** 外部は前回の `play` などからバックグラウンドで考え、ホストまたは clys アダプターが待機を追加する。ホストは `suggest` 往復と固定待機を合計し、再試行間の sleep や他方の探索中に進んだ計算を測らない。CPU 時間・スレッド・最大待機の統一を確認できない。 [TBP agent](../crates/arena/src/tbp_agent.rs#L275) [clys adapter](../adapters/clys/tbp/adapter.ts#L18) |

## 探索・ルール

| ID | 優先度 | 問題と根拠 |
| --- | --- | --- |
| 13 | P1・コード確認 | **探索の相殺モデルが自前レフェリーとずれる。** 実際には attack events ごとに queue と送信累計を評価し、opener 倍増に `queue_total >= sent_total` が必要。探索は合計攻撃に常に倍増を適用し、送信累計を更新せず、到着済み行も単純に減算する。過大な相殺を前提に積む選択が残る。 [探索評価](../crates/bot/src/eval.rs#L608) [レフェリー](../crates/arena/src/engine.rs#L782) [opener](../crates/core/src/opener.rs#L43) |
| 14 | P1・コード確認 | **先読み中のゴミ到着と挿入、設置後の spawn 危険を近似している。** 評価は現在の pending / arrived の減算と高さペナルティ中心で、ゼロライン時に到着済みゴミが盤面にせり上がる局面を展開しない。得点の倍率も探索内では固定 1.0。押し上がった active も通常 spawn から探索し、実 active の配置は後段に低コスト評価で足す。 [探索評価](../crates/bot/src/eval.rs#L517) [bot agent](../crates/arena/src/bot_agent.rs#L221) |
| 15 | P1・コード確認 | **探索の重複排除が将来価値の異なる状態を統合する。** `state_key` がゴミセルのマスクと incoming / arrived を無視する一方、評価・相殺はそれらを読む。同一 occupancy・queue・chain でもゴミ処理結果が異なる場合、探索枝が消え得る。 [探索キー](../crates/bot/src/search.rs#L913) [評価](../crates/bot/src/eval.rs#L517) |
| 16 | P1・コード確認 | **盤面形状の重みが通常域で二重に掛かり、survival 中も残る。** holes、hole cover、hole rows、bumpiness、row transitions、well depth が無条件項と通常域の分岐に重複。コードコメントは survival で形状項を除去すると記す。 `race` の累計差は子で固定され、同一 root 内の候補順位を原則変えない。 [leaf 評価](../crates/bot/src/eval.rs#L674) |
| 17 | P2・モデル限界 | **未知キューをピース順1本で延長する。** 残り bag の境界をプレビュー内の重複から推定し、未知ピースを固定順で足す。公式 TBP と Lab で可視長が違うため、同じ深さの検証でも現実のプレビューに基づく割合が異なる。 [PieceSequence](../crates/bot/src/search.rs#L159) |
| 18 | P2・検証不足 | **S2 忠実性の未確定部分が外部再現で固定されていない。** I の 90° キック順の和集合は対戦運用上の判断。T mini、PC の surge 充填、RNG、遅延・フレームも仕様書に自決値がある。Rust bot / referee 同士の照合は内部整合性を検証するが、外部の公式挙動を裏付ける fixture とは別。 [S2 仕様](s2-spec.md#L1122) [movegen crosscheck](../crates/arena/tests/movegen_crosscheck.rs) |

## 計測・リプレイ・再現性

| ID | 優先度 | 問題と根拠 |
| --- | --- | --- |
| 19 | P0・再現 | **リプレイ検証が欠損・矛盾を通す。** ヘッダー1行のみでも `ok=true`。単一の必須終端を要求せず、schema や時系列・hash レコードの turn を十分に拘束しない。正常な TurnCap の結果を `Forfeit, winner=0` に変えても `ok=true`。TopOut の winner を 3 にすると配列アクセスで panic。 [replay](../crates/arena/src/replay.rs#L404) [結果検証](../crates/arena/src/replay.rs#L698) |
| 20 | P0・再現 | **resume が異なる計測を混ぜる。** seed 9871 で 4 turn・d2 w16 の結果を保存し、同じ出力先で 7 turn・d4 w32 に変えて `--resume` すると旧結果を再利用した。自機名と build ID が中心で、相手・重み・予算・ルールの一致や相手ログを再確認しない。不完全と報告した旧ゲームも再利用された。 [再利用](../crates/arena/src/harness.rs#L1548) [ラウンド互換判定](../crates/arena/src/round.rs#L1115) |
| 21 | P1・コード確認 | **正式ゲートが実験仕様の一部を強制しない。** 100 ms 超過は要約で報告されるだけで `stage_gate` が参照しない。Attax の許容2件に `Stuck` も入り、診断プロファイルや変更されたターン上限などは勝敗条件の側で排除されない。seed 検査は25エントリであり、重複、調整済み seed、seed コミット後のビルド変更を検出しない。重複 seed は同じファイル名を共有する。 [ゲート](../crates/arena/src/round.rs#L298) [判定](../crates/arena/src/round.rs#L375) [時間条件](s2-referee-protocol.md#L748) |
| 22 | P1・コード確認 | **対戦の実体と記録の build / patch / config が結び付かない。** Rust の build ID は `tbp/src`、Cargo.lock、アダプター、ツールチェーンを対象に含めない。外部 commit と patch SHA は引数・既定値として記録できるが、起動する実行ファイル等と照合しない。既定 clys patch SHA は現行ファイルの SHA と不一致。手元の CC2/clys バイナリ SHA も統合文書記載と異なった。 [build.rs](../crates/arena/build.rs#L19) [round CLI](../crates/arena/src/bin/round.rs#L442) [既定 SHA](../scripts/run-round.sh#L40) |
| 23 | P2・コード確認 | **統計の表示が単位を混同する。** ラウンドの median / p95 / p99 はゲーム別分位点の平均を表示し、全手の分位点にはならない。両シートを含む勝率にゲーム単位の Wilson 区間を当てており、同一 seed 内の相関を扱わない。Lab の席替えを二重計上すると標本数も過大になる。 [ラウンド集計](../crates/arena/src/round.rs#L714) [思考時間集計](../crates/arena/src/round.rs#L804) [Lab README](../adapters/s2lab/README.md#L35) |

## 文書、セットアップ、検証運用

| ID | 優先度 | 問題と根拠 |
| --- | --- | --- |
| 24 | P1・実測 | **引き継ぎの入口に過去の議論が累積する。** PLAN 1,934 行、プロトコル 1,778 行、仕様 1,591 行で、着手前に読むよう求める3文書は計 5,303 行。PLAN では章番号 2–5 が再利用され、「公式構成が確定」が複数箇所にあり、先頭が「接続は健全」「課題は純粋に強度」と述べる。撤回と Lab の話は末尾にある。 [PLAN 冒頭](history/plan-2026-11-16.md#L1) [撤回](history/plan-2026-11-16.md#L1908) |
| 25 | P1・照合 | **現行の対象・構成・ゲートを一意に引けない。** Lab README は撤回された `1f41806`、PLAN 末尾は `8891e9e`、Issue の追記は Rust `c83d339`、公式スクリプトは TBP `7b38f51`。探索は現行候補 d8 w88 とスクリプト既定 d6 w96 が並存する。PLAN は clys に穴重み −30 を提案するが既定は −20、ラウンドでは全段階に同じ `spec.bot` を渡す。README のコマンド説明も実行順と逆。48/50 は 96% であり、50/50 から母勝率 99% 以上は立証できない。 [Lab README](../adapters/s2lab/README.md#L22) [PLAN](history/plan-2026-11-16.md#L1802) [ラウンド設定](../crates/arena/src/round.rs#L523) [README](../README.md#L11) |
| 26 | P1・再現 | **README の標準手順がそのまま動かない。** Cargo に複数 binary があり `cargo run -p intetrigence-arena -- match ...` は曖昧で失敗。 `--bin arena` を指定しても match の seed 範囲は非対応、実際のオプションは `--sheets A,B` と `--attax`。render にも `--bin arena` が要る。README のラウンド例は `ROUND`、事前登録 seed ファイル、CC2/clys の設定を欠き、ラッパーは `ROUND` の検証より先に展開する。 [README](https://github.com/seipass/intetrigence/blob/4b2e103263c97aff7c42636107d52d2c07e94bb9/README.md#L105) [arena CLI](../crates/arena/src/bin/arena.rs#L96) [ラッパー](../scripts/run-round.sh#L30) |
| 27 | P2・照合 | **セットアップと配布が特定 Windows 環境に結び付く。** Attax / bridge の既定は個人絶対パスや `.exe`、clys は `tbp.exe` を固定参照。CC2 統合文書の `cc2-tbp-empty-hold.patch` と README の `attax-vision-40.patch` は現行ツリーに存在せず、clys の PowerShell 例はパス区切りが消えている。 [runner](../adapters/s2lab/run-match.mjs#L16) [CC2](cc2-integration.md#L61) [clys](clys-integration.md#L62) |
| 28 | P1・証拠照合 | **公開証拠と撤回の状態が揃わない。** seed registry は 9011–9020 を未使用とするが PLAN には 9020 までの実測、GitHub #3 にも使用指摘がある。動画 manifest は 20 行 Attax への TurnCap 勝ちを `verified:true` として残す。追跡済み計測データ 2,891 ファイルの一方、ignore は現行の raw を除外する。JSON/JSONL 約 334 MB・104万行超の構文検査は全件通過したが、各 run の公開・再計算・撤回を結び付ける台帳がない。 [seed registry](seed-registry.md#L25) [動画 manifest](../videos/manifest.json#L1) [evidence README](../scripts/evidence/README.md) |
| 29 | P2・コード確認 | **既存テストが実アダプターとの契約を検出できない。** Rust のテストを全通過しても 01、19、20 が再現した。GitHub Actions の workflow・ツールチェーン固定・Lab runner の独立統合テストが追跡されていない。 `cargo fmt --all -- --check` は bridge で失敗した。 [arena tests](../crates/arena/tests/) [bridge](../crates/arena/src/bin/s2lab_bridge.rs#L45) |
| 30 | P2・コード確認 | **計測専有がすべての入口に及ばない。** ロックは同一 Git common dir を共有する worktree のスクリプト間で有効。独立 clone、直接 `arena`/Lab を起動する経路、テスト、異なるプロセスには効かず、SIGTERM 用 trap はロックを解放するだけで明示的に終了しない。未完の計測と stale lock の復旧は手作業。 [measurement-lock.sh](../scripts/measurement-lock.sh#L17) |

## 実行記録と再現手順

- `cargo test --workspace --release --locked` は **299 件成功、8 件 ignored**。 `cargo clippy --workspace --all-targets --locked -- -D warnings` は成功。Windows、Rust 1.98.1、Bun 1.4.0 のローカル検証。
- CC2 `ed8b193` と clys `ccae45f` の手元のバイナリで、seed 9001 の A/B 各5 turn が TurnCap まで進み、4/4 replay 検証成功。clys は `--tbp-program bun --tbp-args <adapter>,--clys-dir,<checkout>`。現行互換パッチの reverse check は両方成功した。対戦数とターン数は疎通確認用。
- Lab の再現は `ATTAX_DIR` にクリーンな `8891e9e`、`S2_LAB_ROOT` にローカル Lab、`ATTAX_S2_STOCK_POTENTIAL=1`、`S2LAB_BRIDGE` に release binary を指定。 `bun adapters/s2lab/run-match.mjs --games 1 --seed 9001 --max-locks 16 --out <新規JSONL>` は `left:8, right:8, attaxWidth:null`。01 の確認では一時コピーで `witnessFor` の戻り値と `witnesses.get(witnessKey(searched.best.placement, searched.best.usedHold))` を比較し、同じ盤面から両方を `applyTransition`。 `turnNumber=24`、右側の T で `none / 0` と `mini / 1` が分岐した。
- 別の一時 checkout で Attax `c83d339` を `cargo build --release --locked --manifest-path rust/Cargo.toml -p attax-search-cli` でビルド。 `attax-agree.exe --version` は `engine:"rust"`、`build:"c83d3397d458f4af9cfd6fe3ba3468df48bb2877"`。 `ATTAX_DIR` と `ATTAX_RUST_CLI` をこのビルドに変え、`--max-locks 2` の疎通で `attaxWidth:null` を再現。比較用の一時 runner を `--max-locks 60` で動かすと、同一局面で通常シングルと mini の分岐を再現した。
- リプレイ欠損は `videos/replays/seed-11004-sheet-A.jsonl` の先頭行だけを一時ファイルに保存し、`cargo run --release -p intetrigence-arena --bin arena -- verify <一時ファイル>`。結果は `ok=true turns=0 placements=0 mismatches=0`。
- 再利用は `arena match --no-attax --seeds 9871 --sheets A --turns 4 --depth 2 --width 16 --deterministic --out <一時ディレクトリ>` の後、同じ出力先で `--turns 7 --depth 4 --width 32 --resume`。結果は `1 verified game(s) reused`。同じ replay の結果行を `reason=Forfeit, winner=0` に変更すると `ok=true`、`reason=TopOut, winner=3` で panic。

監査開始時の実行範囲は短い再現と疎通まで。その後の `c83d339` Rust ビルドとの配置対戦は [50先の結果](attax-first-to-50.md) に記録した。

## 追加調査

| ID | 確認した問題 |
| --- | --- |
| 31 | リプレイが0件のディレクトリで `arena verify` が終了コード0を返した。 |
| 32 | リプレイを置かず、実際の短い対局の監査 JSON を勝利へ書き換えた。 `arena report` は監査ファイル内の検証フラグを読み、「検証済み 1/1、1勝」を出力した。 |
| 33 | 対戦相手が未設定で段階が失敗した `round run` が終了コード0を返した。 |
| 34 | Rust レフェリーは候補採用時と適用時の movegen で、相手の `supports_180=false` を引き継いでいなかった。また、BotAgent が渡す hold はピース種から再推定され、同種の hold を表現できなかった。 |
| 35 | Rust レフェリーの事前 TopOut 判定が、すでにスポーンした current の状態に代えて次のプレビューのスポーンセルを確認していた。 |
| 36 | 採点監査側が候補の申告攻撃量を使わず、申告 spin をもとにレフェリーと同じ式で攻撃量を再計算していた。 |
| 37 | `round --deterministic` はラウンドとリプレイに決定論設定を記録したが、実際の探索設定には値を渡していなかった。修正後、CC2 と両シートの短い対戦を行い、リプレイの自機設定が `mode=fixed` であることを確認した。 |
| 38 | 別の checkout から実行したラウンドバイナリが、起動元の seed ファイルとビルド時に埋め込んだ checkout の HEAD を組み合わせて判定していた。起動元の Git ルートに統一し、一時 Git リポジトリの新規割当を受理することを確認した。 |
| 39 | `run-round.sh` が `DRY_RUN=1` にも `--max-stages 2` を渡し、CLI のドライラン既定である1段階を上書きしていた。現在の評価ラウンドで使わない旧 Attax の設定も渡していた。明示した段階数だけを渡すように変更し、CC2 の両シートを使ったドライランで1段階とリプレイ2局を確認した。 |
| 40 | All-Mini+ の3隅未満の immobile T をスピンなしと判定していた。TETR.IO 公式 BETA 1.5.0 と Lab のフィクスチャを照合し、同じ盤面で旧実装の `None` を再現した。探索とレフェリーの共通判定を修正した。 [仕様と出典](s2-spec.md#u2-t-の-mini-判定における-immobile-の扱い) |
| 41 | 余計なゴミ送信レコードを検出した後、実際の送信数より大きい添字を送信数から引いていた。旧バイナリは `18446744073709551615` 件の欠損を報告した。実際に不足した場合だけ欠損を報告するようにした。 |

修正前の観測を保持するため、本文のコード参照は監査開始時の位置を示す。Rust レフェリーの修正後、31・32 を同じ入力で拒否することを確認した。CC2 と clys の両シートで12ターンの接続、リプレイ検証、同一条件の再開、変更した深度での再開拒否を確認した。

## 修正後の検証

- Attax の現在の比較には [Rust 版との50先の結果](attax-first-to-50.md)を使用する。旧測定を再集計して新成績に加えることはできない。
- リプレイの構造・結果、設定の違う再開、TBP の transcript と採用手の一致を検査する。両外部 TBP ボットとの短い試合は両シートで検証した。
- PLAN は [現行の入口](../PLAN.md) と [過去の実験記録](history/plan-2026-11-16.md) に分かれた。両 OS の CI 定義を追加した。ローカル実行は Windows。
- Rust の公式ラウンドと Lab の `--first-to` は、過去の使用済み範囲と追跡済みラウンド割当、同一端末ユーザーの共有台帳との重複を拒否する（21）。Lab でシード62011を使った後、別の Git リポジトリで同じシードを含む Rust ラウンドを拒否した。Lab は Rust 形式の割当と同じシードを拒否し、同じ出力と設定の再開は受理した。Rust は実行前にバイナリと起動元のソース指紋も照合する。 [シード台帳](seed-registry.md)
- CC2（11）は探索グラフで空 hold を展開するよう修正した。旧パッチでは O の通常候補がないと次の I を返せなかった。現行パッチでは初手と数手後の hold、2枚補充、後着プレビュー、同種ピースを TBP の回帰テストで確認した。Rust レフェリーでも2 seed・両シート・各20ターンの4局をすべて再検証した。 [再現テスト](../adapters/cc2-hold-smoke.mjs)
- TBP（12）は CC2 と clys に共通の100 msのリクエスト探索枠を追加し、公式ラウンドは起動時に相手の対応を照合する。旧方式で対戦前の待ちを35 msから350 msにすると、CC2 は75,426から553,780ノード、clys は113,376から1,042,909ノードに増えた。新方式の実測例では CC2 は148,139と166,976ノード、clys は両方312,097ノード。応答には開始済みの展開、停止処理、入出力の時間も入る。CC2 と clys の両シートで短い4局を再検証した。 [回帰テスト](../adapters/tbp-request-window-smoke.mjs)
- 探索（17）は、固定順の補完を廃止し、current と公開プレビューだけを使う。プレビュー5枚のとき、空 hold なら最大5手、埋まった hold なら最大6手まで。同じ深さで hold の有無を比較し、時間切れの代替手にも未知ピースを使わない。CC2 戦は d8 w16 で両シート・各12手を再検証し、全24手が深度6だった。公開キュー14枚の Attax Lab 経路では、同一 seed の旧・新 bridge で全60手の応答が一致し、両方の記録を再検証した。 [探索実装](../crates/bot/src/search.rs)
- Rust レフェリーと単独対戦は、Clutch で押し上がった場合も実際の active と hold 可否を探索に渡す（14の一部）。シングルで21行目にブロックが下がり、次のピースが1行上に出る盤面で、探索の第1候補をレフェリーが採用してロックできることを確認した。hold 後は交換したピースの通常 spawn から探索する。 [回帰テスト](../crates/arena/src/bot_agent.rs)
- 到着済みゴミは消去なしの枝で最大8行を挿入して評価する（14の一部）。穴位置は同一列の10通りで葉評価を平均し、その枝では挿入後の具体的な手を確定しない。ライン消去で挿入を遅らせた枝は引き続き先読みする。ゴミ3行の挿入で旧探索が4手先まで進んだ局面は1手で止まり、8行で次の出現が塞がれる局面では敗着と評価する。Rust の探索と対戦は Clutch を含む出現判定を共有し、Lab の探索は20行の天井判定を使う。 [探索実装と回帰テスト](../crates/bot/src/search.rs)
- ゴミ挿入を評価するビルドで Attax と Lab の疎通を両席で実施した。試験シード9001・9002の2局で、自機はゴミが到着済みの局面を40手、実際のゴミ挿入を58行経験した。Attax の TopOut 勝利1局、上限到達1局を再検証した。 [ローカルリプレイ](../out/garbage-projection-lab.jsonl)
- 探索の各ロックで、受信済みゴミの到着と攻撃倍率を更新する（14）。Rust レフェリーからは受信済みパケットの行数・フレーム、Lab からは公開行数と既定のロック間隔を使用する。Lab の1 PPS・追加配達遅延0の条件では、現在分は公開された挿入可能行数を使い、次のロックまでに既存の未到着分が到着する。相殺済み行はキュー先頭から差し引く。フレーム12受信の3行は、24では挿入せず36で挿入を評価した。倍率はマージン前後でコアの計算とビット単位で一致した。 [回帰テスト](../crates/bot/src/search.rs)
- 時間を反映したビルドで Attax と短い試合を2局実施し、両方を再検証した。自機の各局80手で、連続するロックの間隔はすべて60フレーム。滞留ゴミがあり、現在のロックで挿入できる行が0の局面を27手確認した。 [ローカルリプレイ](../out/lock-timing-lab.jsonl)
- All-Mini+ 修正後、同じ着手が旧判定では `None`、新判定では `Mini` になることをRustレフェリーで確認した。新規リプレイに判定を記録し、直前版の2局と新規2局を再検証した。余計なゴミ送信を1件加えたリプレイは、その1件だけを不一致として拒否した。 [回帰テスト](../crates/arena/src/engine.rs)
- パーフェクトクリア後にB2Bを切る境界（18、仕様 U3・U4）をLabとRustで照合した。PC前の内部B2Bが3・4・5のとき解放は0・4・5行で一致した。実クライアントでこの手順を踏んだリプレイの照合は未確認。 [仕様](s2-spec.md#u3-pc-の-b2b-加算が-surge-の溜めに寄与するか)
- 過去の動画用リプレイ10件は、ヘッダーが指す `aca615c568a5bd6ada2d2cc256f3fa7b26e8507d` の `arena` で再検証し、すべて不一致0（28）。再現は当該コミットの checkout で `cargo build --release --locked -p intetrigence-arena --bin arena` を実行し、生成された `arena verify` に現在の `videos` ディレクトリの絶対パスを渡す。 [動画のマニフェスト](../videos/manifest.json) · [各リプレイのSHA-256付きローカル検証記録](../out/historical-video-replay-verification.json)
- `arena match`、`round run`、Lab ランナー、Bash スクリプトは、checkout に依存しない同一ユーザーの計測ロックを共有する（30）。Windows で Lab 計測中に2つの Rust CLI を拒否し、別 checkout で Rust が計測中に Lab を拒否した。Bash の取得済みロックを子 CLI が引き継ぎ、正常終了と出力エラー後にロックが解放されることを確認した。 [ロックの運用](../scripts/evidence/README.md#measurement-lock)

### 現在の残件

- 将来の相手の攻撃は探索の入力に含まれない。Lab の穴位置とパケット境界は非公開で、複数の穴列が混じる挿入の盤面評価は近似している（14）。
- 50先は Lab の配置対戦。実 TETR.IO 入力対戦との規則・時間・TopOut 判定の照合が残る（10、18）。
- CC2 と clys では100 msの探索枠を確認した。応答全体の100 ms上限、正式な評価ラウンドの完走、外部実行物と依存の完全な固定が残る（12、21、22）。
- 別端末の未公開実行をまたぐ seed 割当の同期と、計測を強制終了した場合のロックの自動回収が残る。計測ロック外のビルド・テスト・OS負荷は実行時に確認する（21、30）。
