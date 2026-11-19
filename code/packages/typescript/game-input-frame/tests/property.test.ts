import assert from "node:assert/strict";
import test from "node:test";

import { SplitMix64, checkProperty } from "@coding-adventures/test-bootstrap";
import { reduce_frame, type ActionChange, type ActionFrame, type InputState } from "../src/index.js";

interface Trace {
  player: number;
  tick: bigint;
  held: bigint;
  axes: number[];
  changes: ActionChange[];
}

const SEED = 0x4750_4c30_3530_61n;

function sampleTrace(rng: SplitMix64): Trace {
  const axes = Array.from({ length: 16 }, () => Number(rng.bounded(65_535)) - 32_767);
  const held = rng.nextU64();
  const changes: ActionChange[] = [];
  const count = rng.bounded(33);
  for (let i = 0; i < count; i += 1) {
    if (rng.bounded(4) === 0) {
      changes.push({
        kind: "axis",
        slot: rng.bounded(16),
        value: Number(rng.bounded(65_535)) - 32_767,
      });
    } else {
      changes.push({ kind: "button", slot: rng.bounded(64), down: rng.bounded(2) === 1 });
    }
  }
  return {
    player: rng.bounded(0x1_0000_0000),
    tick: rng.nextU64(),
    held,
    axes,
    changes,
  };
}

function oracle(trace: Trace): ActionFrame {
  const levels = Array.from({ length: 64 }, (_, slot) => ((trace.held >> BigInt(slot)) & 1n) === 1n);
  const pressed = Array<boolean>(64).fill(false);
  const released = Array<boolean>(64).fill(false);
  const axes = [...trace.axes];
  for (const change of trace.changes) {
    if (change.kind === "button") {
      const wasDown = levels[change.slot];
      if (change.down && !wasDown) {
        levels[change.slot] = true;
        pressed[change.slot] = true;
      } else if (!change.down && wasDown) {
        levels[change.slot] = false;
        released[change.slot] = true;
      }
    } else {
      axes[change.slot] = change.value;
    }
  }
  const mask = (bits: boolean[]): bigint => {
    let value = 0n;
    for (let slot = 0; slot < 64; slot += 1) {
      if (bits[slot]) value |= 1n << BigInt(slot);
    }
    return value;
  };
  return {
    player_id: trace.player,
    tick: trace.tick,
    held_buttons: mask(levels),
    pressed_buttons: mask(pressed),
    released_buttons: mask(released),
    axes,
  };
}

function input(trace: Trace): InputState {
  return { player_id: trace.player, held_buttons: trace.held, axes: trace.axes };
}

function shrinkTrace(initial: Trace, fails: (candidate: Trace) => boolean): [Trace, number] {
  let current: Trace = structuredClone(initial);
  let steps = 0;
  const tryCandidate = (candidate: Trace): void => {
    if (fails(candidate)) {
      current = candidate;
      steps += 1;
    }
  };

  tryCandidate({ ...current, tick: 0n });
  tryCandidate({ ...current, player: 0 });
  tryCandidate({ ...current, axes: Array<number>(16).fill(0) });
  tryCandidate({ ...current, changes: [] });
  for (let slot = 0; slot < 63; slot += 1) {
    tryCandidate({ ...current, held: current.held & ~(1n << BigInt(slot)) });
  }
  return [current, steps];
}

function framesAgree(trace: Trace): boolean {
  const actual = reduce_frame(input(trace), trace.tick, trace.changes);
  return actual.ok && JSON.stringify(encode(actual.value)) === JSON.stringify(encode(oracle(trace)));
}

function encode(frame: ActionFrame): object {
  return {
    player_id: frame.player_id,
    tick: frame.tick.toString(),
    held_buttons: frame.held_buttons.toString(),
    pressed_buttons: frame.pressed_buttons.toString(),
    released_buttons: frame.released_buttons.toString(),
    axes: [...frame.axes],
  };
}

test("seeded valid-biased frame traces agree with an independent Boolean[64]/axis oracle", () => {
  let propertyCalls = 0;
  const verdict = checkProperty(
    SEED,
    { numTests: 1_024, maxShrinkSteps: 1_000 },
    sampleTrace,
    (trace) => {
      propertyCalls += 1;
      return framesAgree(trace);
    },
    shrinkTrace,
  );
  assert.equal(verdict.status, "pass", verdict.status === "fail" ? verdict.message : "");
  assert.equal(propertyCalls, 1_024);
});

test("the trace shrinker preserves a reproducible high-bit mutant failure", () => {
  const initial: Trace = {
    player: 1_234,
    tick: 999n,
    held: (1n << 63n) | 0x1234n,
    axes: Array<number>(16).fill(7),
    changes: [
      { kind: "button", slot: 4, down: false },
      { kind: "axis", slot: 9, value: 20 },
    ],
  };
  const mutantFails = (trace: Trace): boolean => {
    const actual = reduce_frame(input(trace), trace.tick, trace.changes);
    if (!actual.ok) return false;
    const wrong = { ...actual.value, held_buttons: actual.value.held_buttons & ~(1n << 63n) };
    return JSON.stringify(encode(wrong)) !== JSON.stringify(encode(oracle(trace)));
  };
  const [minimal, steps] = shrinkTrace(initial, mutantFails);
  assert.ok(steps > 0);
  assert.deepEqual(minimal.changes, []);
  assert.equal(minimal.player, 0);
  assert.equal(minimal.tick, 0n);
  assert.deepEqual(minimal.axes, Array<number>(16).fill(0));
  assert.equal(minimal.held, 1n << 63n);
  assert.equal(mutantFails(minimal), true);
});
