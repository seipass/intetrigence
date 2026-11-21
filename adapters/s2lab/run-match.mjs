// intetrigence vs attax inside 61bi's s2-bot-lab.
//
// Adapted from attax's `tools/s2-lab/run-match.mjs` (AGPL-3.0, the same licence
// as this repository). The lab is the only interface that carries the full S2
// state to both players: the exact `b2b` chain, the arrived and total garbage
// rows and the logical frame. TBP carries none of that, so a TBP-connected S2
// attax plays with a handicap (souta-lab/attax issue "s2-bot-lab 対応しろ").
//
//   ATTAX_DIR=/path/to/attax S2_LAB_ROOT=/path/to/s2-bot-lab-ref \
//     bun adapters/s2lab/run-match.mjs --games 1 --seed 9001 --out /tmp/x.jsonl
import { appendFile, open, readFile, readdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { isDeepStrictEqual, parseArgs } from "node:util";
import { createHash } from "node:crypto";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createIntetrigenceDecider } from "./intetrigence-decider.mjs";
import { acquireMeasurement } from "./measurement-lock.mjs";
import { labSeedClaims } from "./evaluation-seeds.mjs";

if (!process.env.ATTAX_DIR || !process.env.S2_LAB_ROOT) {
  throw new Error("ATTAX_DIR and S2_LAB_ROOT are required");
}
const ATTAX_DIR = resolve(process.env.ATTAX_DIR);
const exe = process.platform === "win32" ? ".exe" : "";
const BRIDGE = resolve(process.env.S2LAB_BRIDGE ?? `target/release/s2lab_bridge${exe}`);
const attaxRustBuild = "c83d3397d458f4af9cfd6fe3ba3468df48bb2877";
const attaxWidth = 96;
const git = (...args) => execFileSync("git", ["-C", ATTAX_DIR, ...args], { encoding: "utf8" }).trim();

const { values } = parseArgs({
  options: {
    games: { type: "string", default: "1" },
    seed: { type: "string", default: "9001" },
    "seat-offset": { type: "string", default: "0" },
    "max-locks": { type: "string", default: "2000" },
    "first-to": { type: "string" },
    "attax-engine": { type: "string", default: "auto" },
    evaluation: { type: "boolean" },
    resume: { type: "boolean" },
    verify: { type: "boolean" },
    "recorded-runner": { type: "string" },
    out: { type: "string" },
  },
});
if (!values.out) throw new Error("--out <file.jsonl> is required");
if (!["auto", "gpu", "rust"].includes(values["attax-engine"])) {
  throw new Error(`invalid --attax-engine: ${values["attax-engine"]} (expected auto, gpu, or rust)`);
}

const hasGpu = existsSync(resolve(ATTAX_DIR, "tools/s2-lab/gpu-player.ts"));
const hasRust = existsSync(resolve(ATTAX_DIR, "tools/s2-lab/attax-player.ts"));
const attaxMode =
  values["attax-engine"] === "auto"
    ? hasGpu && !hasRust
      ? "gpu"
      : hasRust
        ? "rust"
        : (() => { throw new Error(`no supported s2-lab player found in ${ATTAX_DIR}`); })()
    : values["attax-engine"];

process.env.ATTAX_S2_STOCK_POTENTIAL = "1";
process.env.ATTAX_S2_SOLVENCY = "0";

const labModule = await import(pathToFileURL(resolve(ATTAX_DIR, "tools/s2-lab/lab.mjs")).href);
const { lab, labRoot, parameters, profile } = labModule;
const { createGame, extendSeededQueue, toS2GuiState } = await lab("cc2-gui/game.mjs");
const { guiStateToCanonical } = await lab("src-js/gui-state.mjs");
const { placementGeometry } = await lab("src-js/triangle/placement-geometry.mjs");
const { generateReachablePlacements } = await lab("src-js/triangle/move-generation.mjs");
const { matchOutcome, ppsForCc2Parameters } = await lab("src-js/bot-match-options.mjs");
const { canonicalize } = await lab("src-js/cs1-core.mjs");
const { controller, applyTransition, createS2AmountOnlyDecisionState, fullStateKey } = labModule;

const sha256 = async (file) => createHash("sha256").update(await readFile(file)).digest("hex");

let createAttax;
let closeGpuRuntime = () => {};
let cellKey;
let placementCells;
let identity;
let attaxSha256;
let attaxShadersSha256 = null;

if (attaxMode === "rust") {
  if (!hasRust) throw new Error(`attax-player.ts not found in ${ATTAX_DIR}`);
  if (git("rev-parse", "HEAD") !== attaxRustBuild || git("status", "--porcelain")) {
    throw new Error(`ATTAX_DIR must be a clean checkout of ${attaxRustBuild}`);
  }
  process.env.ATTAX_RUST_CLI ??= resolve(ATTAX_DIR, `rust/target/release/attax-agree${exe}`);
  process.env.ATTAX_ENGINE = "rust";
  process.env.ATTAX_RUST_MODE = "server";
  const attaxPlayer = await import(pathToFileURL(resolve(ATTAX_DIR, "tools/s2-lab/attax-player.ts")).href);
  const { createAttaxDecider, engineIdentity } = attaxPlayer;
  cellKey = attaxPlayer.cellKey;
  placementCells = attaxPlayer.placementCells;
  identity = engineIdentity();
  if (identity.id !== "rust" || identity.build !== attaxRustBuild) {
    throw new Error(`expected Attax Rust ${attaxRustBuild}, got ${JSON.stringify(identity)}`);
  }
  attaxSha256 = await sha256(process.env.ATTAX_RUST_CLI);
  createAttax = () => createAttaxDecider(undefined, { width: attaxWidth });
} else {
  if (!hasGpu) throw new Error(`gpu-player.ts not found in ${ATTAX_DIR}`);
  if (git("status", "--porcelain", "--untracked-files=no")) {
    throw new Error(`ATTAX_DIR (${ATTAX_DIR}) has uncommitted modifications`);
  }
  const attaxHead = git("rev-parse", "HEAD");
  const attaxPkg = JSON.parse(await readFile(resolve(ATTAX_DIR, "package.json"), "utf8"));
  const gpuPlayer = await import(pathToFileURL(resolve(ATTAX_DIR, "tools/s2-lab/gpu-player.ts")).href);
  const labDecision = await import(pathToFileURL(resolve(ATTAX_DIR, "tools/s2-lab/lab-decision.ts")).href);
  cellKey = labDecision.cellKey;
  placementCells = labDecision.placementCells;
  const gpuInfo = await gpuPlayer.gpuEngineInfo();
  closeGpuRuntime = () => gpuPlayer.closeGpuEngine();
  if (values.verify) closeGpuRuntime();
  attaxSha256 = await sha256(gpuInfo.runnerPath);
  const shaderHash = createHash("sha256");
  for (const rel of [
    "gpu/shaders/bpmg_b1.spv",
    "gpu/shaders/expand_s25_noba.spv",
    "gpu/shaders/MANIFEST.sha256",
  ]) {
    shaderHash.update(`${rel}\0${await sha256(resolve(ATTAX_DIR, rel))}\0`);
  }
  attaxShadersSha256 = shaderHash.digest("hex");
  identity = {
    id: "gpu",
    build: attaxHead,
    binary: basename(gpuInfo.runnerPath),
    version: String(attaxPkg.version ?? "unknown"),
    device: gpuInfo.device,
    tablesSha256: gpuInfo.tablesSha256,
    ...(process.env.ATTAX_GPU_RUNNER ? { runnerOverride: true } : {}),
  };
  createAttax = async () => labDecision.wrapGpuDecider(await gpuPlayer.createDefaultGpuDecider());
}
function integer(value, name, minimum, maximum = Number.MAX_SAFE_INTEGER) {
  const number = Number(value);
  if (!/^\d+$/.test(value) || !Number.isSafeInteger(number) || number < minimum || number > maximum) {
    throw new Error(`invalid ${name}: ${value}`);
  }
  return number;
}
const games = integer(values.games, "--games", 1);
const seed = integer(values.seed, "--seed", 1, 0x7fffffff);
const maxLocks = integer(values["max-locks"], "--max-locks", 2, 100_000);
const firstTo = values["first-to"] === undefined ? null : integer(values["first-to"], "--first-to", 1);
const evaluation = values.evaluation === true;
if (evaluation && (games !== 100 || firstTo !== null)) {
  throw new Error("--evaluation requires --games 100 and no --first-to");
}
if (evaluation && seed + games - 1 > 0x7fffffff) {
  throw new Error("evaluation seed range exhausted");
}
if (values.resume && values.verify) throw new Error("--resume and --verify are mutually exclusive");
if (values["recorded-runner"] && !values.verify) throw new Error("--recorded-runner requires --verify");
const seatOffset = integer(values["seat-offset"], "--seat-offset", 0, 1);
for (const [key, fallback, max] of [
  ["S2LAB_DEPTH", "12", 40],
  ["S2LAB_WIDTH", "96", 10_000],
  ["S2LAB_THREADS", "8", 8],
]) {
  process.env[key] = String(integer(process.env[key] ?? fallback, key, 1, max));
}
const pps = ppsForCc2Parameters(parameters);
const projectGarbage = integer(process.env.S2LAB_PROJECT_GARBAGE ?? "2", "S2LAB_PROJECT_GARBAGE", 0, 2);
process.env.S2LAB_PROJECT_GARBAGE = String(projectGarbage);
const attackProfile = integer(process.env.S2LAB_ATTACK_PROFILE ?? "6", "S2LAB_ATTACK_PROFILE", 0, 6);
process.env.S2LAB_ATTACK_PROFILE = String(attackProfile);
const useGpu = integer(process.env.S2LAB_GPU ?? "0", "S2LAB_GPU", 0, 1);
process.env.S2LAB_GPU = String(useGpu);
const framesPerLock = 60 / pps;
if (!Number.isSafeInteger(framesPerLock) || framesPerLock < 1) {
  throw new Error(`unsupported fixed Lab lock cadence: ${pps} PPS`);
}
const digest = createHash("sha256").update(canonicalize(profile)).digest("hex");
const bridgeSha256 = await sha256(BRIDGE);
async function hashLabSources() {
  const hash = createHash("sha256");
  async function visit(path) {
    const dir = await readdir(join(labRoot, path), { withFileTypes: true });
    for (const entry of dir.sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
      if (entry.name === "node_modules" || entry.name.startsWith(".")) continue;
      const relative = `${path}/${entry.name}`;
      if (entry.isDirectory()) await visit(relative);
      else if (entry.isFile()) hash.update(`${relative}\0${await sha256(join(labRoot, relative))}\0`);
    }
  }
  for (const path of ["src-js", "cc2-gui", "rulesets", "schema"]) await visit(path);
  return hash.digest("hex");
}
const settings = {
  mode: "paced",
  pps,
  garbageDelayFrames: 0,
  queueModel: "legacy-lcg",
  publicQueueIncludingCurrent: parameters.queueDepth,
  maxLocks,
  firstTo,
  ...(evaluation ? { plannedGames: games } : {}),
  startSeed: seed,
  seatOffset,
  labSourceSha256: await hashLabSources(),
  championProfileSha256: digest,
  runnerSha256: await sha256(values["recorded-runner"] ?? new URL(import.meta.url)),
  deciderSha256: await sha256(new URL("./intetrigence-decider.mjs", import.meta.url)),
  bridgeSha256,
  intetrigenceSearch: {
    depth: Number(process.env.S2LAB_DEPTH),
    width: Number(process.env.S2LAB_WIDTH),
    threads: Number(process.env.S2LAB_THREADS),
    ...(projectGarbage ? { projectGarbage: projectGarbage === 1 ? true : "split-burst" } : {}),
    ...(attackProfile ? { attackProfile } : {}),
    ...(useGpu ? { gpu: true } : {}),
  },
  attaxEngine: identity,
  attaxSha256,
  ...(attaxShadersSha256 ? { attaxShadersSha256 } : {}),
  attaxWidth,
  attaxStockPotential: true,
  attaxSolvency: false,
};

function checkRecorded(actual, expected, label) {
  // Compare the durable JSON representation (timings may have more decimal
  // places than the Lab's CS1 encoding allows).
  const persisted = JSON.parse(JSON.stringify(actual));
  if (!isDeepStrictEqual(persisted, expected)) throw new Error(`replay mismatch: ${label}`);
}

function witnessKey({ kind, usedHold, spinEntry, cells }) {
  return `${kind}:${usedHold}:${spinEntry}:${cellKey(cells)}`;
}

async function play(gameIndex, expected) {
  const started = performance.now();
  const gameSeed = seed + gameIndex;
  const ourSide = (gameIndex + seatOffset) % 2 === 0 ? "left" : "right";
  const scenario = createGame(gameSeed);
  const initial = guiStateToCanonical(toS2GuiState(scenario));
  const rules = labModule.resolvePlacementRules(initial.rulesetId);
  const geometry = placementGeometry(rules);
  let match = controller.createBotMatch({
    bots: ["left", "right"].map((id, i) => ({ id, gameId: i + 1, state: initial })),
    mode: "paced",
    ppsByBotId: { left: pps, right: pps },
    garbageDelayFrames: 0,
  });
  const queueSeeds = { left: scenario.bagSeed, right: scenario.bagSeed };
  const lastTankTopout = { left: false, right: false };
  const receipts = { left: null, right: null };
  const trace = [];
  const received = { left: 0, right: 0 };
  let ours;
  let attax;
  let outcome = null;
  let error = null;
  let locks = 0;
  let rejectedRoots = 0;
  try {
    if (!expected) {
      ours = await createIntetrigenceDecider(BRIDGE);
      attax = await createAttax();
    }
    loop: while (true) {
      outcome = matchOutcome(
        match.bots.map((b) => ({
          id: b.id,
          toppedOut: lastTankTopout[b.id] || /[^_]/.test(b.state.board.cells.slice(200)),
        })),
        match.turnNumber,
        null,
      );
      if (outcome.complete || locks >= maxLocks) break;
      for (const bot of match.bots) {
        const queue = [bot.state.pieces.current, ...bot.state.pieces.known];
        const extended = extendSeededQueue(queue, queueSeeds[bot.id], 28);
        queueSeeds[bot.id] = extended.bagSeed;
        match = controller.extendBotMatchQueue(match, bot.id, extended.queue);
      }
      const due = controller.botMatchNextStep(match);
      if (locks + due.botIds.length > maxLocks) break;
      const submissions = [];
      for (const side of due.botIds) {
        const bot = match.bots.find((b) => b.id === side);
        const visible = {
          ...bot.state,
          pieces: {
            ...bot.state.pieces,
            known: bot.state.pieces.known.slice(0, parameters.queueDepth - 1),
          },
        };
        const decision = createS2AmountOnlyDecisionState(visible);
        const reachable = generateReachablePlacements(visible, {
          kickTable: rules.kickTable,
          allow180: true,
        });
        const witnesses = new Map();
        const legalRoots = [];
        for (const p of reachable) {
          const root = {
            kind: p.piece,
            usedHold: p.usedHold,
            spinEntry: p.rotationEvidence.lastInputWasRotation,
            cells: geometry.blocks[p.piece][p.rotation].map(([x, y]) => [p.x + x, p.y + y]),
          };
          const key = witnessKey(root);
          if (!witnesses.has(key)) {
            witnesses.set(key, p);
            legalRoots.push(root);
          }
        }
        const attaxRoot = (p, usedHold) => ({
          kind: p.kind, usedHold, spinEntry: p.lastSrs > 0, cells: placementCells(p),
        });
        const thinkStart = performance.now();
        let reply;
        const recorded = expected?.trace[trace.length];
        if (expected) {
          if (!recorded) {
            if (expected.error) {
              error = expected.error;
              break loop;
            }
            throw new Error("replay trace ended early");
          }
          checkRecorded({ side, decision }, { side: recorded.side, decision: recorded.decision }, "decision");
          reply = recorded.reply;
        } else if (side === ourSide) {
          reply = await ours.decide({
            ...decision,
            framesPerLock,
            ownTotals: { sent: bot.stats.garbageSent, received: received[side] },
            legalRoots,
          });
        } else {
          // attax's own runner passes this predicate: without it attax can pick
          // a root the lab cannot witness, the game dies as an error, and only
          // attax's games disappear from the record.
          const searched = await attax.decideAsync(decision, (p, usedHold) => {
            const accepted = witnesses.has(witnessKey(attaxRoot(p, usedHold)));
            if (!accepted) rejectedRoots += 1;
            return accepted;
          });
          if (searched === null) throw new Error("attax found no move before top-out");
          reply = attaxRoot(searched.best.placement, searched.best.usedHold);
        }
        const thinkMs = recorded?.thinkMs ?? (performance.now() - thinkStart);
        const placement = witnesses.get(witnessKey(reply));
        if (!placement) throw new Error(`no lab witness for ${JSON.stringify(reply)}`);
        const transition = applyTransition(
          bot.state,
          { kind: "placement", placement },
          bot.state.rulesetId,
        );
        if (!transition.legality.legal)
          throw new Error(`illegal placement: ${transition.legality.reason}`);
        receipts[side] = transition;
        const tankedRows = transition.tankResult?.inserted?.reduce((n, packet) => n + packet.amount, 0) ?? 0;
        received[side] += tankedRows;
        trace.push({
          side, decision, reply, placement, thinkMs,
          lock: transition.lockResult, attack: transition.attackStages, tankedRows,
        });
        lastTankTopout[side] = transition.tankResult?.toppedOut === true;
        submissions.push({
          botId: side,
          result: { transition, comparison: { positionFingerprint: fullStateKey(bot.state) } },
        });
      }
      match = controller.advanceBotMatch(match, submissions);
      locks += submissions.length;
    }
  } catch (failure) {
    if (expected) throw failure;
    error = failure instanceof Error ? failure.message : String(failure);
  } finally {
    await ours?.close();
    attax?.close();
  }
  const complete = !error && outcome?.complete === true;
  const winnerName =
    complete && outcome.winnerBotId
      ? outcome.winnerBotId === ourSide
        ? "intetrigence"
        : "attax"
      : null;
  const record = {
    schema: "intetrigence-s2lab/2",
    seed: gameSeed,
    ourSide,
    contestants: {
      left: ourSide === "left" ? "intetrigence" : "attax",
      right: ourSide === "right" ? "intetrigence" : "attax",
    },
    status: error ? "error" : complete ? "finished" : "unfinished",
    winner: complete ? outcome.winnerBotId : null,
    winnerName,
    reason: error ? "harness-error" : complete ? outcome.reason : "max-locks",
    locks: Object.fromEntries(match.bots.map((b) => [b.id, b.stats.turns])),
    attack: Object.fromEntries(match.bots.map((b) => [b.id, b.stats.attack])),
    stats: Object.fromEntries(match.bots.map((b) => [b.id, b.stats])),
    finalReceipts: Object.fromEntries(
      Object.entries(receipts).map(([side, t]) => [
        side,
        t === null
          ? null
          : {
              lock: t.lockResult,
              attack: t.attackStages,
              cancel: t.cancelResult,
              tankedRows: t.tankResult?.inserted?.reduce((n, p) => n + p.amount, 0) ?? 0,
            },
      ]),
    ),
    rejectedRoots,
    trace,
    wallMs: performance.now() - started,
    logicalFrames: match.clock.logicalFrame,
    error,
    settings,
    attaxProfile: attax?.profileId ?? (attaxMode === "gpu" ? "config-o" : null),
  };
  if (expected) {
    for (const field of ["winner", "winnerName", "status", "reason", "error", "locks", "stats", "logicalFrames", "trace"]) {
      checkRecorded(record[field], expected[field], field);
    }
    return record;
  }
  await appendFile(values.out, JSON.stringify(record) + "\n");
  console.log(
    JSON.stringify({
      seed: record.seed,
      status: record.status,
      winner: record.winnerName,
      locks: record.locks,
      error,
    }),
  );
  return record;
}

const releaseMeasurement = values.verify ? null : await acquireMeasurement("Attax Lab");
try {
  const seedClaims = evaluation || (!values.verify && firstTo !== null)
    ? await labSeedClaims(values.out, settings)
    : null;
  if (evaluation && !values.verify) {
    // Check the whole range before accepting any result. Reserve it before
    // play so a canceled evaluation cannot quietly reuse unplayed seeds.
    for (let offset = 0; offset < games; offset++) {
      seedClaims.check(seed + offset, values.resume);
    }
  } else if (!values.resume && !values.verify) {
    seedClaims?.check(seed, false);
  }
  if (!values.resume && !values.verify) await (await open(values.out, "wx")).close();
  const score = { intetrigence: 0, attax: 0 };
  let errors = 0;
  let index = 0;
  const isTolerableGpuError = (record) =>
    attaxMode === "gpu" && record.error === "attax found no move before top-out";
  if (values.resume || values.verify) {
    const lines = (await readFile(values.out, "utf8")).split(/\r?\n/).filter(Boolean);
    if (values.verify && !lines.length) throw new Error("no recorded games");
    if (evaluation && (lines.length > games || (values.verify && lines.length !== games))) {
      throw new Error(`expected ${games} evaluation games, found ${lines.length}`);
    }
    for (const line of lines) {
      const record = JSON.parse(line);
      if (record.seed !== seed + index || record.schema !== "intetrigence-s2lab/2" || (record.error && !isTolerableGpuError(record))) {
        throw new Error(`invalid or failed replay at index ${index}`);
      }
      checkRecorded(settings, record.settings, "match settings");
      if (seedClaims && !seedClaims.check(record.seed, true)) {
        throw new Error(`recorded evaluation seed ${record.seed} has no original claim`);
      }
      await play(index++, record);
      if (record.error) errors++;
      else if (record.winnerName) score[record.winnerName]++;
    }
    const decided = score.intetrigence + score.attax;
    console.log(JSON.stringify({
      verified: index,
      errors,
      score,
      ...(evaluation && values.verify ? { plannedGames: games, decidedGames: decided, winRate: score.intetrigence / games } : {}),
    }));
  }
  if (evaluation && !values.verify) {
    for (let offset = 0; offset < games; offset++) {
      await seedClaims.claim(seed + offset, values.resume);
    }
  }
  for (; !values.verify && (firstTo === null ? index < games : Math.max(...Object.values(score)) < firstTo); index++) {
    if (seed + index > 0x7fffffff) throw new Error("seed range exhausted");
    if (!evaluation) await seedClaims?.claim(seed + index, values.resume);
    const record = await play(index);
    if (record.error) {
      if (!isTolerableGpuError(record)) throw new Error(`seed ${record.seed}: ${record.error}`);
      errors++;
    } else if (record.winnerName) {
      score[record.winnerName]++;
    }
    console.log(JSON.stringify({ score, errors, attempted: index + 1, firstTo }));
  }
} finally {
  closeGpuRuntime();
  await releaseMeasurement?.();
}
