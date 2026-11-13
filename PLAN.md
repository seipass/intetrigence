# intetrigence 再建計画 — attax に勝率 90% (TETR.IO S2 ルール・完全新規)

## 1. ゴール

> **goal文(圧縮耐性版)**
>
> TETR.IO Season 2(TETRA LEAGUE デフォルト相当: B2B Charging/surge, All-Mini+, Opener Phase, Multiplier コンボ)で動く versus Tetris AI「intetrigence」を、旧実装(CC2 派生)に依存せず Rust でゼロから構築する。S2 ルールエンジン・レフェリー・監査機能(思考時間記録・verify-replay・採点オーディット)も自作する。ライセンスは AGPL-3.0(ハーネス周りは attax コードの利用可・作者許可済み、本体 bot は独自実装)。
>
> 達成条件: attax(https://github.com/souta-lab/attax の S2 対応版、接続 I/F は作者合意のものを使用)との公式マッチ — 両プロジェクトで未使用の held-out seeds 50 個 × 両シート = 100 ゲーム、5 PPS、思考 100ms、5 preview + hold、全ゲーム replay 検証 — で **90 勝以上(点推定 ≥ 90%)** を達成し、Wilson 95% CI・attack/piece・受信ゴミ・両者の思考時間を明記した再現可能なレポートを docs/ に残すこと。
>
> コンテキストを失った場合は、まず本書(PLAN.md)と docs/s2-spec.md(S2 ルール仕様)を読んでから再開すること。

前提:
- 旧 intetrigence (CC2 派生) のコード・資産は**使わない**。レフェリー/ハーネスも含めて完全新規
- ルールは guideline ではなく **TETR.IO S2**(TETRA LEAGUE デフォルト相当)
- intetrigence は S2 ネイティブエンジン(ゴミキュー・surge・opener phase を含む全状態を参照)
- **attax は S2 対応予定(作者確認済み)**。マッチは S2 ネイティブ同士の真の勝負として行う
- 現行の guideline 版 attax(TBP 接続)は、ハーネスのブリングアップと暫定スパーリング用

## 2. 敵の分析(README + ソース参照)

### attax の仕様
- 探索: beam search, **width 48, depth 6**(98.7%の手), 現在ピース + 5 preview, hold 対応
- movegen: 入力空間(shift / soft drop / SRS 回転+kick)上の完全 BFS。取りこぼしも不正プレイもない
- 評価: Dellacherie 系盤面特徴量 + versus 項(受信ゴミ込み危険度², T-slot readiness), 攻撃報酬, B2B 維持, wasted-T 罰。**手設定の固定重み 1 セット**
- 思考: 固定仕事量, median **35ms/手**, ponder なし
- 制約: **TBP 経由で pending garbage が見えない**、**下 20 段しか見ない**、**bag 状態を無視**
- **ソース参照可(作者許可済み)**: 重み・特徴量・movegen・探索パラメタを直接確認でき、敵分析は README 依存から脱する

### attax の戦績
| 対戦相手 | 結果 |
|---|---|
| Hoiko レプリカ | 195–5 (97.5%) |
| 旧 intetrigence (guideline, 100ms/5PPS) | **100–0** |
| clys (MCTS beam, width 500) | 88–12 |

### attax の S2 対応(作者言及済み・連携は前提としない)
- S2 対応の範囲(攻撃テーブル / surge / opener / all-spin / ゴミキュー / 180°)は**公開された実装を見てから確定する**。作者とのすり合わせは行わない(個人プロジェクト・AGPL 利用許可のみ取得済み)
- **S2 対応 attax は guideline 時より強くなる**(特にゴミキューが danger 特徴量に乗る)。90% のハードルは「ハンディ付き attax」より上がるが、それこそが取りたい主張
- 残る可能性がある構造的制約: 下 20 段視野, bag 無視, 固定重み 1 セット, 固定仕事量 (median ~35ms), ponder なし

→ ルール不一致の「言い訳の余地」は消え、純粋な実力勝負になる。

### 勝ち筋の仮説
1. **計算量**: attax は 35ms 固定。こちらは 100ms anytime 検索 + マルチスレッドで実質 3〜10 倍思考
2. **S2 ネイティブ評価**: surge 溜め/放出タイミングの最適化、B2B charging、all-spin による B2B 維持、combo multiplier の活用、opener phase の相殺×2、PC +5、ゴミ込み Quad/TSD +1
3. **ゴミキュー込みの危険度・downstack 評価**: 遅延挿入を織り込んだ生存評価。相手もキューを見るようになるため、優劣は評価と探索の質で決まる
4. **探索の質**: depth 6 / width 48 超え。transposition 共有グラフ探索 + T-slot cutout(CC2 の公開アイデア)で実効深度を伸ばす
5. **過学習防止**: attax 直接最適化は tuning seeds のみ。S2 ボットプール(CC2-S2 系・自己対戦)で汎化を担保

## 3. ルール仕様(TETR.IO S2)

### 3.1 本命プロファイル(TETRA LEAGUE デフォルト相当・確定)

| 項目 | 設定 |
|---|---|
| 攻撃 base | Single 0 / Double 1 / Triple 2 / Quad 4 / TSM 0 / TSS 2 / TSD 4 / TST 6 / mini Double 1 |
| コンボ | base × (1 + 0.25 × combo)、切り捨て。base 0 は ln(1 + 1.25 × combo)(combo ≥ 2) |
| B2B | **B2B Charging**: difficult clear 毎に +1(フラット)。streak ≥ 4 で surge 溜め(= streak 数)、非 B2B 消しで全放出(3 分割) |
| All-Spin | **All-Mini+**: T 以外は immobile 判定で mini 扱い(SS 0 / SD 1)、B2B は維持・加算。T は 3-corner / 2-corner + 5th kick ルール |
| パーフェクトクリア | +5、B2B **+1**(確定。+2 は S2 プレシーズン表記) |
| ゴミ込みボーナス | ゴミを含む Quad / TSD 等は +1(Garbage Special Bonus) |
| Opener Phase | 最初の 14 ピース、相殺 ×2 |
| ゴミ | change-on-attack(TL は messiness_change = 1.0、攻撃間で必ず列を引き直し。×2.5 は QUICK PLAY の記述)。**キュー + 遅延挿入(遅延 20 frame、cap 8 行/ロック)**、送信攻撃はキュー相殺が優先 |
| 回転 | **SRS+**(I の kick 修正)+ **180° 回転**(独自 kick) |
| 盤面 | 10×20 可視(内部 40 行)、block-out のみ(lock-out 無効)、**Clutch Clear 有効**(docs/s2-spec.md §2.3) |
| ランダマイザー | 7-bag, 両者同一 seed。5 preview + hold |
| ペース | 5 PPS(ペアド ターン), 1000 turn cap |
| 思考時間 | intetrigence `--ms 100`(マルチスレッド可), attax は S2 版デフォルト設定(作者と合意) |
| シート | 各 seed を両シートでプレイ |
| シード | 両プロジェクトの docs/benchmark 未使用の新規 held-out seeds |

**合格条件(確定)**: 新規 held-out seeds 50 個 × 両シート = **100 ゲームで 90 勝以上**(点推定 ≥ 90%)。Wilson 95% CI も併記

**数値定数の正は docs/s2-spec.md(v1.2 凍結・18 fixture を独立 2 実装で検証済み)**。未確定 9 件(U1〜U9)は同 §11:要実機確認 U1/U2/U4/U6/U8、**U3/U5/U7 は自決**(作者確認は任意・非ブロッキング)。

### 3.2 仕様の一次ソース(v1.2 でフリーズ済み → docs/s2-spec.md)

- **docs/s2-spec.md が唯一の正**(自前仕様)。未確定項目は我々が暫定自決し、新情報が得られたら更新する
- tetrio.wiki.gg(Mechanics / Spins / TETRA LEAGUE)
- tetris.wiki「TETR.IO」(Multiplier コンボ / B2B Charging / All-Mini+)
- tetr.io patch notes(定数: ゴミ遅延・cap・surge 分割)
- 61び s2-bot-lab + note.com S2 解説(動く S2 再実装。相互検証ターゲット)
- chouhy/cold-clear-2(all-spin 処理の参考実装)

### 3.3 対戦ハーネスと公平性

- レフェリーは intetrigence 自前(S2 実装)。**作者監査は前提としない**が、公開時の信頼性のため以下を最初から実装:
  1. **飢死防止**: レフェリーはボットの回答を待つ
  2. **思考時間記録**: 両者の median / p95 think time を毎ゲーム記録
  3. **first-choice acceptance ログ**: attax の第 1 候補採用率(フォールバック 0 を確認)
  4. **verify-replay**: 全ゲームを replay から再実行して一致検証
  5. **採点一致オーディット**: suggestion が予測した攻撃とレフェリー実採点の一致検証
- **attax S2 版は公開された I/F(TBP が基本)で接続する**(作者との事前合意は不要・利用許可済み)。S2 状態(ゴミキュー等)は I/F が運べる範囲で渡す。公開版が出るまで現行 TBP 版で配管を先に作る
- 暫定運用: 現行 TBP 版 attax を S2 レフェリーに接続して配管テスト(ハンディありの参考値)
- intetrigence の探索は anytime のため非決定論になりうる。verify-replay は**記録された着手列の再実行**で検証する(探索の再現は不要)。seed + 着手列があれば盤面進行は完全再現可能

### 3.4 セカンダリ計測(任意・合格条件ではない)

- guideline 条件での対 attax(お互いのホームを交換した参考戦績) → 汎化確認用(信頼性の担保はもう不要)
- s2-bot-lab の CC2-S2 系ボットとの対戦(外部 S2 プール)

## 4. アーキテクチャ(Rust workspace)

```
intetrigence/
├─ crates/
│  ├─ core/        # 純粋 S2 ルールエンジン: bitboard, SRS+ + 180° kick, 7-bag, hold,
│  │              #   攻撃式(base×combo), B2B charging/surge, all-spin, PC, opener phase,
│  │              #   ゴミキュー(遅延/cap/相殺/change-on-attack)
│  ├─ bot/         # exact movegen(180° 含む BFS), anytime 探索(マルチスレッド),
│  │              #   S2 評価関数, 重み
│  ├─ arena/       # S2 レフェリー + TBP host + ベンチハーネス + replay 検証 + 監査ログ
│  └─ tbp/         # TBP メッセージ I/F(仕様: tetris-bot-protocol/tbp-spec, tbp-rs 検討)
├─ tools/          # SPSA チューナ, 敗因分析, レポート生成
└─ docs/           # 仕様書, 計測レポート, 敗因分析
```

- core は決定論的・純粋(seed + 入力列 → 状態)。replay / ベンチ / 実戦が同一コードを共有
- intetrigence 自身の外部公開 I/F は後で決める(s2-bot-lab 互換 or TBP)。本命マッチは自前レフェリー内で完結

## 5. マイルストーン

### M0: 計測基盤(完全自作)
- S2 レフェリー実装(§3.1 の条件 + §3.3 の監査機能)
- TBP host 実装 → 現行 attax を `attax-tbp.sh` 経由で接続、スモークテスト(配管確認)
- **連携前提を廃止**: 仕様は docs/s2-spec.md に自決して固定。attax S2 版は公開 I/F 経由で接続
- **合否条件**: attax との 10 ゲームが replay 検証付きで回り、シート入れ替え・seed 固定・監査ログが機能すること

### M1: S2 コアエンジン(仕様フリーズ)
- §3.2 のソース突き合わせで仕様確定(ゴミ遅延/cap、PC の B2B 加算、surge 分割、All-Mini+ の T 判定)
- bitboard 盤面, SRS+ / 180° kick, all-spin 判定, 攻撃式 / surge / opener phase / ゴミキュー
- exact movegen(180° 含む)+ プロパティテスト(全到達配置、spin 判定、採点)
- 61び s2-bot-lab との相互検証(同局面での採点一致)
- **合否条件**: 合法性テスト全通 + 採点オーディットが attax suggestion と一致

### M2: 検索
- まず beam(width ≥ 96, depth ≥ 8, hold + bag 予測込み)で attax 互換
- 次に transposition グラフ探索 + T-slot cutout, anytime 化(100ms 使い切る)
- **合否条件**: 対 attax(S2)で勝率 5 割超(tuning seeds 上)。S2 版 attax が未提供の間は現行 TBP 版を暫定相手とし、結果は参考扱い

### M3: S2 評価関数とチューニング
- 特徴量: Dellacherie 系 + versus 項(ゴミキュー込み危険度², T-slot readiness, downstack 効率) + **S2 項(surge 溜め, B2B charging, all-spin による B2B 維持, combo multiplier, opener 相殺, PC)**
- SPSA 等で win-rate 直接最適化(対 attax は tuning seeds のみ)
- **合否条件**: tuning seeds で対 attax 勝率 ≥ 85%

### M4: 実験サイクル
- 敗因分類(surge スパイク埋没? downstack 失敗? opener 負け? T 浪費?)→ 評価修正のループ
- exploit 検証: surge 放出の到達率, 高盤面(20 段超)・depth 6 超え局面での崩壊率, opener の相殺勝負
- **合否条件**: held-out 予備 seeds 20 個 × 両シートで ≥ 85%

### M5: 最終計測と公開
- 新規 held-out seeds 50 個 × 両シート, 全ゲーム replay 検証
- attax README と同等の証拠水準でレポート(Wilson CI, attack/piece, 受信ゴミ, 両者の思考時間・設定・attax S2 版のコミット固定を明記)
- **合否条件**: 100 ゲームで **90 勝以上**(点推定 ≥ 90%)。Wilson 95% CI も併記

## 6. リスクと対策

| リスク | 対策 |
|---|---|
| S2 ルール再現のバグが勝敗を歪める | §3.2 ソース突き合わせ + s2-bot-lab 相互検証 + 採点オーディットを M1 で先に固める |
| S2 対応 attax は guideline 時より強く、90% はさらに厳しい | M2 で 5 割 → M3 で 85% の段階ゲートで早期検証。旧 0–100 の再燃は S2 ネイティブ評価設計で回避 |
| attax S2 対応の範囲・時期・I/F が未確定 | 公開されたら公開 I/F で接続。現行 TBP 版で配管を先行。対応範囲は公開版の実装から読み取る |
| 90% は野心的(attax に勝った bot は現状 0) | M2 で 5 割 → M3 で 85% と段階ゲートで早期発見 |
| attax への過学習 | tuning と held-out の厳密分離 + S2 対戦プール |
| 計算資源の非対称への批判 | 思考時間・スレッド数を毎ゲーム記録・公開 |
| attax が AGPL(作者許可済み: AGPL 準拠で利用可) | ソース参照・移植は自由。attax 由来コードを intetrigence に含める場合は AGPL-3.0 で公開(ソース開示)。改変なしの subprocess 実行は元々制限なし |

## 7. 決定事項と残件

### 確定
1. **ルール**: TETR.IO Season 2(TETRA LEAGUE デフォルト相当: B2B Charging, All-Mini+, Opener Phase 有効, Multiplier コンボ)。**仕様は docs/s2-spec.md に自前で固定**(作者との共有仕様は前提としない)
2. **attax S2 版**: フル S2 デフォルト対応(攻撃テーブル / surge / opener / all-spin / ゴミキュー / 180°)を期待。接続 I/F と時期は実装時に柔軟に決定
3. **合格条件**: 新規 held-out seeds 50 個 × 両シート = 100 ゲームで **90 勝以上**(点推定 ≥ 90%)。Wilson 95% CI も併記
4. **ベンチ環境**: ローカル機(AMD Ryzen 7 3700X, 8C/16T, 16GB, Windows 11)。intetrigence のスレッド上限はデフォルト 8(物理コア数)
5. **評価関数**: まず手特徴量 + SPSA。NN 系(NNUE / 自己対戦)は M3 ゲート未達時の拡張枠
6. **attax の利用**: 作者許可済み。AGPL 準拠でソース参照・コード移植ともに可(由来コードを含める場合は AGPL-3.0 で公開)
7. **ライセンス**: intetrigence は AGPL-3.0 で公開。ハーネス周り(レフェリー/TBP host/監査)に attax コードを利用可(作者許可済み)、本体(bot)は独自実装
8. **作者との関係**: 個人プロジェクトとして AGPL 利用許可のみ取得。連携・監査・事前合意・進捗共有は行わない。attax は公開リポジトリ・公開 I/F 経由で使用し、未確定仕様は自決して文書化する

### 残件
- 期限(未定でもマイルストーン駆動で進行可)

## 8. 進捗ログ

- **2026-11-13 Wave 1 完了・受理**: docs/s2-spec.md v1.2 凍結(PC B2B +1、ゴミ cap 8/遅延 20f、TL messiness 1.0、surge 3 分割 [r,r,S-2r] を決着。18 fixture を独立 2 実装で検証)。attax を 7b38f51 に固定(bun 1.4.0、build/test 緑)、TBP smoke 14/14 成功(エビデンス scripts/evidence/attax-tbp-smoke/)、docs/attax-integration.md + docs/attax-analysis.md 作成(敵の重み・特徴量を記録)。crates/core 基盤 + テスト 105 緑(commits 5f54c1a, 07aaa24)。仕様監査で core の spawn 行バグ(row 20 → 正 21)を発見 → Wave 2 で修正。
- **Wave 2 完了・受理**: core = S2 採点・opener・ゴミキュー・spin 分類・FRONT 修正・U7 derive_seeds・7-bag MINSTD(dd20cda, 08e883e、151 tests、fixture 18/18 + RNG 4/4)。bot = exact movegen(f0376d6〜b3b33eb、spin 分類を core に一本化、workspace 168 tests 緑)。仕様 v1.3.4(U1〜U9 決定・監査記録・変異テスト要求)。**M1 の残ピースはレフェリー(crates/arena)のみ**。
- **Wave 3 完了・受理(main に統合済み)**: harness-dev = S2 レフェリー + TBP host + M0 ゲート(2dad514, 4460ab4)。**M0 ゲート達成**: 現行 attax(7b38f51)と 10 ゲーム(seeds 1001–1005 × 両シート)、replay 検証 10/10、到達可能性拒否 0、first-choice 両者 1.00、監査ログ完備(エビデンス scripts/evidence/m0/)。勝敗 0–10 は参考値(intetrigence 側は暫定 BaselineBot)。s2-spec-lead = protocol v1.2 + spec v1.3.6、arena/tbp 監査 19/19 適合、per-player bag を確定(docs/arena.md §3 D1)。bot-dev = M2 前半完了(anytime 探索: beam + hold + bag 予測 + transposition グラフ + T-slot cutout、100ms 予算内、単独 399 ライン完走、workspace 196 tests 緑)+ レフェリー接続用 cell-set adapter。**統合**: bot/harness 両ブランチを main に merge(acdfaea, d04e84a)。
- **Wave 4(次)**: ①統合状態の独立検証(s2-spec-lead、cargo test --workspace + M0 エビデンスの verify 再実行)②本物の bot を Agent trait に接続して tuning seeds で現行 attax と参考マッチ(M2 ゲート: 5 割超)③M3(SPSA チューニング、versus 項)。replay header の rules は @v1.3.5+commit 表記に更新予定。
