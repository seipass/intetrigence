import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { setTimeout as delay } from "node:timers/promises";

const [binary, ...args] = process.argv.slice(2);
if (!binary) throw new Error("usage: node adapters/tbp-request-window-smoke.mjs <program> [args...]");

const budgetMs = 100;
const child = spawn(binary, args, {
  env: { ...process.env, INTETRIGENCE_TBP_THINK_MS: String(budgetMs) },
  stdio: ["pipe", "pipe", "inherit"],
  signal: AbortSignal.timeout(15_000),
});
const lines = createInterface({ input: child.stdout });
const exited = new Promise((resolve) => child.once("close", (code) => resolve(code)));
child.on("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
  lines.close();
});
const messages = lines[Symbol.asyncIterator]();
function send(message) {
  child.stdin.write(JSON.stringify(message) + "\n");
}
async function receive(type) {
  const message = await messages.next();
  assert.ok(!message.done, `bot closed before sending ${type}`);
  const parsed = JSON.parse(message.value);
  assert.equal(parsed.type, type);
  return parsed;
}

try {
  const info = await receive("info");
  assert.ok(info.features.includes(`intetrigence-request-window-v1:${budgetMs}`));
  send({ type: "rules" });
  await receive("ready");
  const start = {
    type: "start",
    board: Array.from({ length: 40 }, () => Array(10).fill(null)),
    queue: ["T", "L", "S", "Z", "J", "I"],
    hold: null,
    combo: 0,
    back_to_back: false,
    randomizer: { type: "seven_bag", bag_state: ["O"] },
  };
  for (const betweenRequestsMs of [35, 350]) {
    send(start);
    await delay(betweenRequestsMs);
    const requestedAt = performance.now();
    send({ type: "suggest" });
    const suggestion = await receive("suggestion");
    const responseMs = performance.now() - requestedAt;
    assert.ok(suggestion.moves.length > 0 && suggestion.move_info.nodes > 0, "search yielded no work");
    assert.ok(responseMs >= budgetMs - 5, `pre-request search bypassed the budget: ${responseMs} ms`);
    console.log(JSON.stringify({
      bot: info.name,
      betweenRequestsMs,
      responseMs: Math.round(responseMs),
      nodes: suggestion.move_info.nodes,
    }));
    send({ type: "stop" });
  }
} finally {
  if (child.exitCode === null && !child.killed) {
    send({ type: "quit" });
    child.stdin.end();
  }
  lines.close();
}
assert.equal(await exited, 0, "bot did not shut down cleanly");
