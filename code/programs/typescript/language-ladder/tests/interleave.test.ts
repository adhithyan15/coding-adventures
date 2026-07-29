import { describe, it, expect } from "vitest";
import { buildPool, poolSize } from "../src/interleave.ts";
import { initStates, pickNext, reviewIn } from "../src/scheduler.ts";
import { SCRIPTS } from "../src/data.ts";

describe("buildPool", () => {
  it("interleaves scripts round-robin and includes every (script, letter) once", () => {
    const pool = buildPool([2, 3]);
    expect(pool).toEqual([
      { scriptIndex: 0, letterIndex: 0 },
      { scriptIndex: 1, letterIndex: 0 },
      { scriptIndex: 0, letterIndex: 1 },
      { scriptIndex: 1, letterIndex: 1 },
      { scriptIndex: 1, letterIndex: 2 }, // script 1's extra letter, ragged tail
    ]);
    expect(pool).toHaveLength(poolSize([2, 3]));
  });

  it("handles equal counts, a single script, and empties", () => {
    expect(buildPool([2, 2]).map((p) => `${p.scriptIndex}:${p.letterIndex}`)).toEqual([
      "0:0", "1:0", "0:1", "1:1",
    ]);
    expect(buildPool([3])).toEqual([
      { scriptIndex: 0, letterIndex: 0 },
      { scriptIndex: 0, letterIndex: 1 },
      { scriptIndex: 0, letterIndex: 2 },
    ]);
    expect(buildPool([])).toEqual([]);
    expect(buildPool([0, 0])).toEqual([]);
  });
});

describe("poolSize", () => {
  it("sums letter counts, ignoring negatives", () => {
    expect(poolSize([2, 3, 5])).toBe(10);
    expect(poolSize([])).toBe(0);
    expect(poolSize([-1, 4])).toBe(4);
  });
});

describe("scheduler over the combined pool interleaves scripts", () => {
  it("moves to a different script's letter after answering one correctly", () => {
    const pool = buildPool([2, 2]); // [s0l0, s1l0, s0l1, s1l1]
    let states = initStates(pool.length); // all due at session 0
    let tick = 0;

    const first = pickNext(states, tick);
    expect(pool[first]).toEqual({ scriptIndex: 0, letterIndex: 0 }); // lowest index

    states = reviewIn(states, first, true, tick); // got it right → scheduled later
    tick += 1;

    const second = pickNext(states, tick);
    // the next most-overdue is pool[1] = a DIFFERENT script → interleaved
    expect(pool[second]!.scriptIndex).not.toBe(pool[first]!.scriptIndex);
    expect(pool[second]).toEqual({ scriptIndex: 1, letterIndex: 0 });
  });

  it("a missed letter from one script resurfaces amid the others", () => {
    const pool = buildPool([2, 2]); // [s0l0, s1l0, s0l1, s1l1]
    let states = initStates(pool.length);
    // Master items 0, 1, 3 across several ascending sessions so their next-due
    // drifts well into the future; miss item 2 (script 0, letter 1).
    for (const s of [0, 1, 4]) {
      for (const i of [0, 1, 3]) states = reviewIn(states, i, true, s);
    }
    states = reviewIn(states, 2, false, 4); // wrong → due again at session 5
    // At session 5 only the missed item is due; the mastered ones are far out.
    const next = pickNext(states, 5);
    expect(pool[next]).toEqual({ scriptIndex: 0, letterIndex: 1 });
  });
});

describe("with real script data", () => {
  it("builds a pool spanning all shipped scripts", () => {
    const counts = SCRIPTS.map((s) => s.letters.length);
    const pool = buildPool(counts);
    expect(pool).toHaveLength(poolSize(counts));
    // the pool touches every script
    expect(new Set(pool.map((p) => p.scriptIndex)).size).toBe(SCRIPTS.length);
    // and the first few entries alternate scripts (round-robin)
    expect(pool[0]!.scriptIndex).toBe(0);
    expect(pool[1]!.scriptIndex).toBe(1);
  });
});
