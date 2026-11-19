import assert from "node:assert/strict";
import test from "node:test";

import { FrameError, reduce_frame, type ActionChange, type InputState } from "../src/index.js";

const state = (overrides: Partial<InputState> = {}): InputState => ({
  player_id: 9,
  held_buttons: 0n,
  axes: Array<number>(16).fill(0),
  ...overrides,
});

test("full-width IDs and ticks preserve zero and negative-zero integer semantics", () => {
  const input = state({ player_id: -0 });
  const result = reduce_frame(input, (1n << 64n) - 1n, []);
  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.value.player_id, 0);
  assert.equal(Object.is(result.value.player_id, -0), false);
  assert.equal(result.value.tick, (1n << 64n) - 1n);

  const maxPlayer = reduce_frame(state({ player_id: 0xffff_ffff }), 0n, []);
  assert.equal(maxPlayer.ok, true);
  if (maxPlayer.ok) assert.equal(maxPlayer.value.player_id, 0xffff_ffff);
});

test("wrong types use TypeError while Number width and integer violations use RangeError", () => {
  assert.throws(() => reduce_frame(null as unknown as InputState, 0n, []), TypeError);
  assert.throws(() => reduce_frame(state(), 0n, {} as unknown as ActionChange[]), TypeError);
  assert.throws(() => reduce_frame(state(), 0 as unknown as bigint, []), TypeError);
  assert.throws(() => reduce_frame(state(), -1n, []), RangeError);
  assert.throws(() => reduce_frame(state(), 1n << 64n, []), RangeError);
  assert.throws(() => reduce_frame(state({ player_id: true as unknown as number }), 0n, []), TypeError);
  assert.throws(() => reduce_frame(state({ player_id: 1.5 }), 0n, []), RangeError);
  assert.throws(() => reduce_frame(state({ player_id: Number.NaN }), 0n, []), RangeError);
  assert.throws(() => reduce_frame(state({ player_id: 0x1_0000_0000 }), 0n, []), RangeError);
  assert.throws(
    () => reduce_frame(state({ held_buttons: 0 as unknown as bigint }), 0n, []),
    TypeError,
  );
  assert.throws(() => reduce_frame(state({ axes: Array(15).fill(0) }), 0n, []), RangeError);
  assert.throws(() => reduce_frame(state({ axes: Array(16).fill(0).map((_, i) => i === 0 ? 0.5 : 0) }), 0n, []), RangeError);

  const hole = Array<number>(16).fill(0);
  delete hole[4];
  assert.throws(() => reduce_frame(state({ axes: hole }), 0n, []), /previous\.axes\[4\]/);
  assert.throws(
    () => reduce_frame(state(), 0n, [{ kind: "button", slot: 0, down: 1 } as unknown as ActionChange]),
    TypeError,
  );
  assert.throws(
    () => reduce_frame(state(), 0n, [{ kind: "axis", slot: 0, value: 32768 }]),
    RangeError,
  );

  const sparseEvents = Array<ActionChange>(1);
  assert.throws(() => reduce_frame(state(), 0n, sparseEvents), /changes\[0\]/);
});

test("batch bound precedes tick, state, and event property reads", () => {
  const previous = Object.defineProperties({}, {
    player_id: { get() { throw new Error("state read before count"); } },
    held_buttons: { get() { throw new Error("state read before count"); } },
    axes: { get() { throw new Error("state read before count"); } },
  }) as InputState;
  const changes = Array.from({ length: 257 }, () => ({
    get kind(): string { throw new Error("event read before count"); },
  })) as unknown as ActionChange[];
  const result = reduce_frame(previous, 0 as unknown as bigint, changes);
  assert.deepEqual(result, { ok: false, error: { code: FrameError.TooManyChanges } });
  assert.equal(Object.isFrozen(result), true);
  if (!result.ok) assert.equal(Object.isFrozen(result.error), true);
});

test("batch count is snapshotted once across proxies and event-side appends", () => {
  const event: Record<string, unknown> = { kind: "button", slot: 2 };
  Object.defineProperty(event, "down", {
    enumerable: true,
    get() {
      changes.push({ kind: "button", slot: 3, down: true });
      return true;
    },
  });
  const changes: ActionChange[] = [event as unknown as ActionChange];
  let lengthReads = 0;
  const reported = new Proxy(changes, {
    get(target, property, receiver) {
      if (property === "length") {
        lengthReads += 1;
        return lengthReads === 1 ? 1 : 257;
      }
      return Reflect.get(target, property, receiver);
    },
  });
  const result = reduce_frame(state(), 1n, reported);
  assert.equal(result.ok, true);
  if (result.ok) {
    assert.equal(result.value.held_buttons, 1n << 2n);
    assert.equal((result.value.held_buttons & (1n << 3n)), 0n);
  }
  assert.equal(lengthReads, 1);
  assert.equal(changes.length, 2, "the event accessor appended an ignored later event");
});

test("invalid proxy-reported batch lengths fail before reading the prior state", () => {
  const previous = new Proxy(state(), {
    get() {
      throw new Error("prior state must not be inspected");
    },
  });
  const changes = new Proxy([] as ActionChange[], {
    get(target, property, receiver) {
      if (property === "length") return 1.5;
      return Reflect.get(target, property, receiver);
    },
  });
  assert.throws(() => reduce_frame(previous, 1n, changes), RangeError);
});

test("event validation is lazy and checks action slots before level/value", () => {
  const invalidAxis = Object.defineProperties({ kind: "axis", slot: 16 }, {
    value: { get() { throw new Error("axis value read before invalid slot refusal"); } },
  }) as unknown as ActionChange;
  assert.deepEqual(reduce_frame(state(), 0n, [invalidAxis]), {
    ok: false,
    error: { code: FrameError.InvalidAxisActionId, change_index: 0, action_id: 16 },
  });

  const invalidButton = Object.defineProperties({ kind: "button", slot: 64 }, {
    down: { get() { throw new Error("button level read before invalid slot refusal"); } },
  }) as unknown as ActionChange;
  assert.deepEqual(reduce_frame(state(), 0n, [invalidButton]), {
    ok: false,
    error: { code: FrameError.InvalidButtonActionId, change_index: 0, action_id: 64 },
  });

  const later = Object.defineProperty({}, "kind", {
    get() { throw new Error("later event read after earlier semantic error"); },
  }) as ActionChange;
  const firstInvalid = [{ kind: "axis", slot: 0, value: -32768 } as const, later];
  assert.deepEqual(reduce_frame(state(), 0n, firstInvalid), {
    ok: false,
    error: {
      code: FrameError.InvalidAxisValue,
      change_index: 0,
      axis_slot: 0,
      value: -32768,
    },
  });
});

test("semantic prior and update failures return exact errors without mutating inputs", () => {
  const axes = Array<number>(16).fill(0);
  axes[7] = -32768;
  const previous = state({ axes });
  const invalidChange = { kind: "button", slot: 63, down: true } as const;
  const before = structuredClone({ previous, changes: [invalidChange] });
  assert.deepEqual(reduce_frame(previous, 2n, [invalidChange]), {
    ok: false,
    error: { code: FrameError.InvalidPreviousAxisValue, axis_slot: 7, value: -32768 },
  });
  assert.deepEqual({ previous, changes: [invalidChange] }, before);
});

test("success records and nested axes are frozen fresh copies", () => {
  const inputAxes = Array<number>(16).fill(0);
  const previous = state({ axes: inputAxes });
  const changes: ActionChange[] = [{ kind: "axis", slot: 2, value: -0 }];
  const result = reduce_frame(previous, 4n, changes);
  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.notEqual(result.value, previous);
  assert.notEqual(result.value.axes, inputAxes);
  assert.equal(Object.isFrozen(result), true);
  assert.equal(Object.isFrozen(result.value), true);
  assert.equal(Object.isFrozen(result.value.axes), true);
  assert.equal(Object.isFrozen(previous), false);
  assert.equal(Object.isFrozen(inputAxes), false);
  assert.equal(result.value.axes[2], 0);
  assert.equal(Object.is(result.value.axes[2], -0), false);
  assert.throws(() => ((result.value.axes as number[])[2] = 7), TypeError);
});
