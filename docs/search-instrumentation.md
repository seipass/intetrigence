# 探索の計器 — 受け入れ条件(1 ページ)

**目的**: `think_ms` は「時間を費やした」ことしか示さない。**探索が実際に走ったこと**を証明する計器を定義する。
**根拠**: protocol §8.4.1(事故: anytime ループが期限超過で反復を捨て、全設定で 1-ply greedy にフォールバックしていた)
**対象**: `crates/bot` が返す `suggestion`、`crates/arena` の監査ログ、レポート

---

## 1. `suggestion` に必須のフィールド

| フィールド | 型 | 定義 |
|---|---|---|
| `depth` | u32 | **完了した最深反復の深さ**。`START_DEPTH` から +2 ずつ |
| `width` | u32 | **その反復で展開した最大幅**(候補数) |
| `nodes` | u64 | **その反復で訪問したノード数** |
| `iterations` | u32 | **完了した反復の数** |
| `fallback` | bool | **`iterations == 0` のとき true**。それ以外は false |
| `fallback_reason` | string? | `fallback == true` のとき必須。`"budget"` / `"no_moves"` / `"error"` |

**`depth` / `width` / `nodes` は「完了した最深反復」の値**。途中で捨てた反復の値を混ぜない(混ぜると「探索した」と誤って主張できる)。

---

## 2. `fallback` の定義(厳密)

```
fallback = (iterations == 0)
```

- **1 レベルでも完了していれば `fallback = false`**。期限超過で深い反復を捨てても、完了した浅い反復の結果を返す
- **`fallback = true` は「1 レベルも完了せず、greedy(1-ply)で打った」ときだけ**
- **`fallback = true` の手は「探索していない手」**。レポートで件数と割合を出す

> **v1 の誤り**: 期限超過時に `None` を返し、呼び出し側が greedy に落ちる設計だった。この設計では **`fallback` を記録できない**(落ちたことが観測できない)。**必ず「完了した最深結果」を返し、`iterations` を数える。**

---

## 3. 集計方法(レポート)

| 指標 | 定義 |
|---|---|
| `depth` の median / p95 / max | 全手の `depth` の分布。**実戦の深さ**を示す |
| `width` の median / p95 / max | 同上 |
| `nodes` の median / p95 / max | 同上 |
| **`fallback` 率** | `fallback == true` の手数 / 全手数 |
| **`fallback_reason` の内訳** | 理由別の件数 |
| **`depth` のヒストグラム** | 「depth 2 ばかり」を検出するため。中央値だけでは分からない |

**「合成盤面での depth 8」を実戦の数字として書かない。** 実戦の分布を測る(事故では合成盤面 depth 8、実戦 depth 1.9 だった)。

---

## 4. 無効の規則

**次の `suggestion` は無効**。その手を含むゲームは**測定として成立しない**(protocol §8.3.1)。

| 条件 | 理由 |
|---|---|
| `depth` / `width` / `nodes` / `iterations` / `fallback` のいずれかが無い | 探索の有無を検証できない |
| `fallback == true` かつ `fallback_reason` が無い | 原因が分からない |
| `depth == 0` かつ `fallback == false` | 矛盾(完了した反復が無いのに fallback でない) |
| `iterations > 0` かつ `nodes == 0` | 矛盾 |

> **「`depth` を記録していない `suggestion` は無効」** — これは protocol §8.4.1 の要求であり、**レポートの前提条件**。記録が無いゲームの勝率を主張してはいけない。

---

## 5. 受け入れテスト(最低限)

| # | テスト | 期待 |
|---|---|---|
| 1 | 予算を極端に小さくする(1ms) | `fallback == true`、`fallback_reason == "budget"`、`iterations == 0` |
| 2 | 予算を十分に取る(100ms) | `fallback == false`、`iterations >= 2`、`depth >= 2` |
| 3 | 実ゲーム 400 turn | `fallback` 率が記録され、`depth` のヒストグラムが出る |
| 4 | 期限超過が起きる盤面 | **`fallback == false` かつ `iterations >= 1`**(完了した浅い反復を返す) |

**テスト 4 が今回の事故の回帰テスト**。「期限超過 → greedy」に戻ったら落ちる。

---

## 6. 集計の落とし穴(実測より)

| 落とし穴 | 実例 |
|---|---|
| 合成盤面の数字を実戦の数字として書く | 合成で depth 8、実戦で depth 1.9 |
| 中央値だけを見る | 実戦の depth は 1.9〜2.2 で、**中央値が 2 でも「探索している」とは言えない** |
| `think_ms` を探索の証明として使う | 毎手 105ms 消費して 1-ply を返していた |
| `nodes` を合計だけで見る | 1 手だけ巨大な探索をして残りが greedy、という状態を見逃す |

**レポートには「median / p95 / max」と「ヒストグラム」と「fallback 率」を必ず併記する。**

---

## 7. 改訂履歴

| 版 | 日付 | 変更 |
|---|---|---|
| v1 | 2026-11-16 | 初版。`depth` / `width` / `nodes` / `iterations` / `fallback` / `fallback_reason` を定義。`fallback = (iterations == 0)` を厳密化。集計方法・無効条件・受け入れテスト 4 件・落とし穴 4 件。 |
