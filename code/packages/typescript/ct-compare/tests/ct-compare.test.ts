import { describe, expect, it } from "vitest";
import { ctEq, ctEqFixed, ctEqU64, ctSelectBytes } from "../src/index";

describe("ctEq", () => {
  it("matches byte equality without length confusion", () => {
    expect(ctEq(new TextEncoder().encode("abcdef"), new TextEncoder().encode("abcdef"))).toBe(true);
    expect(ctEq([], [])).toBe(true);
    expect(ctEq([1, 2, 3], [1, 2, 4])).toBe(false);
    expect(ctEq([9, 2, 3], [1, 2, 3])).toBe(false);
    expect(ctEq([1, 2, 3], [1, 2, 3, 4])).toBe(false);
  });

  it("detects every single bit position", () => {
    const base = new Uint8Array(32).fill(0x42);
    for (let index = 0; index < 32; index += 1) {
      for (let bit = 0; bit < 8; bit += 1) {
        const flipped = new Uint8Array(base);
        flipped[index] ^= 1 << bit;
        expect(ctEq(base, flipped)).toBe(false);
      }
    }
  });
});

describe("ctEqFixed", () => {
  it("acts as the dynamic fixed-size companion", () => {
    expect(ctEqFixed(new Array(16).fill(0x11), new Array(16).fill(0x11))).toBe(true);
    expect(ctEqFixed(new Array(16).fill(0x11), [...new Array(15).fill(0x11), 0x10])).toBe(false);
  });
});

describe("ctSelectBytes", () => {
  it("selects either input while preserving byte values", () => {
    const left = Uint8Array.from(Array.from({ length: 256 }, (_, index) => index));
    const right = Uint8Array.from(Array.from({ length: 256 }, (_, index) => 255 - index));

    expect([...ctSelectBytes(left, right, true)]).toEqual([...left]);
    expect([...ctSelectBytes(left, right, false)]).toEqual([...right]);
    expect([...ctSelectBytes([], [], true)]).toEqual([]);
    expect(() => ctSelectBytes([1], [1, 2], true)).toThrow(/equal-length/);
  });
});

describe("input validation (regression: coercion made unequal inputs equal)", () => {
  // Each pair below used to compare equal, because Uint8Array.from rewrote
  // the bad elements to the same bytes. They must now be rejected outright.
  const call = (left: unknown, right: unknown) => ctEq(left as never, right as never);

  it("rejects array elements outside 0..255", () => {
    expect(() => call([256], [0])).toThrow(RangeError);
    expect(() => call([-1], [255])).toThrow(RangeError);
  });

  it("rejects non-integer and non-number elements", () => {
    expect(() => call([1.5], [1])).toThrow(RangeError);
    expect(() => call([Number.NaN], [0])).toThrow(RangeError);
    expect(() => call(["a"], [0])).toThrow(RangeError);
    // A sparse array has holes that read as undefined.
    expect(() => call(new Array(2), [0, 0])).toThrow(RangeError);
  });

  it("rejects strings and other non-byte containers", () => {
    expect(() => call("abc", "xyz")).toThrow(TypeError);
    expect(() => call(new Uint16Array([1]), new Uint16Array([1]))).toThrow(TypeError);
    expect(() => call(new Set([1]), new Set([1]))).toThrow(TypeError);
    expect(() => call(null, [])).toThrow(TypeError);
  });

  it("validates both inputs of ctSelectBytes and requires a boolean choice", () => {
    expect(() => ctSelectBytes([256] as never, [0], true)).toThrow(RangeError);
    expect(() => ctSelectBytes([1], [2], "0" as never)).toThrow(TypeError);
    expect(() => ctSelectBytes([1], [2], 1 as never)).toThrow(TypeError);
  });

  it("still accepts every legitimate byte shape", () => {
    const bytes = Uint8Array.from([0, 127, 255]);
    expect(ctEq(bytes, bytes.buffer)).toBe(true);
    expect(ctEq(bytes, [0, 127, 255])).toBe(true);
  });
});

describe("ctSelectBytes output ownership", () => {
  it("returns a fresh copy that later input mutation does not change", () => {
    const left = Uint8Array.from([1, 2, 3]);
    const right = Uint8Array.from([4, 5, 6]);
    const picked = ctSelectBytes(left, right, true);
    expect(picked).not.toBe(left);
    left[0] = 99;
    expect([...picked]).toEqual([1, 2, 3]);
  });
});

describe("ctEqU64", () => {
  it("handles equality, bit flips, and range validation", () => {
    expect(ctEqU64(0, 0)).toBe(true);
    expect(ctEqU64((1n << 64n) - 1n, (1n << 64n) - 1n)).toBe(true);
    expect(ctEqU64(0n, 1n << 63n)).toBe(false);

    const base = 0x1234_5678_9abc_def0n;
    for (let bit = 0n; bit < 64n; bit += 1n) {
      expect(ctEqU64(base, base ^ (1n << bit))).toBe(false);
    }

    expect(() => ctEqU64(-1, 0)).toThrow(/unsigned/);
    expect(() => ctEqU64(0n, 1n << 64n)).toThrow(/unsigned/);
    expect(() => ctEqU64(Number.MAX_SAFE_INTEGER + 1, 0)).toThrow(/safe integer/);
  });
});
