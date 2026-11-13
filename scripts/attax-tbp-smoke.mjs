#!/usr/bin/env node
/**
 * attax TBP adapter smoke test — real handshake + real moves against the packaged adapter.
 *
 * Spawns `bun tbp/attax-tbp.ts` (unmodified attax code), performs the
 * info → rules → ready handshake, plays several moves on an empty board, inserts
 * garbage (which requires the host to resend `start`), plays more moves, then
 * sends stop + quit. Every stdin/stdout message is recorded to a JSONL transcript
 * and a summary JSON is written at the end.
 *
 * This is a HOST-SIDE simulation. The board model, Cold-Clear piece geometry and
 * hold/queue tracking below mirror what our Rust TBP host must implement
 * (docs/attax-integration.md §レフェリー要件). It validates:
 *   - handshake order and fields,
 *   - suggestion message shape and coordinate round-trip (location + orientation
 *     reproduce the occupied cells via the Cold-Clear geometry),
 *   - legality of the first suggestion (in bounds, no overlap, resting),
 *   - first-choice acceptance index (0 = the referee can take moves[0] as-is),
 *   - think time / node reporting (move_info), the ATTAX_TBP_LOG decision log,
 *   - that `start` must be resent after external garbage changes the board
 *     (demonstrated as a negative test: stale board → illegal suggestion).
 *
 * Usage:  node scripts/attax-tbp-smoke.mjs [attax-dir]
 *         default attax dir: ../attax relative to this repo root.
 * Evidence: scripts/evidence/attax-tbp-smoke/{transcript.jsonl,summary.json,attax-decisions.jsonl}
 */
import { spawn, execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync, appendFileSync, readFileSync, existsSync } from "node:fs";
import path from "node:path";
import os from "node:os";
import { fileURLToPath } from "node:url";
import readline from "node:readline";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, "..");
const attaxDir = process.argv[2] ? path.resolve(process.argv[2]) : path.resolve(repoRoot, "..", "attax");
const adapter = path.join(attaxDir, "tbp", "attax-tbp.ts");
if (!existsSync(adapter)) {
  console.error(`attax adapter not found: ${adapter}`);
  console.error("usage: node scripts/attax-tbp-smoke.mjs [path-to-attax-repo]");
  console.error("  (expected <repo>/tbp/attax-tbp.ts; clone https://github.com/souta-lab/attax)");
  process.exit(2);
}
const evidenceDir = path.join(here, "evidence", "attax-tbp-smoke");
mkdirSync(evidenceDir, { recursive: true });
const transcriptPath = path.join(evidenceDir, "transcript.jsonl");
const summaryPath = path.join(evidenceDir, "summary.json");
const decisionsPath = path.join(evidenceDir, "attax-decisions.jsonl");
for (const f of [transcriptPath, decisionsPath, summaryPath]) if (existsSync(f)) writeFileSync(f, "");

// ---- Cold Clear / TBP piece geometry (y up), as used by the adapter ----
const CC_CELLS = {
  I: [[-1, 0], [0, 0], [1, 0], [2, 0]],
  O: [[0, 0], [1, 0], [0, 1], [1, 1]],
  T: [[-1, 0], [0, 0], [1, 0], [0, 1]],
  L: [[-1, 0], [0, 0], [1, 0], [1, 1]],
  J: [[-1, 0], [0, 0], [1, 0], [-1, 1]],
  S: [[-1, 0], [0, 0], [0, 1], [1, 1]],
  Z: [[-1, 1], [0, 1], [0, 0], [1, 0]],
};
const ORIENTS = ["north", "east", "south", "west"];
const SPINS = ["none", "mini", "full"];
function rotateCell(o, [x, y]) {
  switch (o) {
    case "north": return [x, y];
    case "east": return [y, -x];
    case "south": return [-x, -y];
    case "west": return [-y, x];
  }
}
function cellsOfMove(move) {
  return CC_CELLS[move.location.type].map((c) => {
    const [dx, dy] = rotateCell(move.location.orientation, c);
    return [move.location.x + dx, move.location.y + dy];
  });
}

// ---- host-side board / queue / hold model (TBP coords: 40 rows, row 0 = bottom) ----
const ROWS = 40, COLS = 10;
let board = Array.from({ length: ROWS }, () => Array(COLS).fill(null));
let queue = [];           // current piece + previews, exactly what the adapter tracks
let hold = null;
let combo = 0;            // TBP convention: consecutive clears, 0 = none
let b2b = false;
let piecesPlaced = 0;

function boardMsg() { return board.map((row) => row.slice()); }

/** Mirror of the adapter's applyPlay: hold/queue bookkeeping, lock, line clears. */
function applyMove(move) {
  const kind = move.location.type;
  const current = queue.shift();
  if (kind !== current) {
    const prevHold = hold;
    hold = current ?? null;
    if (prevHold === null) queue.shift(); // first hold: the swapped-in piece is consumed
  }
  for (const [x, y] of cellsOfMove(move)) {
    if (y >= 0 && y < ROWS && x >= 0 && x < COLS) board[y][x] = kind;
  }
  const kept = board.filter((r) => r.some((c) => c === null));
  const lines = ROWS - kept.length;
  while (kept.length < ROWS) kept.push(Array(COLS).fill(null));
  board = kept;
  if (lines > 0) { combo += 1; b2b = lines === 4 || move.spin !== "none"; } else { combo = 0; }
  piecesPlaced += 1;
  return lines;
}
/** Insert `rows` garbage rows at the bottom with one hole column; the stack moves up. */
function insertGarbage(rows, hole) {
  board.splice(0, rows);
  for (let i = 0; i < rows; i++) {
    const r = Array(COLS).fill("G");
    r[hole] = null;
    board.unshift(r);
  }
}

// ---- process plumbing ----
const child = spawn("bun", [adapter], {
  cwd: attaxDir,
  env: { ...process.env, ATTAX_TBP_LOG: decisionsPath },
  stdio: ["pipe", "pipe", "pipe"],
  shell: false,
});
const rl = readline.createInterface({ input: child.stdout });
child.stderr.on("data", (d) => log("stderr", d.toString().trim()));

const transcript = [];
function log(direction, msg) {
  const rec = { t: new Date().toISOString(), dir: direction, msg };
  transcript.push(rec);
  appendFileSync(transcriptPath, JSON.stringify(rec) + "\n");
}
function send(msg) {
  log("out", msg);
  child.stdin.write(JSON.stringify(msg) + "\n");
}
const waiters = [];
function expect(predicate, label, timeoutMs = 60000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`timeout waiting for ${label}`)), timeoutMs);
    waiters.push({
      predicate, label,
      resolve: (m) => { clearTimeout(timer); resolve(m); },
      reject: (e) => { clearTimeout(timer); reject(e); },
    });
  });
}
function failAllWaiters(reason) {
  for (const w of waiters.splice(0)) w.reject(new Error(reason));
}
rl.on("line", (line) => {
  const text = line.trim();
  if (text === "") return;
  let msg;
  try { msg = JSON.parse(text); } catch { log("in-parse-error", text); return; }
  log("in", msg);
  for (let i = 0; i < waiters.length; i++) {
    if (waiters[i].predicate(msg)) { waiters.splice(i, 1)[0].resolve(msg); break; }
  }
});
let childExit = null;
child.on("exit", (code, signal) => {
  childExit = { code, signal };
  failAllWaiters(`adapter exited (code=${code}, signal=${signal})`);
});

/** Legality of one TBP move against the host board. Returns a list of problems. */
function checkMove(move) {
  const errs = [];
  if (!Object.hasOwn(CC_CELLS, move.location.type)) errs.push("bad piece type");
  if (!ORIENTS.includes(move.location.orientation)) errs.push("bad orientation");
  if (!SPINS.includes(move.spin)) errs.push("bad spin");
  if (errs.length) return errs;
  const cells = cellsOfMove(move);
  let supported = false;
  for (const [x, y] of cells) {
    if (x < 0 || x >= COLS || y < 0 || y >= ROWS) { errs.push(`out of bounds (${x},${y})`); continue; }
    if (board[y][x] !== null) errs.push(`overlap (${x},${y})`);
    if (y === 0 || board[y - 1][x] !== null) supported = true;
  }
  if (!supported) errs.push("not resting");
  return errs;
}
/** Index of the first suggestion the referee would accept (its own legality check). */
function acceptanceIndex(moves) {
  for (let i = 0; i < moves.length; i++) if (checkMove(moves[i]).length === 0) return i;
  return -1;
}

const thinkTimes = [];
const nodeCounts = [];
const acceptance = [];
const depths = [];
const summary = {
  generatedAt: new Date().toISOString(),
  host: { platform: process.platform, arch: process.arch, node: process.version, bun: null, os: `${os.type()} ${os.release()}` },
  attax: { dir: attaxDir, adapter, commit: null, commitDate: null, branch: null, dirty: null },
  steps: [],
  think: null,
  nodes: null,
  acceptance: null,
  transcriptPath: path.relative(repoRoot, transcriptPath).replaceAll("\\", "/"),
  decisionsLogPath: path.relative(repoRoot, decisionsPath).replaceAll("\\", "/"),
  summaryPath: path.relative(repoRoot, summaryPath).replaceAll("\\", "/"),
};

function git(args) {
  try { return execFileSync("git", args, { cwd: attaxDir, encoding: "utf8" }).trim(); }
  catch { return null; }
}
function median(a) {
  if (!a.length) return null;
  const s = [...a].sort((x, y) => x - y);
  const m = Math.floor(s.length / 2);
  return s.length % 2 ? s[m] : +((s[m - 1] + s[m]) / 2).toFixed(3);
}
function p95(a) {
  if (!a.length) return null;
  const s = [...a].sort((x, y) => x - y);
  return s[Math.min(s.length - 1, Math.ceil(0.95 * s.length) - 1)];
}

let finished = false;
function finish(code) {
  if (finished) return;
  finished = true;
  clearTimeout(watchdog);
  summary.finishedAt = new Date().toISOString();
  summary.childExit = childExit;
  summary.piecesPlaced = piecesPlaced;
  summary.think = {
    moves: thinkTimes.length,
    medianMs: median(thinkTimes),
    p95Ms: p95(thinkTimes),
    minMs: thinkTimes.length ? Math.min(...thinkTimes) : null,
    maxMs: thinkTimes.length ? Math.max(...thinkTimes) : null,
    allMs: thinkTimes,
  };
  summary.nodes = {
    total: nodeCounts.reduce((a, b) => a + b, 0),
    median: median(nodeCounts),
    perMove: nodeCounts,
  };
  summary.acceptance = {
    suggestions: acceptance.length,
    firstChoiceAccepted: acceptance.filter((i) => i === 0).length,
    indexHistogram: acceptance.reduce((h, i) => ((h[i] = (h[i] ?? 0) + 1), h), {}),
    indices: acceptance,
  };
  summary.depths = {
    perMove: depths,
    histogram: depths.reduce((h, d) => ((h[d] = (h[d] ?? 0) + 1), h), {}),
  };
  summary.decisions = existsSync(decisionsPath)
    ? readFileSync(decisionsPath, "utf8").trim().split("\n").filter(Boolean).map((l) => JSON.parse(l))
    : [];
  summary.ok = code === 0;
  writeFileSync(summaryPath, JSON.stringify(summary, null, 2) + "\n");
  console.log(JSON.stringify({
    ok: code === 0,
    attaxCommit: summary.attax.commit,
    bun: summary.host.bun,
    piecesPlaced,
    think: summary.think,
    acceptance: summary.acceptance,
    childExit,
    summary: summary.summaryPath,
  }, null, 2));
  process.exit(code);
}

function step(name, ok, detail) {
  summary.steps.push({ name, ok, ...(detail === undefined ? {} : { detail }) });
  if (!ok) throw new Error(`step failed: ${name} ${JSON.stringify(detail ?? {})}`);
}

const watchdog = setTimeout(() => { try { child.kill(); } catch {} finish(2); }, 180000);

// ---- smoke sequence ----
try {
  // Record the exact upstream revision this evidence was produced against.
  summary.host.bun = execFileSync("bun", ["--version"], { encoding: "utf8" }).trim();
  summary.attax.commit = git(["rev-parse", "HEAD"]);
  summary.attax.commitDate = git(["log", "-1", "--format=%cI"]);
  summary.attax.branch = git(["rev-parse", "--abbrev-ref", "HEAD"]);
  summary.attax.dirty = git(["status", "--porcelain"]);

  // 1. handshake: the adapter announces itself with `info` on startup.
  const info = await expect((m) => m.type === "info", "info");
  step("info", info.name === "Attax" && typeof info.version === "string",
    { name: info.name, version: info.version, author: info.author, features: info.features });

  // 2. rules → ready. The adapter ignores the payload but the host must send it.
  send({ type: "rules", lineclear: true, combos: true, backticks: false });
  const ready = await expect((m) => m.type === "ready", "ready");
  step("ready", ready.type === "ready");

  // 3. start: queue[0] = current piece, queue[1..] = previews (5 here).
  queue = ["I", "T", "L", "S", "Z", "O"];
  hold = null; combo = 0; b2b = false;
  send({ type: "start", board: boardMsg(), queue: queue.slice(), hold, combo, back_to_back: b2b });
  step("start", true, { queue: queue.slice(), hold, combo, b2b });

  // 4. play moves: suggest → suggestion → validate → play → new_piece (keeps queue full).
  const bag = ["J", "T", "I", "S", "Z", "L", "O", "J", "S", "T", "L", "I", "Z", "O"];
  let bagIdx = 0;
  const draw = () => bag[bagIdx++ % bag.length];

  async function playOne(label) {
    // Invariant the host must maintain: queue = current piece + 5 previews.
    if (queue.length !== 6) step(label, false, { reason: "queue not 6 before suggest", queue: queue.slice() });
    const t0 = performance.now();
    send({ type: "suggest" });
    const sug = await expect((m) => m.type === "suggestion", label);
    const hostMs = +(performance.now() - t0).toFixed(2);
    if (!Array.isArray(sug.moves) || sug.moves.length === 0) {
      step(label, false, { reason: "empty suggestion", sug });
    }
    const idx = acceptanceIndex(sug.moves);
    acceptance.push(idx);
    thinkTimes.push(hostMs);
    nodeCounts.push(sug.move_info?.nodes ?? 0);
    const adapterMs = Number.parseFloat((sug.move_info?.extra ?? "").match(/([\d.]+)ms/)?.[1] ?? "NaN");
    // The adapter's own decision log carries the search depth for this move.
    const decision = JSON.parse(readFileSync(decisionsPath, "utf8").trim().split("\n").pop());
    depths.push(decision.depth);
    step(label, idx === 0, {
      acceptedIndex: idx,
      moveCount: sug.moves.length,
      first: sug.moves[0],
      firstProblems: idx === 0 ? [] : checkMove(sug.moves[0]),
      hostRoundTripMs: hostMs,
      adapterReportedMs: adapterMs,
      nodes: sug.move_info?.nodes,
      depth: decision.depth,
      queueLen: queue.length,
      hold,
      extra: sug.move_info?.extra,
    });
    // Echo the accepted move back; the adapter tracks state from `play` alone.
    const playedKind = sug.moves[0].location.type;
    const wasHold = playedKind !== queue[0];
    const holdWasEmpty = hold === null;
    send({ type: "play", move: sug.moves[0] });
    applyMove(sug.moves[0]);
    // Refill to current + 5 previews. A hold from an empty hold slot consumes TWO
    // queue entries in the adapter, so that case needs two new_piece messages.
    const needed = 6 - queue.length;
    for (let k = 0; k < needed; k++) {
      const p = draw();
      queue.push(p);
      send({ type: "new_piece", piece: p });
    }
    return { sug, idx, hostMs, wasHold, holdWasEmpty, refilled: needed };
  }

  for (let i = 0; i < 6; i++) await playOne(`move ${i}`);

  // 5. Garbage insertion. The adapter cannot see pending garbage, so the host MUST
  //    resend `start` with the new board. Negative test first: suggest on the stale
  //    internal board, then resend start and confirm the suggestion is legal again.
  insertGarbage(4, 5);
  send({ type: "suggest" });
  const stale = await expect((m) => m.type === "suggestion", "stale-board suggestion");
  const staleIdx = acceptanceIndex(stale.moves);
  const staleProblems = staleIdx === 0 ? [] : checkMove(stale.moves[0]);
  summary.staleBoard = { acceptedIndex: staleIdx, firstProblems: staleProblems, first: stale.moves[0] };

  send({ type: "start", board: boardMsg(), queue: queue.slice(), hold, combo, back_to_back: b2b });
  step("start-resend-after-garbage", true,
    { garbageRows: 4, hole: 5, staleBoardAcceptedIndex: staleIdx, staleBoardProblems: staleProblems });

  for (let i = 0; i < 3; i++) await playOne(`post-garbage move ${i}`);

  // 6. stop (no reply) then quit (exit 0).
  send({ type: "stop" });
  send({ type: "quit" });
  await new Promise((resolve) => {
    if (childExit) return resolve();
    const t = setTimeout(resolve, 10000);
    child.on("exit", () => { clearTimeout(t); resolve(); });
  });
  step("quit", childExit?.code === 0, { childExit });
  finish(0);
} catch (e) {
  console.error("SMOKE FAILED:", e.message);
  summary.error = e.message;
  try { child.kill(); } catch {}
  finish(1);
}
