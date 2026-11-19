import assert from "node:assert/strict";
import test from "node:test";

import { CnfBuilder, negateLit, varOf } from "@coding-adventures/test-bootstrap";
import { MAX_U64, reduce_frame, type ActionChange, type InputState } from "../src/index.js";

type TransitionBits = {
  builder: CnfBuilder;
  mask: number[];
  slot: number;
  downs: number[];
  actual: { held: number[]; pressed: number[]; released: number[] };
  expected: { held: number[]; pressed: number[]; released: number[] };
  mismatch: number;
};

function andAll(builder: CnfBuilder, values: number[]): number {
  return values.reduce((acc, value) => builder.and(acc, value), builder.trueLit());
}

function orAll(builder: CnfBuilder, values: number[]): number {
  return values.reduce((acc, value) => builder.or(acc, value), builder.falseLit());
}

function buildMiter(mutant: "none" | "endpoint-only" | "drop-bit-63", slot: number): TransitionBits {
  const builder = new CnfBuilder();
  const mask = Array.from({ length: 64 }, () => builder.newLit());
  const downs = Array.from({ length: 3 }, () => builder.newLit());
  const slotEquals = Array.from({ length: 64 }, (_, value) =>
    value === slot ? builder.trueLit() : builder.falseLit(),
  );

  // Candidate circuit follows the reducer's set/clear transition algorithm.
  // Each event uses the dynamically selected held level to update edge state.
  let actualHeld = [...mask];
  const actualPressed = Array.from({ length: 64 }, () => builder.falseLit());
  const actualReleased = Array.from({ length: 64 }, () => builder.falseLit());
  for (const down of downs) {
    const wasDown = actualHeld[slot];
    const press = builder.and(down, negateLit(wasDown));
    const release = builder.and(negateLit(down), wasDown);
    actualHeld = actualHeld.map((old, index) => index === slot ? down : old);
    for (let index = 0; index < 64; index += 1) {
      actualPressed[index] = builder.or(actualPressed[index], builder.and(slotEquals[index], press));
      actualReleased[index] = builder.or(actualReleased[index], builder.and(slotEquals[index], release));
    }
  }

  // The reference is an independent Boolean-array oracle circuit, updated one
  // physical action slot at a time without using the dynamic mask selector.
  let expectedHeld = [...mask];
  const expectedPressed = Array.from({ length: 64 }, () => builder.falseLit());
  const expectedReleased = Array.from({ length: 64 }, () => builder.falseLit());
  for (const down of downs) {
    const nextHeld: number[] = [];
    for (let index = 0; index < 64; index += 1) {
      const selected = slotEquals[index];
      const old = expectedHeld[index];
      const press = andAll(builder, [selected, down, negateLit(old)]);
      const release = andAll(builder, [selected, negateLit(down), old]);
      expectedPressed[index] = builder.or(expectedPressed[index], press);
      expectedReleased[index] = builder.or(expectedReleased[index], release);
      nextHeld.push(
        builder.or(builder.and(selected, down), builder.and(negateLit(selected), old)),
      );
    }
    expectedHeld = nextHeld;
  }

  let compared = { held: actualHeld, pressed: actualPressed, released: actualReleased };
  if (mutant === "endpoint-only") {
    compared = {
      held: actualHeld,
      pressed: mask.map((initial, index) => builder.and(negateLit(initial), actualHeld[index])),
      released: mask.map((initial, index) => builder.and(initial, negateLit(actualHeld[index]))),
    };
  } else if (mutant === "drop-bit-63") {
    compared = {
      held: actualHeld.map((bit, index) => index === 63 ? builder.falseLit() : bit),
      pressed: actualPressed.map((bit, index) => index === 63 ? builder.falseLit() : bit),
      released: actualReleased.map((bit, index) => index === 63 ? builder.falseLit() : bit),
    };
  }

  const differences: number[] = [];
  for (let index = 0; index < 64; index += 1) {
    differences.push(
      builder.xor(compared.held[index], expectedHeld[index]),
      builder.xor(compared.pressed[index], expectedPressed[index]),
      builder.xor(compared.released[index], expectedReleased[index]),
    );
  }
  const mismatch = orAll(builder, differences);
  builder.addClause([mismatch]);
  return {
    builder,
    mask,
    slot,
    downs,
    actual: compared,
    expected: { held: expectedHeld, pressed: expectedPressed, released: expectedReleased },
    mismatch,
  };
}

function decode(bits: number[], model: boolean[]): bigint {
  let value = 0n;
  for (let bit = 0; bit < bits.length; bit += 1) {
    if (model[varOf(bits[bit])]) value |= 1n << BigInt(bit);
  }
  return value;
}

function bool(bits: number[], model: boolean[]): boolean[] {
  return bits.map((literal) => model[varOf(literal)]);
}

function expectedFrame(input: InputState, tick: bigint, changes: ActionChange[]) {
  const levels = Array.from(
    { length: 64 },
    (_, index) => ((input.held_buttons >> BigInt(index)) & 1n) === 1n,
  );
  const pressed = Array<boolean>(64).fill(false);
  const released = Array<boolean>(64).fill(false);
  const axes = [...input.axes];
  for (const change of changes) {
    if (change.kind === "button") {
      if (change.down && !levels[change.slot]) {
        levels[change.slot] = true;
        pressed[change.slot] = true;
      } else if (!change.down && levels[change.slot]) {
        levels[change.slot] = false;
        released[change.slot] = true;
      }
    } else {
      axes[change.slot] = change.value;
    }
  }
  const mask = (items: boolean[]): bigint =>
    items.reduce((value, set, index) => set ? value | (1n << BigInt(index)) : value, 0n);
  return {
    player_id: input.player_id,
    tick,
    held_buttons: mask(levels),
    pressed_buttons: mask(pressed),
    released_buttons: mask(released),
    axes,
  };
}

function publicReplay(miter: TransitionBits, mutant: "endpoint-only" | "drop-bit-63"): void {
  const solver = miter.builder.getSolver();
  assert.equal(solver.solve({ maxConflicts: 100_000, maxSteps: 2_000_000 }), "sat");
  const model = solver.getModel();
  const held = decode(miter.mask, model);
  const downs = bool(miter.downs, model);
  const previous: InputState = {
    player_id: 0xffff_ffff,
    held_buttons: held,
    axes: Array<number>(16).fill(0),
  };
  const changes: ActionChange[] = downs.map((down) => ({ kind: "button", slot: miter.slot, down }));
  const result = reduce_frame(previous, MAX_U64, changes);
  assert.equal(result.ok, true, "decoded witness must reach the public reducer");
  if (!result.ok) return;

  const oracle = expectedFrame(previous, MAX_U64, changes);
  assert.deepEqual(
    {
      player_id: result.value.player_id,
      tick: result.value.tick,
      held_buttons: result.value.held_buttons,
      pressed_buttons: result.value.pressed_buttons,
      released_buttons: result.value.released_buttons,
      axes: [...result.value.axes],
    },
    oracle,
  );

  if (mutant === "endpoint-only") {
    const bad = {
      held_buttons: result.value.held_buttons,
      pressed_buttons: result.value.held_buttons & ~held,
      released_buttons: held & ~result.value.held_buttons,
    };
    assert.notDeepEqual(
      [bad.held_buttons, bad.pressed_buttons, bad.released_buttons],
      [oracle.held_buttons, oracle.pressed_buttons, oracle.released_buttons],
    );
  } else {
    const lowMask = (1n << 63n) - 1n;
    const bad = [
      result.value.held_buttons & lowMask,
      result.value.pressed_buttons & lowMask,
      result.value.released_buttons & lowMask,
    ];
    assert.notDeepEqual(bad, [oracle.held_buttons, oracle.pressed_buttons, oracle.released_buttons]);
  }
}

test("bounded 64-slot, three-transition miter proves the Boolean-array model", (t) => {
  let provedSlots = 0;
  for (let slot = 0; slot < 64; slot += 1) {
    const healthy = buildMiter("none", slot);
    assert.equal(
      healthy.builder.getSolver().solve({ maxConflicts: 10_000, maxSteps: 500_000 }),
      "unsat",
      `mismatch is impossible for slot=${slot}`,
    );
    provedSlots += 1;
  }
  assert.equal(provedSlots, 64);
  t.diagnostic(`healthy_solver_queries=${provedSlots} UNSAT; domain=2^64 held masks x 64 fixed slots x 2^3 down sequences; exactly three ordered same-slot transitions; no mixed-slot claim`);
});

test("bounded endpoint-only and high-bit-loss mutants yield SAT public witnesses", (t) => {
  publicReplay(buildMiter("endpoint-only", 0), "endpoint-only");
  publicReplay(buildMiter("drop-bit-63", 63), "drop-bit-63");
  t.diagnostic("solver_queries=2 SAT; public_replays=2; bound=64 mask bits x fixed slot[0..63] x 3 same-slot transitions; replay constants player=u32::MAX, tick=u64::MAX, axes=16 zeroes; endpoint-only and high-bit-loss witnesses replayed through public reducer");
});
