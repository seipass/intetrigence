#!/usr/bin/env bun
/**
 * clys TBP adapter — standard TBP on the referee side, clys's dialect on the bot side.
 *
 * clys (citrus610/clys, commit ccae45f) speaks TBP, but with three quirks that a
 * standard host cannot satisfy:
 *
 *   1. `new_piece` must arrive *before* the `play` it supplements. clys batches
 *      the pieces received since the previous `play` and passes them to
 *      `ai.advance(piece, new_piece)`; with the standard order (play then
 *      new_piece) its queue shrinks by one piece per turn.
 *   2. `start.randomizer.bag_state` is mandatory (nlohmann `from_json` requires
 *      the field; a missing field throws and the process dies).
 *   3. `stop` is destructive (`ai.request(0); ai.clear();`), so it must not be
 *      forwarded mid-game.
 *
 * The adapter also gives clys a fixed think budget: clys starts its beam search
 * on `start`/`play` and `suggest` joins the current iteration, so the adapter
 * delays `suggest` by `--think-ms` (default 100). Without the delay the search
 * can be pre-root and return an empty plan, on which clys exits with -1.
 *
 * Usage (the referee spawns this):
 *   bun adapters/clys/tbp/adapter.ts --clys-dir <path> [--think-ms 100] [--log <jsonl>]
 *
 * Environment:
 *   CLYS_TBP_LOG   same as --log (JSONL audit log of every message)
 *   CLYS_THINK_MS  same as --think-ms
 *
 * The adapter forwards `info` / `ready` / `suggestion` unchanged. It never
 * rewrites a placement: coordinates are clys's own TBP coordinates, which the
 * referee validates by cell set (`crates/tbp/src/geometry.rs`).
 */
import { spawn } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import readline from "node:readline";

const KINDS = ["I", "O", "T", "S", "Z", "J", "L"] as const;
type Kind = (typeof KINDS)[number];

interface Args {
  clysDir: string;
  thinkMs: number;
  log: string | null;
}

function parseArgs(argv: string[]): Args {
  let clysDir = process.env.CLYS_DIR ?? "";
  let thinkMs = Number(process.env.CLYS_THINK_MS ?? "100");
  let log = process.env.CLYS_TBP_LOG ?? null;
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    const value = argv[i + 1];
    if (flag === "--clys-dir" && value !== undefined) {
      clysDir = value;
      i += 1;
    } else if (flag === "--think-ms" && value !== undefined) {
      thinkMs = Number(value);
      i += 1;
    } else if (flag === "--log" && value !== undefined) {
      log = value;
      i += 1;
    } else if (flag === "--help" || flag === "-h") {
      console.error(
        "usage: bun adapters/clys/tbp/adapter.ts --clys-dir <path> [--think-ms 100] [--log <jsonl>]",
      );
      process.exit(2);
    } else {
      console.error(`unknown argument: ${flag}`);
      process.exit(2);
    }
  }
  if (!clysDir) {
    console.error("--clys-dir is required (path to the clys checkout)");
    process.exit(2);
  }
  if (!Number.isFinite(thinkMs) || thinkMs < 0) {
    console.error("--think-ms must be a non-negative number");
    process.exit(2);
  }
  return { clysDir: path.resolve(clysDir), thinkMs, log };
}

const args = parseArgs(process.argv.slice(2));
const binary = path.join(args.clysDir, "bin", "tbp", "tbp.exe");
if (!existsSync(binary)) {
  console.error(`clys binary not found: ${binary}`);
  console.error("build it with the command in docs/clys-integration.md §1.2");
  process.exit(2);
}
if (args.log) {
  mkdirSync(path.dirname(path.resolve(args.log)), { recursive: true });
}

function log(direction: string, payload: unknown): void {
  if (!args.log) return;
  appendFileSync(
    args.log,
    `${JSON.stringify({ t: new Date().toISOString(), dir: direction, msg: payload })}\n`,
  );
}

// ---- clys child -------------------------------------------------------------------------
const child = spawn(binary, [], {
  cwd: args.clysDir, // clys reads/writes config.json in its cwd
  stdio: ["pipe", "pipe", "pipe"],
  windowsHide: true,
});
child.stderr.on("data", (chunk: Buffer) => {
  const text = chunk.toString();
  log("stderr", text);
  process.stderr.write(text); // the referee captures this
});
child.on("error", (error) => {
  log("adapter-error", String(error));
  console.error(`clys spawn failed: ${error}`);
  process.exit(1);
});

// ---- message plumbing -------------------------------------------------------------------
function sendToClys(message: unknown): void {
  log("to-clys", message);
  child.stdin.write(`${JSON.stringify(message)}\n`);
}
function sendToReferee(message: unknown): void {
  log("to-referee", message);
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

/** Best-effort `bag_state` when the host did not send a randomizer. */
function fallbackBagState(queue: Kind[]): Kind[] {
  // The bag state is "the contents of the bag at the end of the queue". A
  // 7-bag has no duplicates, so the first repeated piece in the queue marks
  // the start of the next bag.
  //   - no repeat: the whole queue was drawn from the current bag, so the
  //     remainder is all seven pieces minus the queue.
  //   - repeat at k: the suffix queue[k..] was drawn from the next bag, so the
  //     remainder is all seven pieces minus that suffix.
  let boundary = -1;
  const seen = new Set<Kind>();
  for (let i = 0; i < queue.length; i += 1) {
    if (seen.has(queue[i])) {
      boundary = i;
      break;
    }
    seen.add(queue[i]);
  }
  const portion = boundary === -1 ? queue : queue.slice(boundary);
  const drawn = new Set(portion);
  const remaining = KINDS.filter((kind) => !drawn.has(kind));
  // A 7-bag never leaves an empty bag at the end of a queue unless the queue
  // ends exactly on a bag boundary; in that case the next bag is full.
  return remaining.length > 0 ? remaining : [...KINDS];
}

function ensureRandomizer(message: Record<string, unknown>): Record<string, unknown> {
  const queue = Array.isArray(message.queue) ? (message.queue as Kind[]) : [];
  const randomizer = message.randomizer as
    | { type?: string; bag_state?: Kind[] }
    | undefined;
  if (randomizer && Array.isArray(randomizer.bag_state) && randomizer.bag_state.length > 0) {
    return message;
  }
  const bagState = fallbackBagState(queue);
  log("adapter-note", {
    note: "start.randomizer was missing or empty; using fallback bag_state",
    queue,
    bag_state: bagState,
  });
  return {
    ...message,
    randomizer: { type: "seven_bag", bag_state: bagState },
  };
}

// Referee -> adapter -> clys
//
// Standard TBP order is `play` then `new_piece`; clys wants the opposite. The
// adapter therefore holds the `play` until the new pieces for that turn have
// arrived, then forwards `new_piece`* followed by `play`. A short settle timer
// catches the second `new_piece` of a first-hold turn; `suggest` (and `quit`)
// flush immediately so the turn never stalls.
const pendingNewPieces: unknown[] = [];
let pendingPlay: unknown | null = null;
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const SETTLE_MS = 5;
/// True once the first `start` has been forwarded. A later `start` (garbage
/// resend) is preceded by a `stop` so clys joins its running search thread.
let started = false;

function flushTurn(): void {
  if (flushTimer !== null) {
    clearTimeout(flushTimer);
    flushTimer = null;
  }
  if (pendingPlay === null) return;
  for (const piece of pendingNewPieces) sendToClys(piece);
  pendingNewPieces.length = 0;
  sendToClys(pendingPlay);
  pendingPlay = null;
}

const stdin = readline.createInterface({ input: process.stdin });
stdin.on("line", (line) => {
  const text = line.trim();
  if (text === "") return;
  let message: Record<string, unknown>;
  try {
    message = JSON.parse(text) as Record<string, unknown>;
  } catch {
    log("adapter-error", { note: "invalid JSON from referee", line: text });
    return;
  }
  log("from-referee", message);
  switch (message.type) {
    case "rules":
      sendToClys(message);
      break;
    case "start":
      // A start resets the game: drop any buffered pieces from the old board.
      pendingNewPieces.length = 0;
      pendingPlay = null;
      if (flushTimer !== null) {
        clearTimeout(flushTimer);
        flushTimer = null;
      }
      // clys's start calls ai.clear() without joining a running search thread;
      // a stop first (request + clear) joins it, so the new search starts clean.
      if (started) sendToClys({ type: "stop" });
      started = true;
      sendToClys(ensureRandomizer(message));
      break;
    case "suggest": {
      // Make sure the previous turn's play (and its new pieces) reached clys.
      flushTurn();
      // Give clys a think budget: its search starts on start/play, and
      // `suggest` joins the current beam iteration.
      if (args.thinkMs > 0) {
        setTimeout(() => sendToClys(message), args.thinkMs);
      } else {
        sendToClys(message);
      }
      break;
    }
    case "play":
      // Hold the play until its new pieces arrive (see the comment above).
      if (pendingPlay !== null) flushTurn();
      pendingPlay = message;
      break;
    case "new_piece":
      pendingNewPieces.push(message);
      if (flushTimer !== null) clearTimeout(flushTimer);
      flushTimer = setTimeout(flushTurn, SETTLE_MS);
      break;
      // clys's stop clears its engine; never forward it mid-game.
      log("adapter-note", { note: "swallowed stop (clys stop is destructive)" });
      break;
    case "quit":
      flushTurn();
      sendToClys(message);
      break;
    default:
      sendToClys(message);
      break;
  }
});
stdin.on("close", () => {
  // The referee closed stdin: ask clys to quit and give it a moment.
  if (child.exitCode === null) {
    try {
      sendToClys({ type: "quit" });
    } catch {
      /* already gone */
    }
  }
});

// clys -> adapter -> referee
const stdout = readline.createInterface({ input: child.stdout });
stdout.on("line", (line) => {
  const text = line.trim();
  if (text === "") return;
  let message: Record<string, unknown>;
  try {
    message = JSON.parse(text) as Record<string, unknown>;
  } catch {
    log("adapter-error", { note: "invalid JSON from clys", line: text });
    return;
  }
  sendToReferee(message);
});

let finished = false;
function finish(code: number): void {
  if (finished) return;
  finished = true;
  log("adapter-exit", { code, childExit: child.exitCode });
  process.exit(code);
}
child.on("exit", (code, signal) => {
  log("clys-exit", { code, signal });
  // Forward clys's exit status; a non-zero exit is a forfeit cause.
  finish(code ?? 1);
});
process.on("SIGTERM", () => {
  try {
    child.kill();
  } catch {
    /* already gone */
  }
  finish(0);
});
