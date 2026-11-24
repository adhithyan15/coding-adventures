/**
 * Resource bounds and state validation (LGE-DOS).
 *
 * JavaScript's `<<` uses only the low 5 bits of the shift count, so before
 * the cap, decoder of 32 or 64 bits silently returned ONE output. These
 * tests assert oversize requests THROW rather than return anything.
 */
import { describe, it, expect } from "vitest";
import {
  decoder,
  demuxN,
  register,
  shiftRegister,
  counter,
  dFlipFlop,
  MAX_SELECT_BITS,
  MAX_WIDTH,
  MAX_COUNTER_WIDTH,
} from "../src/index.js";

const zeros = (n: number) => Array.from({ length: n }, () => 0) as (0 | 1)[];
const ff = () => ({ masterQ: 0, masterQBar: 1, slaveQ: 0, slaveQBar: 1 });

describe("constants", () => {
  it("match the cross-language contract", () => {
    expect(MAX_SELECT_BITS).toBe(16);
    expect(MAX_WIDTH).toBe(65536);
    expect(MAX_COUNTER_WIDTH).toBe(64);
  });
});

describe("decoder bounds", () => {
  it("accepts 16 bits", () => {
    const out = decoder(zeros(16));
    expect(out.length).toBe(65536);
    expect(out[0]).toBe(1);
    expect(out.reduce((a, b) => a + b, 0)).toBe(1);
  });
  it.each([17, 24, 25, 30, 31, 32, 33, 64, 1000])("rejects %i bits", (n) => {
    expect(() => decoder(zeros(n))).toThrow(RangeError);
  });
});

describe("demuxN bounds", () => {
  it("accepts 2^16 outputs", () => {
    const out = demuxN(1, zeros(16), 65536);
    expect(out.length).toBe(65536);
    expect(out[0]).toBe(1);
  });
  it.each([2 ** 17, 2 ** 31, 2 ** 32, 2 ** 33, 2 ** 64, Infinity])("rejects nOutputs %s", (n) => {
    expect(() => demuxN(1, zeros(1), n)).toThrow();
  });
  it("rejects selects over 16 bits", () => {
    expect(() => demuxN(1, zeros(32), 2)).toThrow(RangeError);
  });
  it.each([NaN, 2.5, "4" as unknown as number, -0])("rejects bad nOutputs %s", (n) => {
    expect(() => demuxN(1, zeros(2), n)).toThrow();
  });
  it("rejects non-power-of-two and wrong sel length", () => {
    expect(() => demuxN(1, zeros(2), 3)).toThrow(RangeError);
    expect(() => demuxN(1, zeros(3), 4)).toThrow(RangeError);
  });
});

describe("width validation", () => {
  const bad: [string, number][] = [
    ["NaN", NaN],
    ["Infinity", Infinity],
    ["-Infinity", -Infinity],
    ["1.5", 1.5],
  ];
  for (const [label, w] of bad) {
    it(`rejects width ${label}`, () => {
      expect(() => shiftRegister(0, 0, undefined, w)).toThrow(TypeError);
      expect(() => counter(0, 0, undefined, w)).toThrow(TypeError);
    });
  }
  it("rejects -0, 0, negatives", () => {
    for (const w of [-0, 0, -1]) {
      expect(() => shiftRegister(0, 0, undefined, w)).toThrow(RangeError);
      expect(() => counter(0, 0, undefined, w)).toThrow(RangeError);
    }
  });
  it("rejects huge widths promptly", () => {
    const t = Date.now();
    expect(() => shiftRegister(0, 0, undefined, 1e9)).toThrow(RangeError);
    expect(() => shiftRegister(0, 0, undefined, MAX_WIDTH + 1)).toThrow(RangeError);
    expect(() => counter(0, 0, undefined, 1e9)).toThrow(RangeError);
    expect(() => counter(0, 0, undefined, 65)).toThrow(RangeError);
    expect(() => register(zeros(3), 0, undefined, 1e9)).toThrow(RangeError);
    expect(Date.now() - t).toBeLessThan(1000);
  });
  it("accepts boundaries", () => {
    expect(shiftRegister(0, 0, undefined, MAX_WIDTH)[0].length).toBe(MAX_WIDTH);
    expect(counter(0, 0, undefined, 64)[0].length).toBe(64);
  });
  it("rejects bad register width argument", () => {
    expect(() => register(zeros(2), 0, undefined, NaN)).toThrow(TypeError);
  });
  it("rejects oversize register data", () => {
    expect(() => register(zeros(MAX_WIDTH + 1), 0)).toThrow(RangeError);
  });
});

describe("state validation on every path", () => {
  it("counter: hold, reset, increment all validate", () => {
    for (const [clock, reset] of [[0, 0], [1, 1], [1, 0], [0, 1]] as const) {
      const good = counter(0, 0, undefined, 4)[1];
      expect(() => counter(clock, reset, { ...good, value: [0, 0, 0] as never }, 4)).toThrow(RangeError);
      expect(() => counter(clock, reset, { ...good, value: [0, 0, 2, 0] as never }, 4)).toThrow(RangeError);
      expect(() => counter(clock, reset, { ...good, value: [0, 0, 0, undefined] as never }, 4)).toThrow(TypeError);
      expect(() => counter(clock, reset, { ...good, value: "0000" as never }, 4)).toThrow(TypeError);
      expect(() => counter(clock, reset, { ...good, ffState: [] }, 4)).toThrow(RangeError);
      expect(() => counter(clock, reset, null as never, 4)).toThrow(TypeError);
      expect(() => counter(clock, reset, { value: zeros(4) } as never, 4)).toThrow(TypeError);
    }
  });
  it("counter error names the offending element", () => {
    const good = counter(0, 0, undefined, 4)[1];
    expect(() => counter(0, 0, { ...good, value: [0, 0, 0, 9] as never }, 4)).toThrow(/state\.value\[3\]/);
  });
  it("counter hold returns a fresh array", () => {
    const [, st] = counter(0, 0, undefined, 4);
    const [, st2] = counter(0, 0, st, 4);
    expect(st2.value).toEqual(st.value);
    expect(st2.value).not.toBe(st.value);
  });
  it("counter still counts", () => {
    let st = counter(0, 0, undefined, 4)[1];
    let out: number[] = [];
    [out, st] = counter(1, 0, st, 4);
    expect(out).toEqual([1, 0, 0, 0]);
  });
  it("shiftRegister rejects malformed state", () => {
    const good = shiftRegister(0, 0, undefined, 3)[2];
    expect(() => shiftRegister(0, 0, good.slice(1), 3)).toThrow(RangeError);
    expect(() => shiftRegister(0, 0, [ff(), ff(), {}] as never, 3)).toThrow(TypeError);
    expect(() => shiftRegister(0, 0, [ff(), ff(), { ...ff(), slaveQ: 7 }] as never, 3)).toThrow(/state\[2\]\.slaveQ/);
    expect(() => shiftRegister(0, 0, [ff(), null, ff()] as never, 3)).toThrow(TypeError);
    expect(() => shiftRegister(0, 0, "abc" as never, 3)).toThrow(TypeError);
    expect(() => shiftRegister(0, 0, null as never, 3)).toThrow(TypeError);
  });
  it("register rejects malformed state", () => {
    expect(() => register([0, 1], 0, [ff()])).toThrow(RangeError);
    expect(() => register([0, 1], 0, [ff(), { masterQ: 0 }] as never)).toThrow(TypeError);
    expect(() => register([0, 1], 0, [ff(), { ...ff(), masterQBar: -0 }] as never)).toThrow(RangeError);
    expect(() => register([0, 1], 0, {} as never)).toThrow(TypeError);
  });
  it("dFlipFlop rejects malformed state", () => {
    expect(() => dFlipFlop(0, 0, { masterQ: 0 } as never)).toThrow(TypeError);
    expect(() => dFlipFlop(0, 0, null as never)).toThrow(TypeError);
    expect(() => dFlipFlop(0, 0, [] as never)).toThrow(TypeError);
  });
});
