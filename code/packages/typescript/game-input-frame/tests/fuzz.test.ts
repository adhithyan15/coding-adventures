import assert from "node:assert/strict";
import test from "node:test";

import { fnv1a64, fuzzTarget, SplitMix64 as FuzzRng, type FuzzOutcome } from "@coding-adventures/fuzz";
import { reduce_frame, type ActionChange, type InputState } from "../src/index.js";

interface Counts {
  calls: number;
  jsonRejected: number;
  boundaryRejected: number;
  semanticRefused: number;
  accepted: number;
}

interface TypedTrace {
  player: number;
  tick: bigint;
  held: bigint;
  axes: number[];
  changes: ActionChange[];
}

interface TypedCounts {
  calls: number;
  buttonEvents: number;
  axisEvents: number;
  highSlotTouches: number;
  sameTickTaps: number;
  emptyFrames: number;
  countBuckets: number[];
}

function typedTrace(bytes: Uint8Array): TypedTrace {
  const rng = new FuzzRng(fnv1a64(bytes));
  const trace: TypedTrace = {
    player: rng.bounded(0x1_0000_0000),
    tick: rng.nextU64(),
    held: rng.nextU64(),
    axes: Array.from({ length: 16 }, () => rng.bounded(65_535) - 32_767),
    changes: [],
  };
  const count = rng.bounded(257);
  for (let i = 0; i < count; i += 1) {
    if (rng.bounded(4) === 0) {
      trace.changes.push({
        kind: "axis",
        slot: rng.bounded(16),
        value: rng.bounded(65_535) - 32_767,
      });
    } else {
      trace.changes.push({ kind: "button", slot: rng.bounded(64), down: rng.bounded(2) === 1 });
    }
  }
  return trace;
}

function typedOracle(trace: TypedTrace) {
  const buttons = Array.from(
    { length: 64 },
    (_, slot) => ((trace.held >> BigInt(slot)) & 1n) === 1n,
  );
  const pressed = Array<boolean>(64).fill(false);
  const released = Array<boolean>(64).fill(false);
  const axes = [...trace.axes];
  for (const change of trace.changes) {
    if (change.kind === "button") {
      if (change.down && !buttons[change.slot]) {
        buttons[change.slot] = true;
        pressed[change.slot] = true;
      } else if (!change.down && buttons[change.slot]) {
        buttons[change.slot] = false;
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
    held_buttons: mask(buttons),
    pressed_buttons: mask(pressed),
    released_buttons: mask(released),
    axes,
  };
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined;
}

function parseU64(value: unknown): bigint | undefined {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) return undefined;
  try {
    const parsed = BigInt(value);
    return parsed <= 0xffff_ffff_ffff_ffffn ? parsed : undefined;
  } catch {
    return undefined;
  }
}

function target(bytes: Uint8Array, coverage: { recordEdge(edge: number): void }, counts: Counts): FuzzOutcome {
  counts.calls += 1;
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown;
  } catch {
    counts.jsonRejected += 1;
    coverage.recordEdge(1);
    return { status: "pass" };
  }

  const root = asRecord(parsed);
  const input = asRecord(root?.input);
  const state = asRecord(input?.state);
  const tick = parseU64(input?.tick);
  const held = parseU64(state?.held_buttons);
  if (
    input === undefined || state === undefined || tick === undefined || held === undefined ||
    !Array.isArray(state.axes) || !Array.isArray(input.changes) ||
    typeof input.player_id !== "number"
  ) {
    counts.boundaryRejected += 1;
    coverage.recordEdge(2);
    return { status: "pass" };
  }

  try {
    const result = reduce_frame(
      {
        player_id: input.player_id,
        held_buttons: held,
        axes: state.axes as number[],
      },
      tick,
      input.changes as ActionChange[],
    );
    if (result.ok) {
      counts.accepted += 1;
      coverage.recordEdge(3);
      if (input.changes.length === 0) coverage.recordEdge(4);
      if (input.changes.some((change) => asRecord(change)?.kind === "button")) coverage.recordEdge(5);
      if (input.changes.some((change) => asRecord(change)?.kind === "axis")) coverage.recordEdge(6);
      if (
        result.value.axes.length !== 16 ||
        result.value.held_buttons < 0n ||
        result.value.held_buttons > 0xffff_ffff_ffff_ffffn ||
        !Object.isFrozen(result.value) ||
        !Object.isFrozen(result.value.axes)
      ) {
        return { status: "crash", kind: "frame-invariant", frames: ["public reducer result"] };
      }
    } else {
      counts.semanticRefused += 1;
      coverage.recordEdge(7);
    }
    return { status: "pass" };
  } catch (error) {
    if (error instanceof TypeError || error instanceof RangeError) {
      counts.boundaryRejected += 1;
      coverage.recordEdge(8);
      return { status: "pass" };
    }
    return {
      status: "crash",
      kind: "unexpected-exception",
      frames: [error instanceof Error ? error.name : typeof error],
    };
  }
}

function rawCase(changes: unknown[], options: Record<string, unknown> = {}): Uint8Array {
  const input = {
    player_id: 7,
    tick: "18446744073709551615",
    state: { held_buttons: "0", axes: Array<number>(16).fill(0) },
    changes,
    ...options,
  };
  return new TextEncoder().encode(JSON.stringify({ input }));
}

function encodeFrame(frame: {
  player_id: number;
  tick: bigint;
  held_buttons: bigint;
  pressed_buttons: bigint;
  released_buttons: bigint;
  axes: readonly number[];
}): object {
  return {
    player_id: frame.player_id,
    tick: frame.tick.toString(),
    held_buttons: frame.held_buttons.toString(),
    pressed_buttons: frame.pressed_buttons.toString(),
    released_buttons: frame.released_buttons.toString(),
    axes: [...frame.axes],
  };
}

function run(initialCorpus: Uint8Array[], seed: bigint): { iterations: number; counts: Counts; failures: number } {
  const counts: Counts = {
    calls: 0,
    jsonRejected: 0,
    boundaryRejected: 0,
    semanticRefused: 0,
    accepted: 0,
  };
  const report = fuzzTarget(initialCorpus, { maxIterations: 256, maxInputLen: 4096, seed }, (bytes, cov) =>
    target(bytes, cov, counts),
  );
  return { iterations: report.iterations, counts, failures: report.crashes.length };
}

test("the existing fuzz engine maps mutations to valid typed traces and checks the independent oracle", (t) => {
  const validSeeds = Array.from({ length: 8 }, (_, i) => new Uint8Array([i, 0x47, 0x50, 0x41, 0x30]));
  const counts: TypedCounts = {
    calls: 0,
    buttonEvents: 0,
    axisEvents: 0,
    highSlotTouches: 0,
    sameTickTaps: 0,
    emptyFrames: 0,
    countBuckets: Array<number>(4).fill(0),
  };
  const report = fuzzTarget(
    validSeeds,
    { maxIterations: 256, maxInputLen: 1024, seed: 0x4755_5a5a_5946_554en },
    (bytes, coverage) => {
      counts.calls += 1;
      const trace = typedTrace(bytes);
      const input: InputState = {
        player_id: trace.player,
        held_buttons: trace.held,
        axes: trace.axes,
      };
      let result;
      try {
        result = reduce_frame(input, trace.tick, trace.changes);
      } catch (error) {
        return {
          status: "crash",
          kind: "valid-generator-binding-error",
          frames: [error instanceof Error ? error.name : typeof error],
        };
      }
      if (!result.ok) return { status: "crash", kind: "valid-generator-refusal", frames: [result.error.code] };
      const expected = typedOracle(trace);
      const actual = {
        player_id: result.value.player_id,
        tick: result.value.tick,
        held_buttons: result.value.held_buttons,
        pressed_buttons: result.value.pressed_buttons,
        released_buttons: result.value.released_buttons,
        axes: [...result.value.axes],
      };
      if (JSON.stringify(encodeFrame(actual)) !== JSON.stringify(encodeFrame(expected))) {
        return { status: "crash", kind: "typed-oracle-mismatch", frames: ["complete frame"] };
      }

      const bucket = trace.changes.length === 0 ? 0 : trace.changes.length <= 8 ? 1 : trace.changes.length <= 64 ? 2 : 3;
      counts.countBuckets[bucket] += 1;
      if (trace.changes.length === 0) counts.emptyFrames += 1;
      for (const change of trace.changes) {
        if (change.kind === "button") {
          counts.buttonEvents += 1;
          if (change.slot === 63) counts.highSlotTouches += 1;
        } else {
          counts.axisEvents += 1;
        }
      }
      if (result.value.pressed_buttons !== 0n && result.value.released_buttons !== 0n) {
        counts.sameTickTaps += 1;
      }
      coverage.recordEdge(bucket);
      coverage.recordEdge(trace.changes.some((change) => change.kind === "button") ? 10 : 11);
      coverage.recordEdge(trace.changes.some((change) => change.kind === "axis") ? 12 : 13);
      coverage.recordEdge((result.value.pressed_buttons !== 0n ? 14 : 0) + (result.value.released_buttons !== 0n ? 15 : 0));
      return { status: "pass" };
    },
  );
  assert.equal(report.crashes.length, 0);
  assert.equal(counts.calls, validSeeds.length + report.iterations);
  assert.equal(report.iterations, 256);
  assert.equal(counts.calls, 264);
  assert.ok(counts.buttonEvents > 0);
  assert.ok(counts.axisEvents > 0);
  assert.ok(counts.highSlotTouches > 0);
  assert.ok(counts.sameTickTaps > 0);
  assert.ok(counts.countBuckets.some((count) => count > 0));
  t.diagnostic(`manual semantic strata: calls=${counts.calls} seeded=${validSeeds.length} mutated=${report.iterations} button_events=${counts.buttonEvents} axis_events=${counts.axisEvents} slot63=${counts.highSlotTouches} same_tick_tap_frames=${counts.sameTickTaps} count_buckets=${counts.countBuckets.join(",")}`);
});

test("a separate malformed JSON-boundary fuzz campaign refuses safely", () => {
  const malformedSeeds = [
    new TextEncoder().encode("{"),
    new TextEncoder().encode("["),
    new TextEncoder().encode("null"),
    new TextEncoder().encode("{\"input\":{\"tick\":\"01\"}}"),
    new Uint8Array([0xff, 0xfe, 0x7b]),
    rawCase([{ kind: "axis", slot: 255, value: -32768 }]),
    rawCase([{ kind: "button", slot: 64, down: true }]),
    rawCase([], { tick: "18446744073709551616" }),
  ];
  const result = run(malformedSeeds, 0x4d41_4c46_4f52_4d45n);
  assert.equal(result.failures, 0);
  assert.equal(result.counts.calls, malformedSeeds.length + result.iterations);
  assert.equal(result.iterations, 256);
  assert.ok(result.counts.jsonRejected + result.counts.boundaryRejected > 0);
  assert.ok(result.counts.semanticRefused > 0);
});
