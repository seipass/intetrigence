// A s2-bot-lab decider that shells out to the Rust bridge.
//
// The lab calls `decide(decision)` once per lock. One bridge process serves the
// whole match, so what we measure is the search, not process startup.
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

export async function createIntetrigenceDecider(bridgePath, { log = null } = {}) {
  const child = spawn(bridgePath, [], { stdio: ["pipe", "pipe", "inherit"] });
  const lines = createInterface({ input: child.stdout });
  const pending = [];
  lines.on("line", (line) => {
    const resolve = pending.shift();
    if (resolve) resolve(line);
  });
  let closed = false;
  child.on("exit", () => {
    closed = true;
    while (pending.length) pending.shift()("__closed__");
  });

  const ask = (payload) =>
    new Promise((resolve, reject) => {
      if (closed) return reject(new Error("bridge closed"));
      pending.push(resolve);
      child.stdin.write(`${JSON.stringify(payload)}\n`);
    });

  return {
    profileId: "intetrigence",
    async decide(decision) {
      const reply = await ask(decision);
      if (reply === "__closed__") throw new Error("bridge closed mid-match");
      const parsed = JSON.parse(reply);
      if (parsed.error) throw new Error(`bridge: ${parsed.error}`);
      if (log) log.push(parsed);
      return parsed;
    },
    async close() {
      if (!closed) {
        child.stdin.end();
        child.kill();
      }
    },
  };
}
