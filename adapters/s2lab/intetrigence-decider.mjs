// A s2-bot-lab decider that shells out to the Rust bridge.
//
// The lab calls `decide(decision)` once per lock. One bridge process serves the
// whole match, so what we measure is the search, not process startup.
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createInterface } from "node:readline";

export async function createIntetrigenceDecider(bridgePath, { log = null } = {}) {
  const child = spawn(bridgePath, [], { stdio: ["pipe", "pipe", "inherit"] });
  const lines = createInterface({ input: child.stdout });
  let pending;
  let failure;
  function finish(error, line) {
    const request = pending;
    pending = undefined;
    if (!request) return;
    clearTimeout(request.timer);
    if (error) request.reject(error);
    else request.resolve(line);
  }
  function fail(error) {
    failure ??= error;
    finish(failure);
  }
  child.on("error", fail);
  child.stdin.on("error", fail);
  child.on("exit", (code, signal) => fail(new Error(`bridge exited: ${signal ?? code}`)));
  lines.on("line", (line) => {
    if (pending) finish(undefined, line);
  });
  await once(child, "spawn");

  const ask = (payload) =>
    new Promise((resolve, reject) => {
      if (failure) return reject(failure);
      if (pending) return reject(new Error("a bridge decision is already pending"));
      const timer = setTimeout(() => {
        fail(new Error("bridge decision timed out after 30 seconds"));
        child.kill();
      }, 30_000);
      pending = { resolve, reject, timer };
      child.stdin.write(`${JSON.stringify(payload)}\n`, (error) => {
        if (error) fail(error);
      });
    });

  return {
    profileId: "intetrigence",
    async decide(decision) {
      const reply = await ask(decision);
      const parsed = JSON.parse(reply);
      if (parsed.error) throw new Error(`bridge: ${parsed.error}`);
      if (log) log.push(parsed);
      return parsed;
    },
    async close() {
      lines.close();
      child.stdin.end();
      if (child.exitCode === null && child.signalCode === null) child.kill();
    },
  };
}
