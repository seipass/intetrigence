# intetrigence 計画 — CC2 100% → clys 100% → attax 95%

## 1. 現行 goal(2026-11-15 改訂)

> **goal文**
>
> Intetrigence を強化する。段階ゲート:
> 1. **Cold Clear 2 に 50 戦で勝率 100%**
> 2. 通過したら **clys に 50 戦で勝率 100%**
> 3. 通過したら **attax に 50 戦で勝率 95%**
>
> 勝ちは **TopOut 決着のみ**。TurnCap(ターン上限)は未決着で勝者を決めない。forfeit は no contest(勝ちにも負けにも数えない)。
> 各段階 50 ゲーム = 事前確定 seeds 25 個 × 両シート。全ゲーム replay 検証 + 監査ログ + 相手の decision log が必須。
> 評価 seeds は実行前にコミットで確定し、チューニングに使わない。再挑戦はラウンド全体のやり直し。
>
> コンテキストを失った場合は、まず本書(PLAN.md)と docs/s2-spec.md(S2 ルール仕様)、docs/s2-referee-protocol.md(計測プロトコル)を読んでから再開すること。

## 2. 現状(2026-11-15 時点の実測)

| 相手 | 条件 | TopOut 成績 | 目標 |
|---|---|---|---|
| **Cold Clear 2** | 10 ゲーム(seeds 9001–9010, sheet A) | **7 勝 3 敗**(70%) | **50 勝 0 敗** |
| **clys** | 10 ゲーム(同条件) | **2 勝 8 敗**(20%) | **50 勝 0 敗** |
| **attax**(40 行パッチ + spawn 修正) | tuning 20 ゲーム | **2 勝 16 敗**(11.1%) | **48 勝以上 / 50** |

**接続は 3 相手とも健全**: forfeit 0 / no-contest 0 / 未決着 0、replay 10/10 検証(CC2・clys の 10 ゲームスモーク)。**課題は純粋に強度**。

**180° の有無**: attax・CC2・clys のいずれも 180° 回転を持たない(`supports_180 = false`)。ただし 180° は forfeit の原因ではなかった(board desync が原因)。

**旧「94%」「95%」は forfeit を勝ちに数えた無効な数字**(5 つの計測バグ。docs/seed-registry.md §5、docs/s2-rule-audit.md)。

## 3. 判明している敗因(attax 戦)

`docs/wave9-loss-analysis.md`(bot-dev、10 ゲーム 3 勝 7 敗の分析):

| 指標 | intetrigence | attax |
|---|---|---|
| 攻撃合計 | 2,633 | 2,710(**3% 差**) |
| **spin からの攻撃** | **489** | **737**(+51%) |
| full TSS / **TSD** | 30 / **40** | 60 / **100** |
| mini 1 ライン | **109** | 60 |
| max B2B(中央値) | 7 | 5 |
| surge 放出 | 104 | 41 |

**攻撃量は互角。差は T-spin の質**(full spin 160 対 70、TSD 100 対 40)。spin 総数は同じ 300 対 298 なので、**我々が T を mini に変換している**。

## 4. 判明している計測ギャップ

1. **attax の movegen に 180° 回転がない**(`src/ai/bot/movegen.ts` に "180" が 0 回)。我々のレフェリーは 180° を含む movegen で合法性を判定するため、**180° でしか到達できない配置を「合法手あり」と判定 → forfeit-other**(40 行パッチ後も 2/20 = 10%)
2. attax の anytime 探索は非決定論的(我々の bot も)。同じ seed の再実行で結果が変わる
3. 段階 1(CC2)は `wins_topout == 50` を要求。**forfeit 0 が前提**

## 5. アーキテクチャ(変更なし)

```
crates/
  core/   # 純粋 S2 ルールエンジン
  bot/    # exact movegen / anytime 探索 / S2 評価関数 / 重み
  arena/  # S2 レフェリー / replay 検証 / 監査 / ラウンドランナー / 動画レンダラ
  tbp/    # TBP メッセージ I/F と host
adapters/ # attax / CC2 / clys の互換パッチとアダプタ
docs/     # 仕様、プロトコル、監査、計測レポート
scripts/  # ベンチ、スモーク、ラウンド実行、動画生成
```

---

*(以下は旧 goal 時代の記録。経緯として保持)*

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
- **Wave 4(進行中)**: 統合状態の独立検証は完了(s2-spec-lead が cargo test --workspace **245 tests 緑** + M0 replay verify **10/10** を実施、監査 5 項目のゲーム単位記録を確認)。仕様 v1.3.9 / protocol v1.7、両ブランチの全コミットを main に統合(HEAD 6da4af5)。**M1 閉じた**。残る M1 後段 = movegen 一本化 + 本物の bot 接続(担当: harness-dev、bot-dev 支持)→ **M2 ゲート: tuning seeds で現行 attax と参考マッチ、5 割超**。その後 M3(SPSA、versus 項)。レポート必須 3 点(スピン判定の寛容さ明記、採点差は spin 分類のみ、200ms 超過 0 件)は protocol v1.7 に記録済み。
- **Wave 4 完了・M2 ゲート達成(main に統合済み)**: movegen 一本化(クロスチェック恒久化、0 mismatch)+ 本物の Searcher を Agent に接続(f1c2e79)。**M2 ゲートマッチ: tuning seeds 9001–9010 × 両シート = 20 ゲーム vs 現行 attax(7b38f51)、18 勝 2 敗 = 90.0%**(Wilson 95% CI 69.9–97.2%)、replay 検証 20/20、intetrigence の forfeit 0、200ms 超 0。think median: intetrigence 97.1ms / attax 29.1ms。**attack/piece 0.986 vs 0.866、受信ゴミ 75.7 vs 110.0/ゲーム**(攻撃優位は実データで裏付け)。**正直な解釈: 18 勝はすべて attax の 20 行視野による forfeit。40 行トップアウトまで到達した 2 ゲームは 0 勝 2 敗(いずれも終盤の downstack 失敗・埋没)**。フル視野の相手(attax S2 版)への強さは未測定。エビデンス scripts/evidence/m2-tuning/ + docs/m2-tuning-report.md。
- **Wave 5(進行中)**: ①M3 = 防御・downstack の改善(2 敗の敗因)+ versus 項 + SPSA チューニング(bot-dev、実行中)②**forfeit 取扱いを protocol v1.8 §7.1 で決定済み**: forfeit は勝ちとして数えるが `reason`(TopOut/Forfeit/TurnCap)+ `forfeit_cause`(vision/other/unknown)の記録が必須、**レポートは勝ちの種類別に内訳を出す**。vision forfeit 勝ちが 1 件でもあれば「フル視野相手の勝率」を主張しない。**M2 の 90% は 20 行視野を突いた結果(TopOut 決着は 0–2)であり、フル視野の相手への勝ち越し根拠は現時点で無い**。③エビデンスの notes ハードコード修正 + forfeit_cause/visible_rows 実装(harness-dev、実行中)。**未解決: attax S2 版の視野(40 行か 20 行か)は公開時に確認** — 主張の形が決まる。ゲート: tuning seeds で ≥85% + TopOut 決着ゲームの改善。その後 M4(held-out 予備)→ M5(最終 100 ゲーム、attax S2 版)。
- **Wave 5 追加成果(M3 ゲート達成・統合済み)**: ①**M3 完了**(bot-dev a723bf3, 363f27d): 敗因分析(両敗ともゴミスパイク後のトップアウト、盤底ゴミの上に積む形)+ versus 項(ゴミ圧力・downstack)+ SPSA チューナ。**ゲート再検証で両敗 seed が勝利に転換**(9006-A: 111 turn 敗→859 turn 勝・受信ゴミ 247 行で生存 7.7 倍、9010-A: 188 turn 敗→140 turn 勝)。②**defense-stress 診断プロファイル**(harness-dev c1535af, c603600、protocol v1.11 §1.3 の規律: official のみ主張に使用): 12 行 20–0(簡単すぎ)、**16 行 18–2(TopOut 負け 2 = 防御の診断信号)**。③**garbage-pressure プロファイルの実装を GO**(harness-dev 実行中)— 一定 turn ごとのゴミ注入で防御を直接 stress、最終マッチ(フル視野の長期戦)に向けた最重要弱点への投資。④forfeit 会計(reason + forfeit_cause + visible_rows、勝ち内訳)実装済み、M2 エビデンス再生成済み。
- **Wave 5 完了・M4 実行中**: ①防御診断が一通り揃った: defense-stress:16 post-M3 は 19–1(非 forfeit 3/4、ただし n=4 で構成が入れ替わり改善断定は保留)、**自己対戦 garbage-pressure で生存境界を定量化**(0.25 行/turn は持続 9/10 cap、**0.5 行/turn が境界**(9/20 cap・平均 220 turn)、1.0 は超過)。合成プロファイル(start 12 + 0.5 行/turn 相当)と公開 API(Engine::with_profile / engine_from_header)も実装。bot-dev は根拠のない項変更を自ら revert(規律維持)。②**M4 を harness-dev にアサイン(実行中)**: official プロファイルで **reserve seeds 10001–10020 × 両シート = 40 ゲーム** vs 現行 attax。過学習検出が目的(勝率 ≥85% は forfeit 込みのゲート、主張ではない)。指標の一貫性(attack/piece、受信ゴミ、think time、敗因パターン)と勝ち内訳を検証。**最終マッチ用 seeds は 20001–20050 に確定**(未使用を厳守)。③M5 は attax S2 版の公開待ち(視野 40 行か 20 行かが主張の形を決める — protocol v1.8 の差し替え条件どおり)。
- **M4 ゲート達成(9d9b83f、main 統合済み)**: official、reserve seeds 10001–10020 × 両シート = 40 ゲーム vs 現行 attax。**38 勝 2 敗 = 95.0%**(Wilson 83.5–98.6)≥85% ゲート通過。replay 40/40、intetrigence forfeit 0。勝ち内訳: **vision-forfeit 34 / turn-cap 4 / TopOut 0**。負け 2(10001-B、10005-B)= 既知の「終盤 downstack 失敗」のみで**新しい敗因パターンなし**。attack/piece 0.977 vs 0.885、受信 113 vs 149、think 97.1/98.7ms、採点不一致 0/16486。**M3 tuning と指標が一貫 = 過学習の兆候なし**。エビデンス scripts/evidence/m4-reserve/、docs/m4-reserve-report.md。
- **Wave 6(進行中)**: **最重要の洞察: attax S2 版がフル視野なら vision-forfeit が消え、勝敗は非 forfeit ゲームで決まる — 現在の非 forfeit 成績は 4/6(66.7%)**。よって待機時間の最高価値投資は防御の追加改善。①bot-dev = M4 の 2 敗(10001-B / 10005-B)+ 合成プロファイル診断で防御改善の第 2 イテレーション → 改善後は**新鲜 seeds 10021–10030 で再検証**(10001–10020 は改善に使った seeds なので再利用は循環になる)。**最終 seeds 20001–20050 は絶対未使用**。②M5 の準備(harness-dev): attax S2 版公開時に接続・視野確認・最終マッチ実行。③レポート要件は protocol v1.8–v1.12 に確定済み(勝ち内訳・非 forfeit 別掲・スピン寛容さ明記)。
- **防御 v2 の評価(6f4ec7f、main 統合済み)**: 相殺伝播(攻撃が pending ゴミを減らすのを探索が見えるように修正 — 仕様正当な機構修正)。M4 同一 seeds 再実行: **37–3 = 92.5%**(Wilson 80.1–97.4)、受信ゴミ **113 → 97(−14%)**、10001-B は敗→勝。ただし **非ターゲット 38 ゲームで pre-v2 38–0 → v2 36–2(10003-A、10018-A が新規敗北)**、10005-B は未解決(389→551 turn に延命のみ)。**判断: v2 は main に残す(機構は仕様正当・統計的に区別不能)が「強度向上の証拠」としては不成立と記録**。
- **Wave 7 完了(2dca3df、da6550e で統合): 高さペナルティ 3 変種すべて棄却、重みは防御 v2 で凍結**。bot-dev が 3 試行(DANGER_BASE=10、downstack 報酬強化、flatness = max_height − median_height)を診断プロキシで検証 — いずれも**高さを −5〜7% 下げる代わりに attack/piece を −3% 落とす同一のトレードオフ**で、fresh seeds での勝率を下げるため不採用。結論: **残る差は積み方そのもの(attax は平積みで攻撃、我々は塔を建てて攻撃)で、単純な高さ罰では解けない**。docs/m4-stack-management-report.md に 3 試行の全記録を残し、M5 で 90% を割った場合の再設計(列高さ分布の特徴量等)の出発点とする。**M5 は防御 v2 の重みで臨むことを確定**(M4 reserve 92.5% ≥ 90% 目標、非 forfeit 5/8 は既知のリスク)。
- **準備完了・M5 は attax S2 版の公開待ち**: 実行条件はすべて揃った — レフェリー(replay 検証・監査・forfeit 会計)、bot(防御 v2 凍結、--ms 100 内)、プロトコル(v1.12: 勝ち内訳・主張制限・レポート要件)、最終 seeds **20001–20050(絶対未使用)**。公開されたら: attax-integration が S2 版のコミット固定と視野(40 行/20 行)を確認 → harness-dev が公開 I/F で接続 → 100 ゲーム実行 → s2-spec-lead が主張の形を確定(protocol §7.1 の差し替え条件どおり)。
- **2026-11-14 ユーザー判断: 暫定 100 ゲームを実施(現行 attax 相手)**。attax S2 版は上流未公開のため、**最終 seeds 20001–20050 は温存**し、**未使用の新規 seeds 11001–11050 × 両シート = 100 ゲーム**(両プロジェクトの使用範囲外: 我々 1001–1005 / 9001–9020 / 10001–1040、attax 1001–1200 / 2001–2060 / 5001–5050 / 7001–7050)で現行 attax(7b38f51、guideline 採点・20 行視野)との参考マッチを走らせる。**公式の 90% 主張は引き続き S2 版との最終マッチで行う**(protocol §7.1.3 のとおり、vision-forfeit を分離して報告)。エビデンス: scripts/evidence/m5-provisional/、レポート: docs/m5-provisional-report.md。担当: harness-dev(実行)、bot-dev(事後分析)。
- **暫定 100 ゲームマッチ結果(c265fa3、main 統合済み)**: **94 勝 6 敗 = 94.0%**(Wilson 87.5–97.2)、replay 検証 100/100、intetrigence forfeit 0。勝ち内訳: **vision-forfeit 91 / turn-cap 3 / TopOut 0**。**非 forfeit は 3/9 = 33.3%**(M4 の 5/8 より低いが n=9 で CI は重複、方向は既知の弱点と一致)。負け 6 = TopOut(downstack 5 + **opener 1 = 新パターン**: 11027-A が 37 turn で死亡、attack/piece 0.595)。attack/piece 0.965 vs 0.862、受信 85.0 vs 117.1、think 97.0/98.8ms(200ms 超 1 手、§8.4 どおり報告)。エビデンス scripts/evidence/m5-provisional/、docs/m5-provisional-report.md。**解釈: 94.0% は 91 の vision-forfeit に支えられた参考値。公式の 90% 主張は attax S2 版・最終 seeds 20001–20050 で行う**。新課題: opener 死(37 turn)の原因調査を bot-dev が事後分析中。
- **2026-11-14 ユーザー判断: 待機を解除してプロジェクトを停止**。「S2 版の公開をここから待っても早くならない」ため、block 状態でチーム全員アイドルのまま停止する。**再開手順(attax S2 版が公開されたら)**: ①attax-integration が S2 コミットを固定・視野(40/20 行)を確認 → ②harness-dev が `scripts/m5-final.sh` を seeds 20001–20050 × 両シートで実行(約 2 時間、replay 検証・監査・レポートまで自動)→ ③s2-spec-lead が protocol §7.1 の差し替え条件どおり主張を確定。**現状の成果**: S2 エンジン・探索 bot(防御 v2)・レフェリー・監査・診断プロファイル・プロトコル v1.12 がすべて main に統合済み。暫定マッチ(vs 現行 attax)94–6 = 94.0%(91 は vision-forfeit、非 forfeit 3/9)。最終 seeds 20001–20050 は未使用。
- **2026-11-14 ユーザー依頼(新しいタスク): リプレイ → 動画レンダラ + intetrigence vs attax の 10 本の動画を作成**。①replay JSONL から再シミュレーションし動画(MP4)を生成する機能(crates/arena または tools/、core で再シミュレート + 盤面ハッシュ検証で動画の正確性を保証、ffmpeg でエンコード)②暫定マッチ(scripts/evidence/m5-provisional/)から 10 ゲームを選んで動画化 — 非 forfeit 9 ゲーム(3 turn-cap 勝ち + 6 TopOut 負け = 本番の激戦)+ 1 短い forfeit 勝ち。動画は videos/(gitignore)、レンダラとマニフェストをコミット。担当: harness-dev。
- **2026-11-14 動画レンダラ + 10 本の動画が完成(harness-dev、4981aee、main af191d5 で統合)**: `crates/arena/src/render.rs`(5×7 ビットマップフォント内蔵、ffmpeg libx264 パイプエンコード)。**描画前に verify_replay + 毎ターンの盤面ハッシュ照合で動画とリプレイの一致を保証**(検証失敗 replay は拒否)。左右の盤面(ピース色つき)、落下アニメーション(5 PPS 実時間)+ ghost、hold / next 5 / pending garbage バー / B2B / combo / attack / think ms、エンドカード(勝者 + 決着理由)。CLI: `arena render --replay <jsonl> --out <mp4>`。**10 本の動画を videos/ に生成**(非 forfeit 9 = turn-cap 勝ち 3 + TopOut 負け 6、+ forfeit-vision 勝ち 1、計 39MB、全 10 本 verified: true、manifest.json コミット済み)。再レンダリングは `scripts/render-m5-videos.sh`(約 6 分)。cargo test 253 緑。
- **2026-11-14 goal 改訂(旧 90% goal は終了・新 goal を §1 に移行)**: attax への勝率を**100%**(50 ゲーム)に引き上げ、**CC2 / clys への勝率 95%**(各 50 ゲーム)を汎化検証として追加。**シード規則(特定シード値への最適化の禁止)を導入**: 評価 seeds の事前確定・チューニングとの完全分離・焼き付き・再挑戦はラウンド全体のやり直し。**Seed レジストリ**: 焼き付き済み = 1001–1005(M0)/ 9001–9020(tuning)/ 10001–1040(M4 系)/ 11001–11050(暫定マッチ)、未使用 = 20001–20050(旧 M5 予約・新 goal では未使用のまま)。**評価 seeds は各ラウンド前に新規範囲をコミットで確定する**(例: ラウンド 1 = 30001–30025)。
- **Wave 8(進行中・新 goal の実装)**: ①bot-dev = **スタック管理の本格再設計**(attax 100% の鍵。6 敗 = downstack 5 + opener 死 1 を潰す。docs/m4-stack-management-report.md の 3 棄却変種を踏まえ、列高さ分布の特徴量等の構造的アプローチ。tuning seeds + 診断プロファイルのみで調整)②attax-integration = **CC2 + clys の統合**(CC2 は TBP ネイティブ、clys は公開 I/F を調査して完璧なアダプタを作成。座標変換はセル集合一致で検証、attax 接続と同等の監査)③s2-spec-lead = ラウンドプロトコルの成文化(段階ゲート・評価 seeds の事前確定手続き・焼き付き台帳)④harness-dev = ラウンドランナー(attax 50 → CC2 50 → clys 50 の段階実行、同一ビルド・同一 seeds、replay 検証・監査・レポート)。**全員の準備が揃ったら評価 seeds をコミット確定 → ラウンド 1 を実行**。
- **🚨 2026-11-14 重大な測定無効の判明(ユーザー指摘 + manager が証拠を直接集計)**: **すべての対 attax の数字は無効。** 全エビデンス 220 ゲームの内訳は **TopOut 勝ち 0 / TopOut 負け 13 / TurnCap 勝ち 13 / forfeit-vision 194**。つまり **実際のゲーム勝率は 13/220 = 5.9%** であり、**intetrigence は一度も attax をトップアウトさせていない**。「M4 95%」「暫定 94%」は attax の TBP アダプタが 194 回死んだことを測っていた。
  - **分類バグ**: `crates/arena/src/referee.rs:656` の `classify_forfeit` が `lowest_cell >= visible_rows` を最初に判定するが、S2 のスポーンは row 21・attax の visible_rows は 20 なので**常に真**。vision 判定が無条件に付き、真の原因(アダプタ故障)を隠していた。seed 11012 turn 32(盤面は低い)が vision になったのはこれ。
  - **条件の非対称**: 審判は自前 S2 実装、intetrigence は 40 行可視・ネイティブ 8 スレッド、attax は 20 行可視・TBP アダプタ経由。
  - **是正措置(全員に発注済み)**: ①attax-integration = 194 forfeit の根本原因特定(seed 11012 turn 32 を起点)+ attax の 40 行対応パッチ ②harness-dev = 分類バグ修正 + **ゲートを TopOut + TurnCap のみに変更** + 既存エビデンスの再分類 ③bot-dev = **TopOut 勝ち 0/220 の克服**(アダプタ修正後の実ベースライン測定から)④s2-spec-lead = protocol v1.17(ゲート定義・分類仕様・無効化された測定の記録)。
  - **新しい合格基準の方向性**: 勝数は TopOut + TurnCap のみで数える。forfeit はゲート外(別途報告)。既存の 194 forfeit を含む過去の数字は「無効化された測定」として docs に記録する。

- **🔧 2026-11-15 測定基盤の修正完了(全エビデンスの再解釈)**:
  - **attax の視野**: upstream `7b38f51` は **20 行**(`git show 7b38f51:src/ai/bot/movegen.ts` → `ROWS = 20`)。作業ツリーが 40 行に見えたのは attax-integration のパッチ適用済みだったため(3 名 + manager が独立確認)。**40 行互換パッチ + posY 修正を採用**。
  - **posY バグ**: attax アダプタが active をグリッド最上段に置いていた(レフェリーの S2 spawn と 18 行ずれ)→ `nodes=0` で即 forfeit。**修正後 seed 11012 は 1000 turn 完走、forfeit 0**。
  - **分類バグ**: `classify_forfeit` の `lowest_cell >= visible_rows` が常に真(S2 spawn row 21 / V=20)。削除済み。判定は「可視行に収まる合法手があるか」のみ。
  - **旧エビデンス(V=20 で再分類)**: vision 32 / **other 162**(104 高スタック + 58 低スタック)/ TopOut 勝 0 / TurnCap 勝 13 / TopOut 負 13 → **ゲート勝数 13/220 = 5.9%**。**すべて無効**として seed-registry §5 に記録。
  - **パッチ適用 attax のスモーク**: seed 11012 A/B = **attax の 2-0**(TopOut 464/246 turn)、forfeit 0、replay 2/2。**forfeit が消えた瞬間、attax が勝つ** — これが真の実力差。
  - **計測基盤の整備完了**: `visible_rows` の自動検出(movegen.ts から)、`--weights key=value` による重み注入(replay header に記録)、ゲートは TopOut+TurnCap のみ、測定完全性チェック(replay+verify+audit+相手 decision log)、surge/B2B 指標。
  - **次の焦点**: bot-dev の surge 改善。surge は放出されている(1,602 回/372 ゲーム)が **80% が 4–7 ライン**で attax に吸収される。効くのは 9–13 ライン。charge の非線形価値 + hold ペナルティを self-play で A/B 中。

- **🚫 2026-11-15 ルール変更(ユーザー指示): TurnCap を勝ちに数えない**。1000 turn 到達で attack_total の差により勝者を決めるのは実力の判定として不適切。**勝ちは TopOut のみ**。TurnCap は「未決着」として勝ちにも負けにも数えず、**1 件でも出ればゲート不通過**(50 ゲームすべてで TopOut 決着が必要)。
  - **attax-40 スモークの訂正**: 「8 勝 2 敗」は誤り。正しくは **TopOut 勝ち 6 / TopOut 負け 2 / 未決着 2**。決着ゲームの勝率 6/8 = 75%、全 10 ゲームでは 6/10。
  - TopOut 勝ち: 11001-B(804), 11002-B(411), 11003-A(351), 11003-B(723), 11004-B(938), 11005-B(322)
  - TopOut 負け: 11004-A(724), 11005-A(191)
  - 未決着(TurnCap 1000): 11001-A, 11002-A
  - ゲート定義: 段階 1 = `wins_topout >= 50 && losses_topout == 0 && turncap == 0`、段階 2・3 = `wins_topout >= 48 && turncap == 0`
- **🚫 2026-11-15 追加(ユーザー指示): 時間切れで攻撃力が高い方を勝ちにしない**。TETR.IO にそのようなルールは存在しない。**TurnCap は勝者を決めない**(`winner: None`)。
  - `crates/arena/src/referee.rs` の TurnCap 判定から `received_total` / `sent_total` の比較を削除。
  - `crates/arena/src/replay.rs` の verify 側にも同じ比較が複製されていたため削除(記録と再計算が食い違う原因になっていた)。
  - ゲートは `turncap == 0` を要求(1 件でも未決着があれば不通過)。

- **🔍 2026-11-15 S2 ルール監査(ユーザー指示)**: 実装と文書の全規則を [S2 準拠] / [我々のプロトコル] / [誤り] に 3 分類。**docs/s2-rule-audit.md**(レフェリー・文書)と **docs/core-rule-audit.md**(core)を作成。
  - **勝者決定の経路は 4 つだけ**(`referee.rs` の `match &end`)。W1 `TopOut` = S2 準拠 / W2 `Stuck` = 誤り / W3 `TurnCap` = 修正済み / W4 `Forfeit` = 修正(no contest)
  - **修正した誤り**: ①TurnCap の攻撃量勝者決定(referee + verify の 2 箇所)②`Stuck` を `TopOut` と記録 ③`Forfeit` で相手を勝者に ④`draws` が未決着と引き分けを混同 ⑤`wins_forfeit_other` が `cause == None` も数える ⑥ゲートが存在しない `wins_turncap` を参照 ⑦**0 ライン設置に直前 combo の `ln` 攻撃が乗る**(実バグ)⑧protocol §6.3 に攻撃量順位付けの記述が残存
  - **ゲートを goal に合わせて厳格化**: 段階 1 = `wins_topout == 50 && losses_topout == 0 && turncap == 0 && no_contest == 0`(完全試合)、段階 2・3 = `wins_topout >= 48 && turncap == 0`。**5 項目の合計 == 50 の検算を必須化**
  - **真の実力が判明**: 40 行パッチ + spawn 修正済み attax に対して **TopOut 勝ち 2/18 = 11.1%**(tuning 20 ゲーム)、スモーク 10 ゲームは **0 勝 9 敗 1 未決着**。旧「94%」「95%」「8 勝 2 敗」はすべて無効
  - **残る計測ギャップ**: 40 行パッチ後も forfeit-other が 2/20(10%)。`no_contest == 0` がゲート条件なので公式ラウンド前に desync 修正が必要(attax-integration が board dump 診断で対応中)

## 6. 2026-11-15 の重大発見: 探索が動いていなかった

bot-dev のプロファイル(`91ec267`)で、**anytime ループが期限超過で反復を捨てて `None` を返し、呼び出し側が 1-ply greedy にフォールバックしていた**ことが判明。

| 設定 | depth | width | nodes | 実際の手 |
|---|---|---|---|---|
| baseline | 8 | **1** | **1** | **1-ply greedy** |
| deep(12/16) | 12/16 | 1 | 1 | greedy |
| wide(max 2048) | 8 | 1 | 1 | greedy |
| 1 スレッド | 8 | 1 | 1 | greedy |

毎手 ~105ms を消費しながら、**探索は 1 ノードも走っていなかった**。

### 含意

**これまでの全対戦成績は「1-ply greedy のボット」の数字**であり、ビーム探索の実力ではありません。

| 測定 | 数字 | 但し書き |
|---|---|---|
| CC2 ベースライン(bot-dev) | 16-4(80%) | 1-ply greedy |
| CC2 スモーク(harness-dev) | 10-0(100%) | 1-ply greedy |
| clys | 2-8(20%) | 1-ply greedy |
| attax(40 行パッチ) | 2-16(11.1%) | 1-ply greedy |

**clys(depth 32)に 20% しか勝てない理由、attax に 11.1% の理由は、評価関数の弱さではなく「探索していないこと」**で説明できる。

### 仕様側の対応(protocol v1.31 §8.4.1)

`think_ms` は「探索した」ことを意味しない。レポートに次を必須化:
- `think_ms`(median / p95 / p99 / max)
- **完了した探索の深さ・幅・ノード数**
- **1-ply フォールバックに落ちた手数と割合**

### 評価 seeds の扱い

**探索修正が完了するまでラウンドを開始しない。** 1-ply greedy の数字で評価 seeds 25 個を焼くと、規則(使ったら焼き付き)により無駄になる。

### 探索修正の結果(`00d1a5e` / `9fee983`)

修正は成功: **greedy フォールバック 0 回**、合成盤面で depth 8 / width 324(1 スレッドで width 96)。

**しかし CC2 戦は悪化した。**

| ビルド | ゲーム | 勝 | 敗 | attack/piece | quad | single |
|---|---|---|---|---|---|---|
| 1-ply greedy(修正前) | 20 | **16** | 4 | 0.811 | 170 | 355 |
| 実探索・既定重み | 20 | **7** | 13 | 0.722 | 113 | 466 |
| 実探索・attack=28 | 10 | 7 | 3 | 0.723 | 42 | 218 |
| 実探索・attack=28, holes=−18 | 10 | 6 | 4 | 0.753 | 52 | 180 |

**診断**: depth 8 では「ライン消しは表面を凸凹にする」ことが見えるため、`holes=−30` / `height_sq=−12` が `attack=14` を上回り、**攻撃せず平積みするボット**に変わる。1-ply はこのトレードオフを見ないので、結果的に攻撃的で勝てていた。**重みは「探索が動かない前提」で校正されていた。**

### 未解決の緊張

- 損失分析(`wave9-loss-analysis.md`): 攻撃量はほぼ同等、**死ぬのは我々**(高さ 20–23 vs 10–19)→ 守備が問題
- bot-dev の測定: **守備的にすると CC2 に負ける** → 攻撃が問題

**両立する説明**: 評価関数が「常に平らに」を指向しているため、攻撃も守備も中途半端になる。`height_sq` が盤面全体に常時効くのが原因。

**方針**: danger 項を非線形化(高さ ~12 以下ではほぼゼロ、超えたら急峻)。これで「普段は攻撃、危なくなったら降ろす」を表現する。

### 実戦の探索深度は 2 だった(`1c682e1`)

bot-dev が `search_distribution.rs` で実ゲーム 400 turn を測定:

| 区間 | 手数 | depth | width | nodes | iters |
|---|---|---|---|---|---|
| turn 1–50 | 50 | **1.9** | 352 | 43 | 4.0 |
| turn 51–100 | 50 | 1.9 | 391 | 47 | 4.3 |
| turn 101–150 | 50 | 1.8 | 406 | 44 | 4.4 |
| turn 151–200 | 50 | 1.9 | 460 | 39 | 4.7 |
| turn 201+ | 200 | **2.2** | 695 | 227 | 6.7 |

fallback 0/400。**合成盤面の depth 8 は実戦では一度も起きていない。** width の拡張(width 96→768)が予算を使い切り、深さに到達していなかった。

深さ優先に変更後: depth **3.5**、width 96、iters 2.0。100ms/8t で width 96 の反復が 2 回しか入らない。**depth 8 には 4 倍の速度が必要。**

### 未解明の疑問

`nodes 43` は depth 2 / width 352 の期待値(数百)と桁が合わない。100ms / 45 nodes = **2.2ms/node** は、ビットボード評価としては 100 倍遅い。**時間が探索以外(スレッド生成・同期・盤面コピー・ソート)で消えている疑い**。bot-dev が時間内訳を測定中。

### 重要な事実

**1-ply greedy が depth 2–3.5 の探索に 16-4 vs 7-13 で勝っている。** 現状の探索は貪欲法より弱い。これは「浅いから」ではなく「評価関数が探索の horizon で誤った方向を向くから」。**depth を上げる前に、depth 2–3 で greedy に勝てる評価にする必要がある。**

## 7. 攻撃の成分分解: 真の穴はコンボ(s2-spec-lead `aa035d8`)

406 ゲーム(attax 相手)の replay から §6.8 の式を再実装して攻撃を分解。`lines > 0` の **243,484 設置は完全一致**(不一致はすべて `lines == 0` の F1 バグ由来)。

### S2 ボーナスは attax より多く発火している(仮説は否定)

| 指標 | intetrigence | attax | 比 |
|---|---|---|---|
| attack/piece | **0.878** | 0.790 | 1.11x |
| surge の寄与 | **8.6%** | 1.8% | **4.8x** |
| B2B flat +1 の寄与 | **12.9%** | 7.3% | **1.8x** |
| surge 発火回数 | **1,657** | 379 | **4.4x** |

### 本当の穴: コンボ

`combo_before` の分布:

| combo | intetrigence | attax |
|---|---|---|
| **0** | **53.8%** | 46.9% |
| 3–4 | 7.4% | 13.4% |
| 5–9 | 3.0% | 4.3% |
| 10+ | 0.04% | 0.04% |

### 数式の検算(マネージャが実施)

`combo >= 2` で `base = max(base, ln(1.25*combo + 1))`。**single でも ln 項が効く**:

| combo | single が送る攻撃 |
|---|---|
| 1 | 0 |
| 2–5 | 1 |
| 6–15 | 2 |
| 16+ | 3 |

**コンボ 1..20 を single で繋ぐと 39 attack / 20 ピース = 1.95 attack/piece。** 我々の実測 0.878 → **2.2 倍の伸びしろ**。

### なぜ今それが取れていないか

**探索の地平が短いから。** depth 4 では「combo 0 の single は 0 attack」しか見えず、コンボの価値(5 手先)が見えない。だから探索はコンボを切る手を選ぶ。

**コンボは「探索が発見する」ものではなく、評価関数に明示的な項として入れる必要がある。**

### 方針

評価関数に `combo_state`(現在の combo の価値)と `combo_potential`(コンボを継続できる盤面)を追加。A/B/C の 3 構成で 20 ゲームずつ CC2 と比較。

**受け入れ基準**: `attack/piece >= 1.2`、combo 3+ の設置が 15% 以上、CC2 に 16 勝以上。

## 8. コンボ戦略の実測(s2-spec-lead `efd38e2`)

### 訂正: 理論上限は 1.95

`BASE_ATTACK[none] = [0,0,1,2,4]` — **single の base は 0**。素の single の攻撃は combo の ln 項のみ:

| combo | single が送る攻撃 |
|---|---|
| 1 | 0 |
| 2–5 | 1 |
| 6–15 | 2 |
| 16+ | 3 |

持続 single コンボの上限 = **1.95 attack/piece**(39 attack / 20 ピース)。s2-spec-lead が最初に出した 3.14 / 4.39 は「single base = 1」で計算した誤りで、撤回済み。

### 実測は「長いコンボ」を支持しない

406 ゲームの全設置を combo 帯ごとに集計:

| combo | 設置数 | attack/piece | lines/piece |
|---|---|---|---|
| **0** | 173,756 | **0.695** | 0.565 |
| **1–2** | 68,753 | **1.073** | 0.873 |
| **3–4** | 16,509 | **1.196** | 0.878 |
| **5–9** | 6,835 | 1.084 | **0.670** |
| 10+ | 82 | 1.110 | 0.488 |

**combo 3–4 がピーク。combo 5 以上は改善しない**(lines/piece が 0.670 に落ちる = single で繋いでいる)。

**正解は「combo 2–4 を保ちながら高い base(TSD 4 / quad 4)を撃つ」。** 「コンボを伸ばす」ではない。

### 盤面形状の比較

| 形状 | 1 ライン/ピース維持 | ゴミ耐性 | 判定 |
|---|---|---|---|
| 4-wide ウェル | **×**(4-wide の 1 行は 4 セル。水平 I だけが完成させる) | 弱 | 不採用 |
| 1-wide ウェル | ×(縦 I が 4 行を一気に完成) | 最弱 | 不採用 |
| 平坦 + single | ×(4 セル < 10 セル) | 中 | 不採用 |
| 欠損 3 セルの段 | ○ 条件付き(7 種中 3 種) | 中 | 有力 |
| **TSD 主体 + 短いコンボ** | **○**(TSD は 2 行消すのでゴミを吸収) | **強** | **最有力** |

### ゴミ耐性

| 状況 | 判断 |
|---|---|
| ゴミが出現前 | コンボを続ける(相殺できるなら相殺) |
| ゴミが出現した | **コンボを切ってでもゴミを消す** |
| 4 行以上出現 | コンボを捨てて立て直す |
| combo 5 以上 | 伸ばすより次の消去で高い base を狙う |

### レース計算(CC2 10 + clys 10)

| | 勝った 9 ゲーム | 負けた 11 ゲーム |
|---|---|---|
| 我々 attack/piece | 0.829 | 0.780 |
| 相手 attack/piece | 0.747 | **0.989** |
| 我々の被ゴミ | 34.7 | **67.4** |
| 相手の被ゴミ | 43.4 | 37.2 |

**負ける構造**: 相手が攻撃モードに入ると 0.989(勝つゲームでは 0.747)。我々の被ゴミは 1.8 倍。1.2 にすると 152 turn で +32 行の逆転。

**50-0 には 3 つ必要**: ①1.2 超のレート ②burst 耐性(受けたゴミの即処理)③分散の制御。

### 受け入れ基準

| 指標 | 現状 | 最低 | 目標 |
|---|---|---|---|
| `attack/piece` | 0.878 | **1.10** | **1.20** |
| combo 0 の割合 | 53.8% | 45% | 40% |
| spin 攻撃の寄与 | 31.2% | 33% | 35% |
| 負けゲームの被ゴミ | 67.4 | 55 | 50 |
| CC2 20 ゲーム | 7 勝 | **16 勝** | 18 勝 |

## 9. 現在地とゲートの数学(2026-11-15)

### 実測の推移(CC2、tuning seeds 9001–9010 × 両シート、20 ゲーム)

| ビルド | 勝 | 敗 | a/p | combo 0 | combo 3–4 | quad | single |
|---|---|---|---|---|---|---|---|
| 1-ply greedy(探索バグ時代) | 16 | 4 | 0.811 | — | — | 170 | 355 |
| 実探索・旧既定重み | 7 | 13 | 0.722 | 53.8% | 7.4% | 113 | 466 |
| + 非線形 danger / レース / attack 28 / combo | **14** | **6** | 0.745 | **38.8%** | **14.8%** | 162 | 1423 |

TopOut のみ、forfeit 0、未決着 0、20/20 replay 検証済み。

### ゲートの数学(重要)

段階 1 は `wins_topout == 50 && losses == 0`。**1 ゲームも落とせない**ので、必要なのは「平均勝率」ではなく**ほぼ 100% の per-game 勝率**:

| per-game 勝率 | P(50 連勝) |
|---|---|
| 70%(現在) | 0.00% |
| 90% | 0.52% |
| 95% | 7.7% |
| 98% | 36.4% |
| 99% | 60.5% |
| 99.5% | 77.8% |

**目標は「per-game 99% 以上」= 実質的に「絶対に負けない」。** 70% → 99% は大きな飛躍であり、重みの調整だけでは届かない可能性が高い。構造的な優位(攻撃レート、burst、相手の S2 非対応)を突く必要がある。

### 検証中の仮説

1. **burst 仮説**: 平均レートより「序盤に大ダメージ」が効く。CC2 は S2 のゴミ処理が弱い。ゲームが短くなれば自滅の機会も減る
2. **base 仮説**: single の ln 依存(1〜2)から TSD/quad の base(4)依存へ移す
3. **守備仮説**: 負けゲームは被ゴミが 1.8 倍。burst 耐性が要る

## 10. burst 防御の仕様(s2-spec-lead `501d577`)

### 最重要: 「1 ラインでも消せばゴミは出ない」

```rust
pub fn take_insertable(&mut self, lines_cleared: u8, current_frame: u64) -> Vec<u8> {
    if lines_cleared > 0 { return Vec::new(); }   // ← burst 防御の本体
```

**コンボを持続することと burst を防ぐことは同じ操作。** コンボが切れた瞬間が危険。したがって「コンボ項」と「生存」は対立しない。

### ゴミのタイミング

| turn | 出現 |
|---|---|
| T(送信) | しない |
| T+1 | しない(frame 差 12 < 遅延 20) |
| **T+2** | **出現し得る** |

**猶予 2 turn、1 回の上限 8 行**(`GARBAGE_CAP`)。20 行の burst でも 8 行ずつ刻まれるので、合間に 1 行消せば止まる。

### 死ぬ条件

spawn の最下段セルは**すべて row 21**。**block-out = 高さ ≥ 21**。`insert_garbage` は挿入後に active を再スポーンし、置けなければ**その場で topped_out**。

**生存条件: `H + pending <= 20`。**

### 危険の 3 段階

| 段階 | 条件 | 行動 |
|---|---|---|
| 安全 | `H + pending <= 17` | 攻撃優先 |
| 警告 | 18–20 | 0 ライン設置を避ける |
| **危険** | **`>= 21`** | **1 行消し最優先。コンボは捨ててよい** |

### bot-dev の実装の誤り(修正指示済み)

```rust
incoming >= SURVIVAL_PENDING(8) || height >= SURVIVAL_HEIGHT(15)   // ← OR で早すぎる
```

| 状態 | 現行 | 実際 |
|---|---|---|
| `H=15, pending=0` | 守備 | `15 <= 20` で安全 → 攻撃を無駄に捨てる |
| `H=5, pending=8` | 守備 | `13 <= 20` で安全 → 攻撃を無駄に捨てる |

**正: `max_height + incoming >= 21`。** `DANGER_BASE = 12` も低すぎる(15〜17 を推奨)。12 だと通常プレイで常に罰則が効き、攻撃項を圧迫する。

### 崩壊の定義(測定に追加)

**`最終 y >= 14` かつ `turn < 100`。** 9004-A が該当。**この件数が 0 にならない限り 50-0 は不可能。**

## 11. CC2 18/20 — 残る 2 敗の構造(s2-spec-lead `71fa04d`)

### 負けは接戦ではない

| | n | turn | sent | recv | **net** | a/p | 最終 y | 相手 y |
|---|---|---|---|---|---|---|---|---|
| **勝ち** | 18 | 59–636(平均 250) | 36–343 | 16–186 | **+16〜+157(平均 +58)** | 0.69–0.89 | 2–10 | 16–25 |
| **負け** | 2 | **60 / 109** | **20 / 37** | **31 / 48** | **−11 / −11** | 0.65 / 0.66 | **21 / 20** | **10 / 7** |

**負けの特徴**: ①net damage が両方 −11 ②送った量が勝ち平均の半分以下 ③ゲームが短い ④**相手の盤面が低い(y=10/7)= 一方的** ⑤我々だけが死んでいる。

**「序盤に攻撃できず、burst を受けて自滅」。** 攻撃レースの接戦ではない。

### 決定的な発見: Opener Phase の ×2 相殺

S2 の Opener Phase(設置 1〜14)では `pending >= sent` なら**相殺量が ×2**。

**序盤に 1 ライン消せば 2 行キャンセルできる。** 逆に序盤に消さないと **×2 の恩恵を全部捨てる**。

**要求: 設置 1〜14 個目で最低 2 ライン消す**(2 ライン → 4 行キャンセル)。

### 目標の修正

| 指標 | 現状 | 新しい目標 |
|---|---|---|
| `attack/line` | 1.19 | **1.33〜1.40**(spin 30〜35%) |
| `attack/piece` | 0.761 | **0.85〜0.89** |
| **`net damage`** | 勝ち +58 / 負け −11 | **全ゲームで >= 0** |
| **崩壊件数** | 1/20 | **0/50** |

**`attack/piece` は主指標にしない。** 負け 2 件の a/p は 0.650 / 0.661、勝ち平均 0.77 で大差なし。**判別しているのは `net damage` と最終 y。**

**`a/line` 1.73 は撤回。** `a/line = s × 2.5 + (1-s) × x` で、1.73 には spin ライン比率 40〜50% が必要(トップ人間でも難しい)。**重みでは届かない。**

### TSD が重みで増えない理由

TSD は 2〜3 ピースかけてスロットを作る必要があり、**depth 2〜4 の探索では見えない**。したがって**重みではなく構造の問題**(mini への変換、浅いスロット)。後回し。

## 12. 現在地のまとめ(2026-11-15 時点)

| 相手 | ゲーム | TopOut 勝ち | 率 | ゲート |
|---|---|---|---|---|
| **CC2** | 20 | **18** | **90%** | 50-0 が必要 → 未達 |
| clys | 10 | 1 | 10% | 未着手(CC2 通過後) |
| attax | 18 | 2 | 11.1% | 未着手 |

**探索は健全**: depth median 4 / p95 8 / max 8、fallback 0.4%(CC2)。**接続も健全**: forfeit 0(CC2)、replay 20/20。

**残る課題**: ①崩壊 0 件(Opener Phase の ×2 相殺) ②net damage >= 0 ③TSD の構造的改善 ④clys プロセス死の原因特定。

## 13. 相手によって「効く指標」が違う(s2-spec-lead `c8e6c94`)

| 相手 | 戦績 | net damage | **崩壊** | opener lines | a/p |
|---|---|---|---|---|---|
| **CC2**(20) | **17–3(85%)** | **+34.15** | **1/20(5%)** | 3.65 | 0.719 |
| **clys**(10) | **1–8–1(10%)** | **+17.3** | **3/10(30%)** | 4.3 | 0.751 |

**clys には net damage がプラスなのに 1 勝 8 敗。** CC2 にはもっと大きなプラスで 17 勝 3 敗。

### 解釈: clys の攻撃は「塊」で来る

- **CC2**: 安定した攻撃 → **net damage が勝敗を決める**
- **clys**: T-spin / quad 主体で **1 turn に 5〜7 行**がまとめて来る → **総量は少なくても peak が高い**

**したがって clys 相手では `net damage` は判別指標にならない。** 必要な指標:

| 指標 | 定義 |
|---|---|
| `max_burst_1` | 1 turn に受けた最大行数 |
| `max_burst_5` | 連続 5 turn で受けた最大行数 |
| **`burst_over_capacity`** | `max_burst_5 > 21 - H`(その時点)だった回数 |

### 戦略上の含意: clys が本当の関門

| 段階 | 現状 | 目標 | 距離 |
|---|---|---|---|
| 1. CC2 | 17/20(85%) | 50/50 | 3 敗 + 崩壊 1 |
| **2. clys** | **1/10(10%)** | **50/50** | **8 敗 + 崩壊 3** |
| 3. attax | 2/18(11%) | 48/50 | — |

**clys の 10% は CC2 の 85% より遥かに遠い。** CC2 が 100% になっても段階 2 で止まるので、**burst プロファイルの測定は CC2 の 100% 化と並行して進める**。

## 14. clys プロセス死の扱い(マネージャ判断)

**再起動・リトライはしない。**

- プロセスが死んだ = そのゲームで clys が完走できなかった、という事実
- 再起動すると盤面・RNG・コンボが失われた後の**別ゲーム**になる
- それを「同じゲーム」として数えるのは、**forfeit を勝ちに数えるのと同じ種類の誤り**

**プロトコル**: プロセス死 = `forfeit` = **no contest**(`winner = None`)、勝ちにも負けにも数えない、**分母は 50 固定**(不成立を除いて勝率を計算しない)、`no_contest == 0` が段階 2 の要求。

**運用**: clys 戦は常に `CLYS_TBP_LOG` を有効にして、再発時に code/signal を取る。

## 15. 評価の核心的な誤り(s2-spec-lead `4968782`)

**現在の評価は「消去の種類」に重みを付けていて、「実際に送る攻撃量」を見ていない。**

- single の `clear[None][1]` に重みがある → **しかし single の実攻撃は 0**
- したがって「single を撃つ方が得」に見える
- **`compute_attack` の結果そのものに重みを付ければ、single は自動的に 0 点になる**

**`placement_value = W_attack × 実際の攻撃量` が single バイアスの根本原因を消す最も安い修正。** 既存項の置き換えで新しい特徴量は不要。TSD(5)/TSS(3)/mini Quad(11)/quad(5)の比率が自動的に正しくなる。

### 付随する修正

- `combo_potential` は「次の 1 手でゴミの出現を止められる」ときだけ有効に(常時だと single を誘発)
- 準備項に `readiness_scale = min(wait_pieces/4, 1)`、`wait_pieces = 21 - max_height - pending`。**`budget = 0` なら準備項は 0**

### 訂正: Opener の ×2 は attack 対象

| 消去 | attack | Opener 中の相殺 |
|---|---|---|
| single | **0** | **0** |
| double | 1 | 2 |
| quad / TSD | 5 | **10** |

**9007-B は 5 ライン消して attack 1。問題は「0 ライン」ではなく「attack 0」。**

## 16. 攻撃の価格化で 19/20(2026-11-16)

### 実装(`850f238`)

1. **`clear[None]` を全て 0 に** — `weights.attack × attack.total` が唯一の価格。`compute_attack` が single 0 / double 1 / quad・TSD 5 / mini quad 11 を返すので、**順序が自動的に正しくなる**
2. **`combo_potential` を `height + pending >= 21` でゲート** — 「1 セルで埋まる行」は single でもあるため
3. **`readiness_scale = min(wait_pieces/4, 1)`** — 待つ余裕がないときは準備項 0

### 推移

| ビルド | 勝 | 敗 | a/p | a/line | spin% | net<0 | 崩壊 |
|---|---|---|---|---|---|---|---|
| 1-ply greedy(探索バグ時代) | 16 | 4 | 0.811 | — | — | — | — |
| 実探索・旧既定 | 7 | 13 | 0.722 | 1.12 | 16.3 | — | — |
| + danger/race/attack/combo | 14 | 6 | 0.745 | 1.17 | 16.1 | — | 1 |
| + 和条件 survival / DANGER_BASE 15 / TSD 40 | 18 | 2 | 0.761 | 1.19 | 16.1 | 2 | 1 |
| **+ 攻撃価格化 / ゲート / readiness** | **19** | **1** | 0.767 | **1.25** | 15.8 | **1** | **0** |

**TopOut のみ、forfeit 0、未決着 0、20/20 replay 検証済み。**

### 教訓

**「消去の種類」に重みを付けると single に最適化される。** single の実攻撃は 0 なので、`clear[None][1]` に正の重みがある限り「single を撃つ方が得」に見える。**`compute_attack` の結果そのものだけを価格にすれば、順序が自動的に正しくなる。**

### 残る課題

- **9010-B**(t75、net −6、最終 y 13): 崩壊の定義(`最終 y >= 14 かつ turn < 100`)に該当しない別の失敗モード
- **a/line 1.25 → 1.33**: spin% 15.8% が動かない。TSD の構造改善が必要(**手段。崩壊 0 を壊さない範囲で**)
- **20 ゲームでは判断できない**: 19/20 の Wilson CI は 68–99%。**40 ゲームに増やして確定させる**

## 17. 「防御」と「攻撃」は別問題ではない(s2-spec-lead `8ce8477`)

### 9010-B の診断

| turn | 我々の消去 | 我々の送信 | 相手の送信 |
|---|---|---|---|
| 62–63 | 0 | 0 | **5 + 7** |
| **64–68** | **1,1,1,1,1** | **0,0,0,0,0** | 0 |
| 70–71 | 0 | 0 | **5 + 6** |
| **74** | **0** | 0 | — ← **死** |

**相手は 11 turn で 23 行送り、我々は 6 ライン(single のみ)消した。**

- **1 ライン消すと、その turn の出現は止まる**(実際に 10 turn 生き延びた)
- **しかし pending は減らない。減らすのは相殺(攻撃を送ること)だけ**
- **single の攻撃は 0** → 相殺できない
- **pending 23 行のまま t74 で 0 ラインを置いて死ぬ**

### 結論

| 条件 | 内容 |
|---|---|
| **必要条件** | `H + pending <= 20` |
| **十分条件** | **`sent >= received` を維持する** |
| **single 主体では十分条件を満たせない** | 攻撃 0 なので `sent` が増えない |

**`attack/line` を上げることが唯一の恒久的な生存策**で、single 主体は**時間稼ぎにしかならない**。

**`placement_value = W_attack × 実際の攻撃量` は、攻撃だけでなく防御にも効いている。**

### 負けの 3 型(測定に追加)

| 型 | 判定 |
|---|---|
| **崩壊(自滅)** | `最終 y >= 14` |
| **pending 押し切り** | `y < 14` かつ **`received - sent >= 20`** |
| **攻撃レース負け** | `y < 14` かつ `received - sent < 20` |

**「崩壊 0」を達成しても、pending 押し切り型の負けは残る。両方を 0 にする必要がある。**

### 汎化のリスク

**CC2 の 19/20 に最適化が進むと、clys / attax への汎化が落ちる。**

- CC2 は**安定した攻撃** → 「net damage を保つ」戦略が効く
- clys は**burst 型**(1 turn 8 行)→ 同じ戦略では 10%
- **`H + pending >= 21` の survival は clys でも効くはず**だが、1 turn 8 行では `H+8` が一気に来る

**CC2 専用の重み調整は避ける。** 「`sent >= received` を維持する」は**相手に依存しない原則**なので、そこを軸にする。

## 18. 40 ゲーム測定で 9004-A が崩壊 — 評価に「防御の価値」が欠けていた(2026-11-16)

20 ゲームの 19/20 は **seed 運**でした。新しい seeds(9011–9020)を入れると **9004-A が最終 y 21 で崩壊**(sent 26)。

### 原因

`clear[None]` を 0 にした結果、**single の価値が「高さ −1」だけ**になった。しかし single には**実際の防御価値**がある:

```rust
pub fn take_insertable(&mut self, lines_cleared: u8, current_frame: u64) -> Vec<u8> {
    if lines_cleared > 0 { return Vec::new(); }   // この turn の出現を止める
```

**1 ラインでも消せば、その turn のゴミ出現が止まる。1 回の出現は最大 8 行**(`GARBAGE_CAP`)。

### 欠けていた項

**`defense_value = min(pending, 8)` if lines_cleared > 0 else 0**

| 状況 | 正しい行動 | 現状 |
|---|---|---|
| `pending = 0`、盤面低い | 攻撃(quad/TSD を狙う) | ○ |
| **`pending = 8`、盤面低い** | **1 ライン消して出現を止める** | **× 積む** |
| `pending = 8`、盤面高い | 1 ライン消し最優先 | × |

**`pending = 8` のときの 1 ライン消しは 8 行分の価値 = quad の攻撃 5 より高い。** 単位が「行」なので `attack` の重みと直接比較できる。

**9004-A は 2 行目で失敗した。** single の価値が 0 なので「積む」を選び、pending が 21 行まで溜まって死んだ。

### 期待される効果

- `pending = 0` では single を撃たない(攻撃 0・防御 0)→ **`attack/line` が上がる**
- `pending > 0` では 1 ライン消しを選ぶ → **崩壊と pending 押し切りが減る**
- **両方同時に改善する。トレードオフではない**

### 教訓

**20 ゲームでは足りない。** 19/20(Wilson CI 68–99%)は「あと 1 勝」と「まだ 10% 落ちる」を区別できない。**40 ゲームに増やしたことで崩壊が再現した。**

## 19. 測定の非決定性(2026-11-16)

### 発見

同じ seed で結果が分かれた:

| 測定者 | 9004-A |
|---|---|
| bot-dev | **崩壊**(最終 y 21) |
| harness-dev | **勝ち**(164 turn、最終 y 2) |

**原因**: anytime 探索が **8 スレッド + 実時間 deadline** で動くため、スレッドのタイミングで探索結果が変わり、手が変わる。

### なぜ問題か

- **20〜40 ゲームの A/B 比較に大きなノイズが乗る**。`defense` 14 vs 28 vs 56 の差がタイミングノイズに埋もれる
- **同じ seed で同じゲームが再現しない**ので「19/20」が何を意味するか曖昧
- **50/50 ゲートは 1 ゲームも落とせない**。運の要素は減らしたい
- replay 検証は成立する(記録した手を再生するため)

### 方針: 決定論モードを実装

- `deterministic = true`: **deadline チェックをせず**、固定の `(depth, width)` 列を実行。**必ず同じ結果**
- `deterministic = false`: 現在の anytime(実時間 deadline)
- スレッド数も固定して、チャンク分割を決定的にする
- **テスト**: 同じ seed で 2 回実行して完全に同じ replay になること

**決定論モードが公式ラウンドの要件になる可能性が高い**。再現可能なラウンドは監査可能性が段違い。**決定論モードが anytime より弱くても「弱いが再現可能」を選ぶ価値がある**。判断は数字を見てから。

### 測定プロトコルの変更

- **A/B は決定論モードで行う**。非決定なまま回すと結果を信じられない
- harness-dev の pre-fix 40 ゲーム測定は**中止**(bot-dev の 13/40 が比較基準)

## 20. 決定論モードの実装と公式ラウンドの方針(2026-11-16)

### 実装(`a99c135`)

```rust
pub struct SearchConfig { ..., pub deterministic: bool }   // 既定 false
```

- `true`: **deadline チェックなし**、`(depth, width)` を 1 回だけ実行
- ビームは決定的(子はチャンク順マージ、`commit` は value ソート)なので**スレッド数に依存しない**
- テスト `deterministic_mode_repeats_exactly`: 同一局面の 2 回探索で完全一致、**同一 seed のゲームが `actions`/`turns`/`lines` まで一致**

### 方針: 公式ラウンドは決定論モード

**非決定な 50/50 は「運が良かった」と言われうる。決定論モードなら 50 ゲームが完全に再現可能で、監査可能性が段違い。**

**決定論モードが anytime より数ポイント弱くても、再現可能性を優先する。**

| 用途 | モード |
|---|---|
| **A/B 比較** | **決定論** |
| **公式ラウンド** | **決定論** |
| 強さの上限確認 | anytime(参考) |

### 未解決: 決定論モードの思考時間

決定論モードは deadline チェックをしないので、**盤面によっては 100ms を大きく超える可能性**がある。**最悪ケースを必ず記録**し、超えるなら:
- スケジュールを下げる、または
- **`min(スケジュール, deadline)` のハイブリッド**(deadline に当たったら打ち切る。決定論は壊れるが時間は守る)

### テストの前提修正

`plays_a_legal_game` の `assert!(attack >= lines / 4)` は **S2 に存在しない前提**(single 0 / quad 1.25 / TSD 2.5 per line)。**上限の不変条件** `attack <= lines + surge` に変更した。

## 21. 公式の探索構成が確定(2026-11-16)

### 決定論モードの経緯

| 段階 | 結果 |
|---|---|
| `run_sequential` 固定 | 再現するが**遅すぎる**(`d8 w96` が毎手 95ms 張り付き、400/400 aborted) |
| ワーカーの deadline チェックを決定論モードで外す | **並列の速度を維持しつつスレッド非依存** |

**原因はワーカーの `if index % 4 == 0 && Instant::now() >= job.deadline { break; }`** — ビーム内で唯一の時刻依存箇所でした。

### 測定(`official`、400 手、release、8 スレッド、並列決定論)

| スケジュール | median | p99 | max | 100ms 超 | aborted |
|---|---|---|---|---|---|
| d4 w128 | 17.5 | 22.7 | 29.6 | 0 | **0** |
| **d6 w128** | **30.7** | 47.5 | **51.4** | 0 | **2** |
| d8 w96 | 36.5 | 94.2 | **98.9** | 0 | 3 |

### 決定: `d6 w128`

**理由は「強さ」ではなく「再現性・時間の裾の無さ・全手で depth を主張できること」。** これらは 50-0 の主張を強くする。

**`d8 w96` は不採用**: max 98.9 は 1.1ms の余裕しかない。

### 打ち切り時刻の計算(s2-spec-lead の指摘)

**レベル境界の打ち切りは、レベルの途中を止められない。**

```
実際の上限 = 打ち切り時刻 + 1 レベルの最大所要時間
```

**`d8 w96` の max 98.9 は「95ms で打ち切った後、そのレベルが 3.9ms 走った」結果。** 打ち切り時刻は `100 − max_level_time − 5` にすべき。`d6 w128` は max 51.4 なので 95ms のままで問題ない。

### anytime が上限を保証できない理由

**anytime の deadline チェックが「レベル間」だけ**なので、1 レベルが 20ms 以上かかると期限を 5ms 過ぎていても次のレベルに入る。**実測で anytime は 400 手中 10〜12 手で 100ms を超えた。**

## 22. 決定論と時間上限は両立しない(2026-11-16 確定)

### 最終設計

**決定論モードは時計を一切見ない。**

- 固定スケジュール `(depth, width)` 自体が仕事量の上限(`depth × width × ノードあたりの配置数`)
- **これは位置の関数で、機械の負荷に依存しない**
- **時間の保証はスケジュール選択で行う**(実行時の打ち切りではない)
- `aborted` は決定論モードで常に false → **指標として無意味**(常に偽の条件は条件ではない)

### なぜ打ち切りを削除したか

**レベル境界の打ち切りも実時間だった。** ここで打ち切られると**完了するレベル数が変わり、手が変わる**。短いテスト(30 turn)では発火せず、**長いゲームでだけ発火**した。

### 公式構成

| 項目 | 値 |
|---|---|
| モード | 決定論 |
| スケジュール | `depth 6 / width 96` |
| 時間上限 | **なし(設計上)** |
| 実測(専有、1,200 手) | median 23.3ms / max 47.5〜72.9ms |
| `aborted` | 常に false |
| 専有 | **必須**(負荷が 3 倍になれば 200ms になり得る) |

### 不採用の候補

| スケジュール | 理由 |
|---|---|
| `d6 w128` | max が 99.5ms まで行く回がある |
| `d8 w96` | max 105.5ms、負荷に関係なく不適 |
| `d4 w128` | max 40.1ms で安全だが depth 4 は anytime の p50 6 より浅い |

### 測定論の整理(s2-spec-lead §8.4.1.1)

| 指標 | 何を主張できるか |
|---|---|
| `depth` / `width` / `nodes` | **「この手で何をしたか」を機械負荷に依存せず主張できる** |
| `think_ms` | 「この機械でどれだけ時間がかかったか」。**ボットの性質ではない** |

**避けるべき 2 つの誤り**: ①「100ms 思考したから探索した」(1-ply 事件) ②「111.9ms かかったから違反」(負荷かもしれない)。

### 専有スケジュール

**bot-dev が先、harness-dev が後。公式ラウンドはラウンドの間だけ専有。** `scripts/measurement-lock.sh` で排他制御。

## 23. 決定論が完成(2026-11-16)

### 最後の穴: マージのタイブレーク

時計を外しても、**ビームマージの同値候補のタイブレークがワーカーの完了順に依存**していました。

```rust
// 修正前: (value desc, key asc, index asc)
//   index = マージ後のベクトル位置 → ワーカーの完了順に依存
// 修正後: (value desc, key asc, (rotation, x, y, hold_used) asc, index asc)
```

**タイブレークを「配置そのもの」にした**ので、ソートが内容の全順序になり、到着順に依存しません。`state_key` は FNV(ランダムシードなし)なので、ハッシュ自体は非決定ではありませんでした。

### 検証

- **`cargo test --workspace` が並列で 3 回連続緑**(288 passed)
- **`deterministic_mode_repeats_exactly` を `threads: 1 / 2 / 8` すべてで検証**
- **arena の `threads: 2` / `8` テストも緑**

### 公式構成(最終)

| 項目 | 値 |
|---|---|
| モード | 決定論 |
| スケジュール | `depth 6 / width 96` |
| 時間上限 | なし(設計上) |
| 実測(専有、1,200 手) | median 23.3ms / max 47.5〜72.9ms |
| 専有 | 必須 |

**再現性が確保された。** 同じ seed + 同じ設定で**完全に同じゲーム**が再現できる。

## 24. 3 方式の比較: anytime の実測(2026-11-16)

`cc2m-anytime`(9001–9010 × 両シート、20 ゲーム):

| 項目 | 値 |
|---|---|
| 勝敗 | **19 勝 1 敗**(unresolved 0、no-contest 0、20/20 検証済み) |
| 負け | **9003-B**(99 turn、net −3、a/p 0.657) |
| `attack/piece` | 0.767 |
| `attack/line` | **1.129** |
| `spin rate` | **8.5%**(以前は 16%) |
| `think_ms` max | **109.2ms** |
| **100ms 超** | **10 手** |
| `fallback` 率 | 0.0% |
| `depth` 中央値 | **8.0** |

### 重要な発見

1. **実ゲームの depth 中央値は 8.0**(proxy 測定では p50 6)。**実ゲームの盤面は proxy より浅く、anytime は depth 8 に届いている。** したがって **`d6 w96` は anytime より浅い**可能性が高い
2. **anytime は 10 手が 100ms 超、max 109.2ms** — deadline チェックがレベル間だけなので**時間を守れない**。決定論を選ぶ理由の実測証拠
3. **`spin rate` が 16% → 8.5% に低下**。`clear[None]` を 0 にした影響の可能性。TSD の構造改善は後回しだが記録

### 次の候補

`d6 w96` が anytime に明確に負ける場合:

| 候補 | 根拠 |
|---|---|
| **`d8 w64`** | depth 8 を維持しつつ幅を絞る。コストは `d6 w96` の約 0.89 倍の見込み |
| `d8 w96` | 実測 max 105.5 で不適 |
| `d6 w128` | max 99.5 で不適 |

## 25. 100ms は上限(マネージャ判断、2026-11-16)

### 判断

**goal の「思考 100ms」は上限と読む。** 超過は「相手より多くの計算資源を得た」ことを意味するので、**公平性の主張が崩れる**。10 手の超過を注記して通すのは 50-0 の主張を弱めるので取らない。

**したがって anytime は公式ラウンドに使えない**(`think_ms` max 109.2ms、100ms 超 10 手)。

**判定は `max` で行う**(p99 ではない)。**1 手でも超えたら不適。**

### 方向は「スケジュールを諦める」ではなく「高速化」

**anytime の 19/20 は depth 8 で取った数字。** `d6 w96`(depth 6)で届くかは測定中だが、届かない可能性が高い。

**10〜20% の高速化は十分見込める**:

| 処理 | 削減案 |
|---|---|
| **`t_slot_cutout` の盤面 clone** | **やめる**(毎リーフ 1 回 + `best_t_slot` でもう 1 回) |
| `best_t_slot` の probe clone | 同上 |
| `apply_placement` の盤面 clone | in-place + undo |
| `analyse`(400 セル走査) | 親からの差分更新 |

**`d8 w96` は max 105.5ms なので、10% 速くなれば 95ms 前後**で条件を満たす。**clone 除去は局所的で探索の意味を変えない**(低リスク)。

### 目標

| スケジュール | 現在 max | 目標 max |
|---|---|---|
| `d8 w96` | 105.5 | **≤ 85** |
| `d6 w96` | 72.9 | 現行の安全弁 |

**`d8 w96` が max ≤ 85ms になれば、depth 8 + 決定論 + 条件遵守が揃う。** これが最良の構成。

### 検証条件

- `official`、1,200 手以上で median / p99 / max / 100ms 超
- **`cargo test --workspace` 3 回連続緑**
- **`deterministic_mode_repeats_exactly` が `threads: 1/2/8` で緑**
- **最適化で同じ seed のゲームが変わらないこと**(変わったらバグ)

## 26. `d6 w96` の実力と `d8` への道(2026-11-16)

### 3 方式の比較(CC2、9001–9010 × 両シート)

| 方式 | ゲーム | 勝 | 敗 | depth | `think_ms` max | 100ms 超 | 判定 |
|---|---|---|---|---|---|---|---|
| **anytime** | 20 | **19** | 1 | **8.0** | 109.2 | **10 手** | **条件違反** |
| **決定論 `d6 w96`** | 13 | **11** | 2 | 6 | 72.9 | 0 | 条件遵守 |
| 決定論 `d6 w128` | – | – | – | 6 | 99.5 | 0 | 余裕なし(不採用) |
| 決定論 `d8 w96` | – | – | – | 8 | 105.5 | 0(400 手) | **要高速化** |

**`d6 w96` は 84.6%(11/13)。anytime(depth 8)の 95% より弱い。** 負けは 9003-B / 9004-A。**depth が効いている。**

### したがって本命は `d8 w96` を通すこと

**10〜20% の高速化で 105.5 → 85〜95ms になる見込み。** 意味を変えない最適化:

| 処理 | 状態 |
|---|---|
| **`best_t_slot` の probe clone** | **削除済み**(`aa490ed`、毎リーフ 1 回) |
| `t_slot_cutout` の外側 clone | 未(undo が必要) |
| `apply_placement` の clone | 未(in-place + undo) |
| `analyse` の差分更新 | 未(重い) |

**意味を変えない最適化は強さの測定を汚さない**(protocol §2.4.13)。**`think_ms` だけ測り直せばよい。**

### 検証条件

- `cargo test --workspace` 3 回連続緑
- `deterministic_mode_repeats_exactly` が `threads: 1/2/8` で緑
- **同じ seed で同じゲーム**(最適化は意味を変えないので、`cc2m-det96` の replay と一致するはず。変わったらバグ)

## 27. 公式構成が確定: `d8 w96` 決定論(2026-11-16)

### 出発点からの推移

| 項目 | 最初 | 現在 |
|---|---|---|
| 探索 | **1-ply greedy にフォールバック** | 決定論 depth 8 / width 96 |
| 実戦 depth | 1 | **8**(全手で保証) |
| 決定論 | なし(スレッド依存) | **`threads: 1/2/8` で同一結果** |
| `think_ms` max | – | **79.2ms**(余裕 21ms) |
| 100ms 超 | – | **0/1,200 手** |

### 3 つが揃った

| 項目 | 状態 |
|---|---|
| **深さ** | depth 8(anytime の実ゲーム depth 8 と同等) |
| **決定論** | 時計を見ない・内容の全順序・threads 1/2/8 で同一 |
| **100ms 遵守** | max 79.2、余裕 21ms |

**anytime の 100ms 違反(10 手、max 109.2)が無く、同じ深さ。** これが公式構成として最良。

### 25% 高速化の内訳

**`best_t_slot` の clone 除去だけで 105.5 → 79.2ms。** 毎リーフで盤面を clone して `place` + `clear_lines` していたのを、**2 行マスクで行が揃うかを直接判定**する形にした。

**意味は変わらない**(同じ seed で同じゲームになることをテストで確認)。

### 残る最適化は予備

`t_slot_cutout` の外側 clone と `apply_placement` の clone は**必須ではない**(余裕 21ms)。**1,200 手で max 79.2 でも 10,000 手では裾が伸びる**ので、**100ms を超える手が出たら実施**する予備として保持。

## 28. `d8 w96` の実ゲーム結果(2026-11-16)

### 勝敗(20 ゲーム、5 分類の合計検算 20)

| 分類 | 件数 |
|---|---|
| **TopOut 勝ち** | **19** |
| TopOut 負け | **0** |
| **未決着(TurnCap)** | **1**(9010-B、1000 turn) |
| no-contest | 0 |
| forfeit | 0 |

**anytime(19 勝 1 敗)より良い。負け 0。**

| 項目 | `d8 w96` 決定論 | anytime |
|---|---|---|
| `attack/piece` | 0.761 | 0.767 |
| `attack/line` | **1.135** | 1.129 |
| `spin rate` | 8.6% | 8.5% |
| `depth` 中央値 | **8.0** | 8.0 |
| `fallback` 率 | **0.0%** | 0.0% |

### 未解決 2 件(どちらもゲートを止める)

**1. `think_ms` 100ms 超が 2 手**(max 111.2、`load_note` 発火)

**負荷ではなく実ゲームの裾**。proxy の 1,200 手(max 79.2)は実ゲームの裾を過小評価していた。超過した 2 手は 436 turn / 207 turn の長いゲームにある。

**対応**: 残る clone 除去(`t_slot_cutout` の外側 clone、`apply_placement` の in-place + undo)。**目標は実ゲーム 20 ゲームで 100ms 超 0。**

**2. TurnCap 1 件(9010-B、1000 turn)**

**段階 1 は `turncap == 0` を要求するので不通過。** 1000 turn でどちらも TopOut しないのは異常で、**両者が安全に積み続けた**ことを意味する。

**仮説**: `defense` 項と `race` 項が「相手より多く送っているので守備に回る」と判断し、両者が延長戦に入った。あるいは `danger` の非線形項が強すぎて高さ 15 付近で固まった。

**TurnCap は「勝てなかった」ではなく「殺しに行かなかった」**なので、**攻撃性の調整が必要**になる可能性がある。

## 29. TurnCap の定理(`docs/s2-turncap-theorem.md`)

### なぜ 9010-B は 1000 turn 決着しなかったか

| 消去 | 回数 | 攻撃 | 寄与 |
|---|---|---|---|
| **single** | **373(53%)** | **0** | **0** |
| double | 51 | 1 | 51 |
| triple | 18 | 2 | 36 |
| **quad** | **44(9%)** | **5** | **220** |
| combo ln・GSB | — | — | 約 66 |

**`lines/piece = 0.705`。ゴミなしの上限は 0.4。** **0.705 は「受けたゴミを消している」ことを意味する。**

**ループ**: 0 ライン設置(514 回)→ ゴミ挿入 → single(373 回)で 1 行消す → 盤面は上がらない → 繰り返し。**我々は「ゴミ処理機」**。

### 定理

```
ΔH = 0.4 − L/P + max(0, S_them − S_us)/P
```

**`S_us <= S_them` が持続し `L_them/P >= 0.4` なら TurnCap。** CC2 の `L_them/P = 0.581 > 0.4` → **CC2 は自己安定**。**我々がゴミを送らない限り絶対に死なない。**

**決着条件**: `a_us > 2.5 × S_them/P`。CC2 は `S_them/P = 0.715` → **`a_us > 1.79`**。

**9010-B は `a_us = 0.53`、`S_us = 373 <= S_them = 715` が持続 → `ΔH_them = −0.181` → `T = ∞`。**

**我々の送信は既に理論上限の 96.7%**(`a_us = 0.53` での上限 386、実測 373)。**量ではなく質(`a/line`)の問題。**

### 原因は `defense` ではなく `height` 項

**`arrived ≈ 0` なら `defense ≈ 0`。** それでも single が選ばれるのは:

| 消去 | 高さ削減 | ピース | 高さ効率 |
|---|---|---|---|
| **single** | 1 | 1 | **1.0** |
| quad | 4 | 10 | **0.4** |

**ゴミが 0.7 ライン/ピースで来るとき、`height` 項は single を正しく評価している。** **single は最も高さ効率の良い消し方。** **「single を減点する」のは誤り**(盤面が上がって死ぬ)。

### 施策

1. **`height` / `danger` を `risk(H)` でゲート(下限 0.2)** — 低い盤面では高さの罰則を弱め、攻撃に回る
2. **測定に `S_us > S_them` を追加** — 決着の必要条件
3. **burst を優先** — `a/line` 1.79 は現実的でないので「長引かせない」が本命。**19 勝は短いゲームで burst して勝っている**

## 30. 🚨 P0: シート B が turn 1 で forfeit(2026-11-16)

### 症状

`cc2m-risk`(20 ゲーム)の結果: **10 勝 0 敗 0 未決着 10 no-contest**。**シート B の 10 ゲームが全部 `forfeit_other`、turns=1。**

CC2 の応答:
```
{"payload":{"move_info":{"extra":"NaN% of selections expanded, overall speed: 0.0 Mnps","nodes":0,"nps":0.0},"moves":[]},"type":"suggestion"}
```

**空盤面で `moves: []`、`nodes: 0`、`NaN%`** = CC2 の movegen が 0 配置しか見つけていない。

### 切り分け(マネージャが実施)

| 条件 | 結果 |
|---|---|
| シート A(我々が seat 1、CC2 が seat 2) | **動く** |
| **シート B(CC2 が seat 1、我々が seat 2)** | **turn 1 で forfeit** |
| `--deterministic` なし | 同じく forfeit |
| `--tbp-visible-rows 20` | 同じく forfeit |
| **`171eb3c`(前のコミット)のバイナリ** | **同じく forfeit** |

**コードの回帰ではない。環境依存のレース。** `171eb3c` でも再現するので、**以前の `cc2m-det8w96`(19 勝 0 敗 1 未決着)は運が良かった**可能性がある。

### 仮説

**TBP host が `start` を送ってから、CC2 の読み取りスレッドが準備できる前に `suggest` を送っている。**

- シート A では CC2 が後手なので、我々の手番の間に `start` を処理する時間がある
- シート B では CC2 が先手なので、`start` の直後に `suggest` が来る

### 影響

**シート B が動かないと `no_contest == 0` のゲートが絶対に通れない。** また**過去の「20 ゲーム」の数字が両シートだったか**の再検証が必要。

### `cc2m-risk` の参考値(シート A の 10 ゲームのみ)

- 10 勝 0 敗、`a/p` 0.723、`a/line` **1.12**、`spin` 8.6%、`net<0` 0、**崩壊 0**、`over100` 2、`depth` 8、`fallback` 0%

**`risk` ゲートで `a/line` は 1.135 → 1.12 とほぼ変わらず、spin も 8.6% のまま。** ただしシート A のみなので判断できない。

## 31. `risk=0` で TurnCap は消えたが、負けと時間が悪化(2026-11-16)

### 結果(`cc2m-risk0`、20 ゲーム両シート)

| 指標 | `risk=0.2` | **`risk=0`** | 目標 |
|---|---|---|---|
| **TurnCap** | 1 | **0** ✅ | 0 |
| **勝敗** | 19-0-1 | **17-3-0** ❌ | 20-0 |
| **崩壊** | 0 | **1** ❌ | 0 |
| `a/line` | 1.12 | 1.088 | 1.33 |
| `spin` | 8.6% | 9.4% | 15% |
| **`over100`** | 2 | **13** ❌ | 0 |
| `net<0` ゲーム | 0 | 0 ✅ | 0 |
| `depth` / `fallback` | 8 / 0% | 8 / 0% | 8 / 0% |

### 診断

**1. 時間が悪化した理由**: 盤面が高くなると候補数が増え、`d8 w96` が重くなる。**clone 除去が必須になった。**

**2. `a/line` と `spin` はほぼ動いていない**(1.12 → 1.088、8.6% → 9.4%)。**高さを無料にしても quad/TSD を狙わない** = **別の項が single を正当化している**。

**候補**: `holes` / `bumpiness` / `row_transitions`。**single は平らに積めるので形状項で有利。quad は 4-wide の穴を一時的に作るので形状項で不利。**

**3. 崩壊 1 件** — 高さを無料にした副作用。

### 現在地

**`risk=0.2` は 19-0-1 だが TurnCap 1、`risk=0` は TurnCap 0 だが 17-3-0 + 崩壊 1 + `over100` 13。** どちらもゲート不通過。

**必要なのは「TurnCap 0 かつ負け 0 かつ崩壊 0 かつ `over100` 0」。** 現状は 4 条件のうち 2 つしか同時に満たせていない。

## 32. 既定値が危険側だった: シート B の P0 の真因(2026-11-16)

### 原因

```
crates/arena/src/bin/arena.rs:81   let mut tbp_retry_empty_ms = 0u64;   // 既定 0
scripts/tuning-smoke.sh:41         --tbp-retry-empty-ms 2000            // 明示的に渡している
```

**`tuning-smoke.sh` は渡しているが、`arena match` を直接叩くと既定 0 で `TbpAgent::begin_round` の probe が無効**になる。**シート B(相手が先手)では `start` の処理前に `suggest` が届き、CC2 が `moves: []` / `nodes: 0` を返して turn 1 で forfeit。**

**修正**: `tbp_retry_empty_ms` の既定を **2000** に。`0` は明示的な opt-in。

**教訓**: **「既定値が危険側」というバグの種類**。`supports_180`(既定 `true`、仕様は `false`)も同じ種類。

### 決定論の切り分け(harness-dev、確定)

同一 seed・同一ビルドで 2 回実行し、`placements` を slot 別に比較:

- **turn 0: 両 slot 完全一致**
- **turn 1, slot 1(CC2): 最初の分岐**(T が `x=7` vs `x=5`)
- **slot 0(intetrigence)は完全一致**

**我々の手は局面列の純関数。CC2 は別プロセスで独自の時間予算を持つため非決定。**

**protocol の主張を「自手の決定性」に限定**(v1.54)。`verify_replay` は記録された手から再シミュレーションするので**相手の決定性に依存せず**、監査可能性は保たれる。

### 監査の穴の修正(harness-dev)

`crates/arena/build.rs` が `crates/bot` + `crates/core` + `crates/arena` の `.rs` を FNV-1a でハッシュ → `ARENA_BUILD_ID`。replay header / audit / `summary.json` に `build_id` と `binary`(パス・サイズ・mtime)を記録。**`--resume` は bot 名と build id の両方を比較。**

**「git HEAD はソースの状態しか表さず、バイナリがそのソースでビルドされたかを保証しない」**という指摘が正しかった。**stale binary が「最新コミットの結果」として記録される穴**が塞がれた。

### `risk=0.1` の測定(シート A のみ、10 ゲーム)

| 指標 | 値 |
|---|---|
| 勝敗 | **8 勝 2 敗** |
| TurnCap | **0** |
| `a/line` | 1.11 |
| `spin` | 9.1% |
| `net<0` | 2 |
| **崩壊** | **2** |
| `over100` | 3(max 115.7) |
| `think_median` | **34.9**(高さは管理できている) |

**`risk=0.1` は高さ管理が良好で TurnCap 0 だが、崩壊 2 と負け 2。** `risk=0.2`(19-0-1、崩壊 0、`think_median` ~30)の方が安定。

### 公平性監査(core-engine が要約)

`docs/fairness-audit.md`(633 行)。**ゲームルールではなく情報 I/F・思考時間・監査メタデータの 3 層に不公平がある。**

| # | 重要度 | 内容 |
|---|---|---|
| **F1** | **高** | **情報非対称**: intetrigence だけが `pending_garbage` / `arrived_garbage` / 正確な `b2b_streak` / `sent_total` / `received_total` を受け取る。TBP は `back_to_back` bool のみ |
| **F2** | 高 | CC2/clys だけ空候補を最大 2 秒リトライ |
| **F3** | 高 | `supports_180` の既定が `true`、仕様は `false` |
| **F4** | 中〜高 | intetrigence 8 threads / 外部ボットは実質 1 thread。threads / suggest_delay / retry が記録されない |
| F5/F6 | 中 | replay header の版が古い、slot 1 を常に attax と記録、TurnCap を draw と表示、パッチ SHA 欠落 |
| F7 | 中 | Clutch 時に active 位置がズレる(TBP `start` に active が無い) |
| F8/F9 | 中 | `--seeds` override の抜け道、`forfeit-other` とゲートの矛盾 |
| F10-F13 | 低 | RNG の位相違い、文書の誤り |

**core のルールは両者同一で、不公平な分岐は無い。** F1〜F9 は harness/arena 側。

## 33. 40 ゲーム完走: 37 勝 3 敗、TurnCap 0、no-contest 0(2026-11-16)

### `risk=0.1`、`d8 w96` 決定論、seeds 9001–9020 × 両シート(40 ゲーム)

| 分類 | 件数 |
|---|---|
| **TopOut 勝ち** | **37** |
| **TopOut 負け** | **3** |
| 未決着(TurnCap) | **0** ✅ |
| no-contest | **0** ✅ |
| stuck | **0** ✅ |
| **合計** | **40** ✓ |

**92.5%。TurnCap 0、no-contest 0、崩壊 0。** `risk=0.1` は**構造として最良**。

### 残る唯一のブロッカー: 時間

| 指標 | 値 | 目標 |
|---|---|---|
| **`over100`** | **33 手** ❌ | 0 |
| `think_max` | 115.8ms ❌ | ≤ 100 |
| **`think_median`** | **71.5ms** | 30〜40 |
| `a/line` | 1.137 | 1.33 |
| `spin` | 9.1% | 15% |
| `net<0` | 1 ゲーム | 0 |
| `depth` / `fallback` | 8 / 0% | 8 / 0% |

**`think_median` 71.5 は「盤面が高い」ことを意味する。** `risk=0.2` では 30 前後だったので、**`risk=0.1` は高さを許して候補数を増やし、探索が重くなっている**。

### 必要な対応

1. **clone 除去**(`t_slot_cutout` の外側 clone、`apply_placement` の in-place + undo、必要なら `analyse` の差分更新)。**`think_median` 71.5 → 45 以下が目標**
2. **足りなければ `d8 w64`**(depth 8 を保ち幅を半減)
3. `d6 w96` は最終手段(depth 6 は anytime の p50 6 と同じで、決定論にする意味が薄れる)

### ゲートとの距離

| 条件 | 現状 |
|---|---|
| 50 勝 0 敗 | **37/40(92.5%)** — 3 敗 |
| TurnCap 0 | **達成** |
| no-contest 0 | **達成** |
| 100ms 超 0 | **未達(33 手)** |

**4 条件のうち 2 つ達成。残るは「負け 0」と「100ms 超 0」。**

## 34. clone 除去後: 40 ゲームで 37-3、**100ms 超 0 を達成**(2026-11-16)

### `cc2m-40fast`(`risk=0.1` + clone 除去、seeds 9001–9020 × 両シート)

| 分類 | 件数 |
|---|---|
| **TopOut 勝ち** | **37** |
| **TopOut 負け** | **3** |
| 未決着(TurnCap) | **0** ✅ |
| no-contest | **0** ✅ |
| stuck | **0** ✅ |
| **合計** | **40** ✓ |

| 指標 | 値 | 判定 |
|---|---|---|
| **`over100`** | **0** ✅ | **達成** |
| `think_max` | 97.7ms | 達成(余裕 2.3ms) |
| `think_median` | 70.8ms | 高い(盤面が高い) |
| `depth` / `fallback` | 8 / 0% | 達成 |
| `load_note` | **None** ✅ | 汚染なし |
| `a/line` | 1.136 | 未達 |
| `spin` | 9.9% | 未達 |
| `collapses` | **2** | 未達 |
| `net<0` | 2 ゲーム | 未達 |

**ゲート 4 条件のうち 3 つ達成**(TurnCap 0、no-contest 0、100ms 超 0)。**残るは「負け 0」。**

### 3 敗はすべて「短いゲームでの早期死」

| game | turn | net | 最終 y |
|---|---|---|---|
| 9004-A | **38** | −10 | 12 |
| 9007-B | **43** | −6 | 14 |
| 9011-A | **82** | 2 | 20 |

**38 turn / 43 turn で死んでいる。** 長いゲームの TurnCap ではなく、**序盤の崩壊**。

### 原因

**`risk` の下限 0 だと、序盤(`pending = 0`、`defense = 0`、`height` 無料)に積み上がり、その後の burst で死ぬ。**

`risk=0.2` は崩壊 0 だったが TurnCap 1。`risk=0.1` は TurnCap 0 だが崩壊 2。**しきい値の設計が「TurnCap」と「早期崩壊」のトレードオフになっている。**

### 次の候補

**`risk` のしきい値を下げる**(低い高さから罰則を効かせる):

| 案 | 式 | 狙い |
|---|---|---|
| 現行 | `clamp((H-12)/6, 0, 1)` | H≤12 で無料 |
| **A** | `clamp((H-8)/8, 0, 1)` | H≤8 で無料、H=16 で最大 |
| **B** | `clamp((H-10)/6, 0, 1)` | H≤10 で無料、H=16 で最大 |
| C | `clamp((H-8)/6, 0, 1)` | H≤8 で無料、H=14 で最大 |

**序盤の積み上がりを抑えつつ、攻撃の自由度を残す**のが狙い。

## 35. `risk` しきい値の A/B: `base`(8/8)が 20-0(2026-11-16)

`risk` の式を `clamp((H - risk_base) / risk_span, 0.1, 1.0)` と重み化し、3 腕を 20 ゲームずつ(seeds 9001–9010 × 両シート)。

| 腕 | `risk_base` / `span` | 勝 | 敗 | TurnCap | no-contest | 崩壊 | `net<0` | `over100` | `think_max` | `a/line` | `spin` |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **base** | **8 / 8** | **20** | **0** | 0 | 0 | **0** | **0** | 5 | 103.0 | 1.114 | 8.8% |
| mid | 10 / 6 | **20** | **0** | 0 | 0 | 0 | 0 | 13 | 116.5 | 1.149 | 9.8% |
| tight | 8 / 6 | 18 | 1 | 0 | **1** | 0 | 1 | 9 | 115.2 | 1.069 | 8.6% |

**`base`(H≤8 で無料、H=16 で最大)が最良: 20 勝 0 敗、TurnCap 0、no-contest 0、崩壊 0、`net<0` 0。**

**残るは時間だけ**: `over100` 5 手、`think_max` 103.0ms(**3% 超過**)。`think_median` 74.1ms は盤面の高さ由来(負荷ではない)。

### 次の手

1. **`risk_base=8` / `risk_span=8` を既定に**
2. **`apply_placement` の in-place + undo** で裾を削る(最後の clone)
3. **40 ゲームで 4 条件同時達成を確認**

### 推移(CC2、20 ゲーム単位)

| 構成 | 勝 | 敗 | TurnCap | 崩壊 | `over100` |
|---|---|---|---|---|---|
| 1-ply greedy | 16 | 4 | 0 | – | – |
| 実探索・旧既定 | 7 | 13 | 0 | – | – |
| + 攻撃価格化 | 19 | 1 | 0 | 0 | – |
| `d8 w96` 決定論(risk=0.2) | 19 | 0 | **1** | 0 | 2 |
| `risk=0` | 17 | 3 | 0 | 1 | 13 |
| `risk=0.1`(40 ゲーム) | 37 | 3 | 0 | 2 | 0 |
| **`risk_base=8/8`** | **20** | **0** | **0** | **0** | 5 |

## 36. `risk` 8/8 + `width 88` で 38-2、**ゲート 4 条件のうち 3 つ達成**(2026-11-16)

### `cc2-40-base-w88`(seeds 9001–9020 × 両シート、40 ゲーム)

| 分類 | 件数 |
|---|---|
| **TopOut 勝ち** | **38** |
| **TopOut 負け** | **2** |
| 未決着(TurnCap) | **0** ✅ |
| no-contest | **0** ✅ |
| stuck | **0** ✅ |

| 指標 | 値 | 判定 |
|---|---|---|
| **`over100`** | **0** ✅ | **達成** |
| `think_max` | 97.9ms | 達成(余裕 2.1ms) |
| `think_median` | 69.9ms | 高い(盤面が高い) |
| `depth` / `fallback` | 8 / 0% | 達成 |
| `load_note` | **None** ✅ | 汚染なし |
| `collapses` | **0** ✅ | 達成 |
| `net<0` | 1 ゲーム | 未達 |
| `a/line` / `spin` | 1.121 / 9.3% | 未達 |

**95%。ゲート 4 条件のうち 3 つ達成。残るは「負け 0」だけ。**

### 2 敗は「攻撃レースの負け」(早期崩壊ではない)

| game | turn | net(我々, CC2) | 最終 y |
|---|---|---|---|
| 9008-B | 178 | **−20, 96** | 18 |
| 9017-A | 272 | 60, 74 | 21 |

**9008-B は CC2 に 96 対 20 で圧倒された。** 9017-A は接戦(60 対 74)。

**早期崩壊(38 turn / 43 turn)は消えた。** `risk_base=8` が序盤の積み上がりを抑えたため。

### 推移(CC2)

| 構成 | 勝 | 敗 | TurnCap | no-contest | 崩壊 | `over100` |
|---|---|---|---|---|---|---|
| 1-ply greedy | 16 | 4 | 0 | – | – | – |
| 実探索・旧既定 | 7 | 13 | 0 | – | – | – |
| + 攻撃価格化 | 19 | 1 | 0 | 0 | – | – |
| `d8 w96` 決定論(risk=0.2) | 19 | 0 | **1** | 0 | 0 | 2 |
| `risk=0` | 17 | 3 | 0 | 1 | 1 | 13 |
| `risk=0.1`(40 ゲーム) | 37 | 3 | 0 | 0 | 2 | 0 |
| `risk=0.1` + clone 除去(40) | 37 | 3 | 0 | 0 | 2 | **0** |
| **`risk` 8/8 + `width 88`(40)** | **38** | **2** | **0** | **0** | **0** | **0** |

### `Stuck` → TopOut の修正(harness-dev `ea5633b`)

**spawn セル(row 21)が埋まっている盤面 = そのプレイヤーの TopOut。** `suggest` を待たずに判定する。

**clys 10 再実行で no-contest が 1 → 0。** 9006-A は 97 turn で我々の TopOut 勝ちになった。**clys 段階の `no_contest == 0` が原理的に可能になった。**

**ただし clys は 5 勝 5 敗。** `a/line` 1.7(clys)対 1.17(我々)。**段階 2 は遠い。**

## 37. 20 ゲームの A/B は seed 運だった: D 腕は 40 ゲームで 35-5(2026-11-16)

### 結果

| 腕 | 20 ゲーム(9001–9010) | **40 ゲーム(9001–9020)** |
|---|---|---|
| 基準(`risk` 8/8、`w88`、既定の重み) | — | **38-2** |
| A(`quad_ready=45,b2b_break=-32,wasted_t=-30`) | 20-0 | 未測定 |
| **D**(`quad_ready=45,defense=22,b2b_break=-32,surge_charge=2.2,surge_exponent=1.3`) | **20-0** | **35-5** ❌ |

**D の 20-0 は seed 運でした。** 40 ゲームに広げると 5 敗。**基準(38-2)の方が良い。**

### 判断

**D を revert し、基準の重みに戻しました**(`81d06e6`)。

**教訓: 20 ゲームの A/B では足りない。** 20 ゲームで 20-0 → Wilson CI 83–100%。**40 ゲームで 35-5 になると 87.5%。** 差は 20 ゲームでは見えない。

**今後、重みの A/B は 40 ゲーム(seeds 9001–9020 × 両シート)を最小単位**とします。**20 ゲームは「方向を見る」ためだけに使い、採用判断には使いません。**

### D の 5 敗(参考)

| game | turn | net(我々, CC2) | 最終 y |
|---|---|---|---|
| 9008-A | 583 | 103, 175 | 14 |
| 9009-B | 343 | 8, 151 | 18 |
| 9011-B | 107 | −4, 57 | 25 |
| 9013-B | 111 | −13, 53 | 15 |
| 9019-A | 275 | 55, 64 | 15 |

**9008-A / 9009-B は CC2 に 1.7〜19 倍の攻撃を浴びた。** `defense=22` に下げたことで**守備が弱くなり、CC2 の攻撃を受け切れなくなった**と考えられます。

### 現在の最良構成

**`risk_base=8` / `risk_span=8` / `risk_floor=0.1`、`width 88`、決定論 `depth 8`、既定の重み → 38-2。**

**ゲート 4 条件のうち 3 つ達成**(TurnCap 0、no-contest 0、`over100` 0)。**残るは「負け 0」だけ。**
