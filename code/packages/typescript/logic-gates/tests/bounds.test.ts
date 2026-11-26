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
  muxN,
  encoder,
  priorityEncoder,
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

// Returns a Proxy over `target` whose reads past the first pass over the
// indices return `evil` instead -- a classic check-then-use attack.
function flipping<T>(target: T[], evil: unknown): T[] {
  const seen = new Map<string, number>();
  return new Proxy(target, {
    get(t, k, r) {
      if (typeof k === "string" && /^\d+$/.test(k)) {
        const c = (seen.get(k) ?? 0) + 1;
        seen.set(k, c);
        if (c > 1) return evil;
      }
      return Reflect.get(t, k, r);
    },
  });
}

describe("snapshot-then-validate (check-then-use)", () => {
  it("counter: proxy value that changes on second read cannot leak", () => {
    const good = counter(0, 0, undefined, 4)[1];
    for (const [clock, reset] of [[0, 0], [0, 1], [1, 0], [1, 1]] as const) {
      const st = { ...good, value: flipping(zeros(4), 99) };
      const [out, ns] = counter(clock, reset, st as never, 4);
      expect(out.every((b) => b === 0 || b === 1)).toBe(true);
      expect(ns.value.every((b) => b === 0 || b === 1)).toBe(true);
    }
  });
  it("counter: proxy ffState that changes on second read cannot leak", () => {
    const good = counter(0, 0, undefined, 2)[1];
    const st = { value: zeros(2), ffState: flipping(good.ffState, { bad: 1 }) };
    const [, ns] = counter(1, 0, st as never, 2);
    expect(ns.ffState.every((f) => f.slaveQ === 0 || f.slaveQ === 1)).toBe(true);
  });
  it("counter: getter state fields are read once", () => {
    let n = 0;
    const st = {
      get value() {
        return n++ === 0 ? zeros(2) : [7, 7];
      },
      ffState: [ff(), ff()],
    };
    const [out] = counter(0, 0, st as never, 2);
    expect(out).toEqual([0, 0]);
  });
  it("shiftRegister ignores own map and Array-subclass species", () => {
    const st = [ff(), ff(), ff()] as unknown[];
    (st as { map: unknown }).map = () => [5, 5, 5];
    const [out, sout] = shiftRegister(1, 1, st as never, 3);
    expect(out).toEqual([0, 0, 0]);
    expect(sout).toBe(0);
    class Evil extends Array {
      static get [Symbol.species]() {
        throw new Error("species consulted");
      }
    }
    const e = Evil.from([ff(), ff()]);
    expect(() => shiftRegister(0, 0, e as never, 2)).not.toThrow();
  });
  it("own Symbol.iterator is never consulted", () => {
    const v = zeros(2) as unknown as Record<symbol, unknown>;
    v[Symbol.iterator] = () => {
      throw new Error("iterated");
    };
    const st = { value: v, ffState: [ff(), ff()] };
    expect(() => counter(0, 0, st as never, 2)).not.toThrow();
  });
  it("proxied flip-flop state returning a bad slaveQ later is harmless", () => {
    const st = [flipping([0], 1) && ff(), ff()];
    const [out, sout] = shiftRegister(0, 0, st as never, 2);
    expect([...out, sout].every((b) => b === 0 || b === 1)).toBe(true);
  });
});

describe("own data properties only", () => {
  it("inherited (polluted) fields are rejected", () => {
    const proto = Object.prototype as unknown as Record<string, unknown>;
    try {
      proto.masterQ = 0;
      proto.masterQBar = 1;
      proto.slaveQ = 0;
      proto.slaveQBar = 1;
      expect(() => dFlipFlop(0, 0, {} as never)).toThrow(TypeError);
      expect(() => register([0], 0, [{}] as never)).toThrow(TypeError);
    } finally {
      delete proto.masterQ;
      delete proto.masterQBar;
      delete proto.slaveQ;
      delete proto.slaveQBar;
    }
  });
  it("accessor properties are rejected", () => {
    const o = {
      get masterQ() {
        return 0;
      },
      masterQBar: 1,
      slaveQ: 0,
      slaveQBar: 1,
    };
    expect(() => dFlipFlop(0, 0, o as never)).toThrow(/masterQ/);
  });
  it("returned state is a fresh object, not the caller's", () => {
    const mine = ff();
    const [, , ns] = dFlipFlop(1, 0, mine);
    expect(ns).not.toBe(mine);
  });
});

describe("muxN / encoders: array and length checks", () => {
  const arrayLike = { length: 2 ** 32 } as never;
  it("reject non-arrays and array-likes", () => {
    expect(() => muxN(arrayLike, [0])).toThrow(TypeError);
    expect(() => encoder(arrayLike)).toThrow(TypeError);
    expect(() => priorityEncoder(arrayLike)).toThrow(TypeError);
    expect(() => muxN(new Uint8Array(4) as never, [0, 0])).toThrow(TypeError);
  });
  it("reject lengths over the cap before the power-of-two test", () => {
    const big = new Array(2 ** 17).fill(0);
    expect(() => muxN(big, zeros(17))).toThrow(RangeError);
    expect(() => encoder(big)).toThrow(RangeError);
    expect(() => priorityEncoder(big)).toThrow(RangeError);
    const huge = new Array(2 ** 32 - 1) as never;
    expect(() => encoder(huge)).toThrow(RangeError);
  });
  it("proxy inputs that change on re-read are snapshotted", () => {
    const [bits, valid] = priorityEncoder(flipping([0, 1, 0, 0] as never[], 9) as never);
    expect(bits).toEqual([1, 0]);
    expect(valid).toBe(1);
    expect(muxN(flipping([0, 1] as never[], 9) as never, [1])).toBe(1);
  });
  it("error messages do not call caller toString", () => {
    const evil = { toString() { throw new Error("called"); } };
    expect(() => shiftRegister(0, 0, undefined, evil as never)).toThrow(TypeError);
    expect(() => demuxN(1, [0], evil as never)).toThrow(TypeError);
  });
});
