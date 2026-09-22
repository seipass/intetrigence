# 検証状況と達成条件

## 要求

Rust実装、現行ガイドラインの公開仕様への準拠、Cold Clear 2の最強設定への勝利。同一ハードウェア・思考時間・PPS、固定した相手版・設定・ルール、独立した評価用seed、反復対戦で勝率50%超の統計的確認、再現手順と対戦ログが必要です。

## 現状

- 10×40盤面、下20段が可視、7-bag、ホールド、SRSの90度回転とキック、T-spin判定、ライン消去、B2B・コンボの状態を実装しています。
- 改良版のエンジン差分は限定しています。上流のコンボ更新漏れをライン消去時の加算に修正し、探索木で最善候補が降格したときも親評価を再伝播します。未知NEXTを後から確定した場合も、投機層の平均値を確定ミノの値へ置き換えて既知の祖先まで再伝播します。移動生成はSRSの回転後にソフトドロップしてから再回転する経路も探索し、上流の高速ショートカットが落としていたノッチ配置を保持します。差分は`diff -u vendor/cold-clear-2/src/data.rs engine/src/data.rs`、`diff -u vendor/cold-clear-2/src/dag.rs engine/src/dag.rs`、`diff -u vendor/cold-clear-2/src/movegen.rs engine/src/movegen.rs`で確認できます。
- `src/guideline.rs`には探索エンジンを呼ばない配置レベルのSRS参照実装があります。7種類すべてを53個の決定的な盤面（低いスタック40、高いスタック13）で比較し、スポーン、キック、衝突、重力、ハードドロップ、正規化を検証する`tests/guideline.rs`を実行できます。`src/timing.rs`には公開Guidelineのレベル1〜15の落下速度（60Hzの分数累積）、通常落下の20倍ソフトドロップ、0.2秒のGeneration Phase（60Hzで12フレーム、`Validator::new_guideline`）、0.5秒相当のロックディレイ、Extended Placementの最大15回リセットと低い行への降下時のリセット、60Hzの決定的なDAS/ARR入力変換（18/2フレーム）を分離したフレーム検証器があります。Generation Phaseの値は公開2009プロファイルを採用したものです。特定クライアントの入力順序との照合は未実施で、全面的なガイドライン準拠を証明していません。
- `--rules guideline`では、公開マルチプレイヤー表のSingle=0、Double=1、Triple=2、Tetris=4、T-Spin Single/Double/Triple=2/4/6、Back-to-Back=+1を使い、コンボ攻撃とパーフェクトクリア攻撃を加えません。受信キューは相殺後、各ロックの直後に残りを全投入します。従来のベンチマークは再現性のため`arena`プロファイルを使用します。攻撃表以外のお邪魔穴乱数は各試合で固定seedから生成します。
- Guidelineの改良探索は、探索済みルートを最大64候補まで取得して実際の`Game::play`で再評価します。攻撃量、相殺量、Garbage投入、ロック後の盤面高さ・総高さ・穴・穴の深さ・凹凸、次ミノの生存可能性、候補順位を同じスコアへ入れ、Arenaで使う受信圧力補正はGuidelineへ二重適用しません。ArenaのGarbage上昇後は、14段以上かつ8穴以上の崩れた地形を複数手の回復探索へ切り替えます。これはルール経路と局所戦術の実装であり、単独では勝率の証明ではありません。
- 本体はヘッドレスAIです。描画・操作UI・音楽等のゲーム製品全体の要件は扱っていません。
- TBPの基本メッセージと`seven_bag`宣言に対応しています。`uniform`や`general_bag`は`unsupported_rules`を返します。MVPでrandomizerが省略された場合は`unknown`として受理しますが、ゲーム側のルールを外部で固定する必要があります。nextがまだ届かない短いキューでは、合法なハードドロップへフォールバックします。
- 公式ソースに名前付きの「最強」プリセットはありません。pinned commit の`src/lib.rs::spawn_workers`が`0..1`をハードコードし、設定ファイルも`src/default.json`だけなので、公開ソースで選択できる最大プロファイルを`pinned-public-maximal-single-worker-default`として固定しました。`arena --pps N`で同じターン周期を強制できますが、どのPPS・思考時間を製品固有の「最強設定」と呼ぶかは別途固定が必要です。低予算の開発対戦を最終評価とは扱いません。

## 記録済みの証拠

今回の探索補正に対する開発回帰では、`arena --games 20 --seed 1200001 --ms 0 --iterations 30 --turns 1000`で改良版17勝・同梱ベースライン3勝・引き分け0、決着試合の記述的Wilson 95%区間は63.96%〜94.76%でした。過去の外部Cold Clear 2戦で地形崩壊したseed 1048034と1048096も、`external-arena --games 1 --ms 20 --pps 40 --turns 1000 --no-swap`で各1勝0敗でした。探索選択には乱数があるため、この20試合と単一seed再実行は局所回帰であり、下記の固定済み100試合評価を置き換える勝率証明ではありません。

`results/development/pilot.json` は5ms/手、探索木を毎手リセットする初期評価器での7勝3敗です。開発用の予備データで、最終評価から除外します。独立試合を仮定した参考Wilson区間ですら50%をまたぎ、同一seedの左右交換による相関もあるため成功判定には使えません。

`verify-replay` でpilotの10試合、内部比較の100試合・20試合・追加100試合、公式実行ファイル比較の100試合・各20試合・追加100試合の盤面・対戦結果を再現できました。テストではログ破損と終局記録の欠落を拒否します。これは記録と結果の再生検証であり、入力フレーム時刻や思考過程の再現ではありません。

新規リプレイはJSONL v2です。`start.agents`と各`move.agent`で`intetrigence`と相手を識別し、`start.initial`に開始時のActive/Hold/Nextを含めます。`board_cells`は底から上の40×10セルを`null`・`block`・`garbage`で表します。各着手には`active`、`hold_before`/`hold_after`、`next_before`/`next_after`、`hold_used`を含めます。既存ログの数値`board`と旧スキーマは互換維持します。形式の固定値は[docs/replay-format.md](replay-format.md)に記録しています。

記録済みの100試合マニフェストは、それぞれの対戦を収録した時点のAIバイナリSHA-256を保持しています。現行release/strictバイナリで、Arenaの履歴ログとGuideline外部評価ログを含む全6本の着手ログを`verify-replay`できます。各マニフェストは収録時点の実行ファイルを固定しており、再生検証と同一条件での再収録は区別しています。

`results/benchmarks/benchmark-10ms-100.json` はseed 9001からの50組100試合で、改良版80勝・ベースライン20勝・引き分け0でした。10ms/手、PPS指定なしの旧開発プロファイルで、保守的なseedペア単位の片側95%下限は62.69%です。`results/benchmarks/benchmark-10ms-100.jsonl` は100試合の再生ログで、`verify-replay` が100試合を検証します。この結果は実時間PPSを固定していないため、最終バイナリの評価証拠には使いません。

`results/benchmarks/benchmark-10ms-40pps-100.json` は同じseedペア方式で、40 PPSを固定した50組100試合です。改良版83勝・ベースライン17勝・引き分け0、seedペア単位の片側95%下限は65.69%でした。`results/benchmarks/benchmark-10ms-40pps-100.jsonl` は100試合すべてを`verify-replay`で再検証できます。この測定は同一ソース版・同一マシン・同一10ms探索予算の比較を検証しますが、ソースに「最強」プリセットがないため、Cold Clear 2の無制限探索や外部クライアントのPPS設定に対する証明ではありません。

追加の高予算プロファイル `results/benchmarks/benchmark-100ms-5pps-20.json` は5 PPS・100ms/手、10組20試合で13勝7敗、片側95%下限26.30%でした。現行releaseで固定した未使用seedの `results/benchmarks/benchmark-100ms-5pps-100.json` は同条件の50組100試合で86勝14敗、片側95%下限68.69%でした。両方のログを再検証できます。これは固定した公開ソース版のベースライン比較であり、外部クライアントの未定義な「最強設定」の証明ではありません。

`results/comparisons/cold-clear2/external-retained-10ms-40pps-100.json` は pinned commit からビルドした公式 `cold-clear-2` 実行ファイル（SHA-256はマニフェストに記録）との50組100試合です。改良版73勝・相手27勝・引き分け0、片側95%下限55.69%でした。`external-arena` は通常ターンで`play`/`new_piece`を送り探索木を保持し、garbage上昇で盤面が変わった時だけ`stop`/`start`で再同期します。外部ワーカーは非同期に計算するため、`ms`は厳密な1手同時間予算ではありません。

`results/comparisons/cold-clear2/external-retained-100ms-5pps-20.json` は同じ公式実行ファイルを100ms/手・5 PPSで20試合実行し、改良版15勝・相手5勝・引き分け0、片側95%下限36.30%でした。固定した未使用seedの `results/comparisons/cold-clear2/external-retained-100ms-5pps-100.json` は同条件の50組100試合で改良版77勝・相手23勝・引き分け0、片側95%下限59.69%でした。こちらは実行ファイルにコンパイルされた pinned source の既定設定を使っています（設定ファイルのSHA-256はマニフェストに固定）。高予算の100試合ではこの外部プロファイルについて50%超を統計的に確認できましたが、TBPのgarbage更新制約と非同期時間計測の制約は残ります。

`results/comparisons/cold-clear2/external-strict-100ms-5pps-100.json` は新しいstrictモードでArenaルールの公式実行ファイルと100ms/手・5 PPSの交互窓を与えた50組100試合です。改良版82勝・相手18勝・引き分け0、片側95%下限64.69%でした。strictモードは改良AIの探索中に外部プロセスを`SIGSTOP`し、外部側の窓だけ`SIGCONT`してから提案を読み取り、各窓の終了時に再び停止します。リプレイは100試合すべて検証済みで、マニフェストにstrictバイナリと両実行ファイルのSHA-256、コマンド、固定プロファイルを記録しています。

`results/comparisons/cold-clear2/external-guideline-strict-10ms-40pps-100.json` は、同じ公式実行ファイルを公開Guideline攻撃表・ガベージ規則で動かし、10ms/手・40 PPSの交互窓を与えた50組100試合です。改良版69勝・相手31勝・引き分け0、片側95%下限51.6918%でした。上位64候補を実際の攻撃・相殺・ロック後盤面で再評価するGuideline探索を含む当時のバイナリで収録し、JSONL全試合を`verify-replay`で検証できます。これは収録時に固定したpinned source・既定設定・1スレッド・同一ハードウェアで勝率50%超を確認する証拠ですが、Unixのプロセススケジューリング、TBPのgarbage更新制約、製品固有の入力タイミングを含む全Guideline実装の証明ではありません。

`results/comparisons/cold-clear2/external-guideline-strict-10ms-40pps-100-v2.json` と`.jsonl`は、同じ条件で現行v2リプレイ形式を収録した新しい50組100試合です。改良版77勝・相手23勝・引き分け0、片側95%下限59.6918%でした。リプレイには`intetrigence`/`cold_clear_2`の主体名、開始時と各手のHold/Next、`block`/`garbage`セル種別を含み、`verify-replay`で100試合すべてを検証済みです。

## 未検証の範囲

1. フレーム検証器を特定クライアントのARE・生成遅延・入力順序と照合し、公開Guidelineの出典と採用プロファイルを固定する。DAS/ARRは決定的な採用プロファイルを実装済みだが、製品固有値との照合は残っている。
2. TBPのgarbage更新制約を解消する標準拡張または外部リファリーを固定する。現状はstrictモードでもgarbage上昇時に`stop`/`start`再同期を行うため、その区間の探索木保持までは証明していません。
3. 記録済みの評価はそれぞれ収録時に固定したpinned source・設定・ワーカー数・10ms/手・40 PPSのプロファイルに対する結果です。現行の実時間探索は利用可能なCPUから最大8ワーカーを既定で使い、`INTETRIGENCE_SEARCH_WORKERS=1`で当時の1ワーカー条件を再現できます。製品固有の入力タイミングや名前のない無制限探索を含む主張ではありません。同一seedの左右交換は1組として集計し、引き分けは勝利に数えていません。

## 一次資料

- Cold Clear 2: https://github.com/MinusKelvin/cold-clear-2/tree/ed8b19327b6bd1410ddd873d8611485bd45d8fae
- 2009 Tetris Design Guideline（公開転載）: https://studylib.net/doc/28100586/2009-tetris-design-guideline
- TBP基本仕様: https://github.com/tetris-bot-protocol/tbp-spec/blob/master/text/0000-mvp.md
- TBPランダマイザー拡張: https://github.com/tetris-bot-protocol/tbp-spec/blob/master/text/0001-randomizer.md
- SRSの公開キック表: https://tetris.wiki/Super_Rotation_System

`battle.tet`接続は`battle-bot` featureの任意アダプターです。通常の`cargo build`ではプロトコルとTLS依存を有効にせず、`Game`・`Searcher`・ローカルrefereeはサーバー仕様から独立してビルドできます。接続時だけ`cargo build --features battle-bot`（`ws://`）または`cargo build --features battle-wss`（`wss://`）を指定します。アダプターの探索経路はサーバーの得点・攻撃表を再実装せず、受信した可視状態から合法候補を選び、最終的な到達可能性・固定・消去・勝敗はサーバーへ委譲します。

TBPはガイドライン準拠ゲーム向けの緩い既定値を規定します。公式ガイドライン全体を置き換える資料ではありません。
