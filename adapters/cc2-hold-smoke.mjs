import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";

const binary = process.argv[2];
if (!binary) throw new Error("usage: node adapters/cc2-hold-smoke.mjs <CC2 executable>");

const child = spawn(binary, [], {
  stdio: ["pipe", "pipe", "inherit"],
  signal: AbortSignal.timeout(15_000),
});
const lines = createInterface({ input: child.stdout });
child.on("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
  lines.close();
});
const messages = lines[Symbol.asyncIterator]();
const send = (message) => child.stdin.write(JSON.stringify(message) + "\n");
async function receive(type) {
  const message = await messages.next();
  assert.ok(!message.done, `CC2 closed before sending ${type}`);
  const result = JSON.parse(message.value);
  assert.equal(result.type, type);
  return result;
}

function start(queue, obstructO) {
  const board = Array.from({ length: 40 }, () => Array(10).fill(null));
  if (obstructO) board[22][5] = "G";
  send({ type: "start", board, queue, hold: null, combo: 0, back_to_back: false });
}

async function choose(piece) {
  for (let elapsed = 0; elapsed < 2000; elapsed += 25) {
    await delay(25);
    send({ type: "suggest" });
    const { moves } = await receive("suggestion");
    const selected = moves.find((move) => move.location.type === piece);
    if (selected) return selected;
  }
  assert.fail(`CC2 did not suggest ${piece}`);
}

function play(move, ...refill) {
  send({ type: "play", move });
  for (const piece of refill) send({ type: "new_piece", piece });
}

try {
  await receive("info");
  send({ type: "rules" });
  await receive("ready");

  start(["O", "I", "T", "L", "S", "J"], true);
  play(await choose("I"), "Z", "I");
  await choose("T");

  start(["J", "O", "I", "T", "L", "S"], true);
  play(await choose("J"), "Z");
  play(await choose("I"), "J", "O");
  await choose("T");

  start(["O"], true);
  await delay(100);
  send({ type: "suggest" });
  assert.deepEqual((await receive("suggestion")).moves, []);
  send({ type: "new_piece", piece: "I" });
  await choose("I");

  start(["I", "I"], false);
  play(await choose("I"));
  await choose("I");
  console.log("CC2 empty hold, two-piece refill, late preview, and same-type queue verified");
} finally {
  if (child.exitCode === null && !child.killed) {
    send({ type: "quit" });
    child.stdin.end();
  }
  lines.close();
}
