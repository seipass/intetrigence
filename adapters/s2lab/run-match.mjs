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
import { appendFile, open } from "node:fs/promises";
import { parseArgs } from "node:util";
import { createHash } from "node:crypto";
import { createIntetrigenceDecider } from "./intetrigence-decider.mjs";

const ATTAX_DIR = process.env.ATTAX_DIR ?? "C:/Users/Hasilan/Projects/attax";
const BRIDGE = process.env.S2LAB_BRIDGE ?? "target/release/s2lab_bridge.exe";

const attaxPlayer = await import(`${ATTAX_DIR}/tools/s2-lab/attax-player.ts`);
const labModule = await import(`${ATTAX_DIR}/tools/s2-lab/lab.mjs`);
const { lab, labRoot, parameters, profile } = labModule;
const { createAttaxDecider, cellKey, placementCells } = attaxPlayer;
const { createGame, extendSeededQueue, toS2GuiState } = await lab("cc2-gui/game.mjs");
const { guiStateToCanonical } = await lab("src-js/gui-state.mjs");
const { placementGeometry } = await lab("src-js/triangle/placement-geometry.mjs");
const { generateReachablePlacements } = await lab("src-js/triangle/move-generation.mjs");
const { matchOutcome, ppsForCc2Parameters } = await lab("src-js/bot-match-options.mjs");
const { canonicalize } = await lab("src-js/cs1-core.mjs");
const { controller, applyTransition, createS2AmountOnlyDecisionState, fullStateKey } = labModule;

const { values } = parseArgs({
  options: {
    games: { type: "string", default: "1" },
    seed: { type: "string", default: "9001" },
    "seat-offset": { type: "string", default: "0" },
    "max-locks": { type: "string", default: "1000" },
    "opponent-profile": { type: "string" },
    out: { type: "string" },
  },
});
if (!values.out) throw new Error("--out <file.jsonl> is required");
const games = Number(values.games);
const seed = Number(values.seed);
const maxLocks = Number(values["max-locks"]);
const pps = ppsForCc2Parameters(parameters);
const digest = createHash("sha256").update(canonicalize(profile)).digest("hex");
await (await open(values.out, "a")).close();

const ROTATION = { "0": "spawn", R: "right", "2": "reverse", L: "left" };

/**
 * Find the lab's own placement object for a reply.
 *
 * The two bots report different coordinate frames, so each is matched the way
 * its own adapter defines it: ours carries the occupied cell set (board space,
 * row 0 at the bottom), attax's carries the lab's screen-space cell set via
 * `placementCells`. The witness map is keyed the way attax's runner keys it.
 */
function witnessFor(witnesses, geometry, reply) {
  const key = Array.isArray(reply.cells)
    ? cellSetKey(reply.cells)
    : cellKey(placementCells(reply));
  for (const p of witnesses.values()) {
    const labKey = cellSetKey(
      geometry.blocks[p.piece][p.rotation].map(([x, y]) => [p.x + x, p.y + y]),
    );
    if (labKey === key) return p;
  }
  return null;
}

function cellSetKey(cells) {
  return cells
    .map(([x, y]) => `${x},${y}`)
    .sort()
    .join(";");
}

async function play(gameIndex) {
  const started = performance.now();
  const gameSeed = seed + gameIndex;
  const seatOffset = values["seat-offset"] === "1" ? 1 : 0;
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
  const ours = await createIntetrigenceDecider(BRIDGE);
  const attax = createAttaxDecider(values["opponent-profile"]);
  let outcome = null;
  let error = null;
  let locks = 0;
  let rejectedRoots = 0;
  try {
    while (true) {
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
        for (const p of reachable) {
          const cells = cellKey(
            geometry.blocks[p.piece][p.rotation].map(([x, y]) => [p.x + x, p.y + y]),
          );
          const key = `${p.piece}:${p.usedHold}:${p.rotationEvidence.lastInputWasRotation}:${cells}`;
          if (!witnesses.has(key)) witnesses.set(key, p);
        }
        const witnessKey = (p, usedHold) =>
          `${p.kind}:${usedHold}:${p.lastSrs > 0}:${cellKey(placementCells(p))}`;
        let reply;
        if (side === ourSide) {
          reply = await ours.decide(decision);
        } else {
          // attax's own runner passes this predicate: without it attax can pick
          // a root the lab cannot witness, the game dies as an error, and only
          // attax's games disappear from the record.
          const searched = attax.decide(decision, (p, usedHold) => {
            const accepted = witnesses.has(witnessKey(p, usedHold));
            if (!accepted) rejectedRoots += 1;
            return accepted;
          });
          if (searched === null) throw new Error("attax found no move before top-out");
          reply = {
            kind: searched.best.placement.kind,
            rotation: searched.best.placement.rotation,
            x: searched.best.placement.x,
            y: searched.best.placement.y,
            usedHold: searched.best.usedHold,
            spinEntry: searched.best.placement.lastSrs > 0,
          };
        }
        const placement = witnessFor(witnesses, geometry, reply);
        if (!placement) throw new Error(`no lab witness for ${JSON.stringify(reply)}`);
        const transition = applyTransition(
          bot.state,
          { kind: "placement", placement },
          bot.state.rulesetId,
        );
        if (!transition.legality.legal)
          throw new Error(`illegal placement: ${transition.legality.reason}`);
        receipts[side] = transition;
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
    error = failure instanceof Error ? failure.message : String(failure);
  } finally {
    await ours.close();
  }
  const complete = !error && outcome?.complete === true;
  const winnerName =
    complete && outcome.winnerBotId
      ? outcome.winnerBotId === ourSide
        ? "intetrigence"
        : "attax"
      : null;
  const record = {
    schema: "intetrigence-s2lab/1",
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
    wallMs: performance.now() - started,
    logicalFrames: match.clock.logicalFrame,
    error,
    settings: {
      mode: "paced",
      pps,
      garbageDelayFrames: 0,
      queueModel: "legacy-lcg",
      publicQueueIncludingCurrent: parameters.queueDepth,
      maxLocks,
      labRoot,
      championProfileSha256: digest,
      bridge: BRIDGE,
      attaxProfile: values["opponent-profile"] ?? "DEFAULT_WEIGHTS",
      // The opponent's configuration decides whether the number means anything:
      // the tree, the beam width and whether the stock potential is on.
      attaxTree: process.env.ATTAX_TREE ?? null,
      attaxWidth: attax.width ?? null,
      attaxStockPotential: process.env.ATTAX_S2_STOCK_POTENTIAL ?? "0",
      attaxSolvency: process.env.ATTAX_S2_SOLVENCY ?? "0",
    },
  };
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
}

for (let index = 0; index < games; index++) await play(index);
