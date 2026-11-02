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

describe("input validation (regression: forged, subclassed and proxied inputs)", () => {
  const call = (left: unknown, right: unknown) => ctEq(left as never, right as never);

  it("rejects an object that only fakes the Uint8Array prototype", () => {
    const fake = Object.setPrototypeOf({ length: 1 }, Uint8Array.prototype);
    expect(() => call(fake, [0])).toThrow(TypeError);
  });

  it("ignores a subclass's overridden length", () => {
    class Shrunk extends Uint8Array {
      override get length(): number {
        return 0;
      }
    }
    // The real contents are [1, 2, 3], so they must not equal [].
    expect(call(new Shrunk([1, 2, 3]), [])).toBe(false);
    expect(call(new Shrunk([1, 2, 3]), [1, 2, 3])).toBe(true);
  });

  it("rejects a Proxy over a real Uint8Array", () => {
    const proxied = new Proxy(new Uint8Array([1, 2, 3]), {});
    expect(() => call(proxied, [1, 2, 3])).toThrow(TypeError);
  });

  it("reads an array's length once, even if a getter changes it", () => {
    const shrinking = [7, 0, 0];
    Object.defineProperty(shrinking, 0, {
      get() {
        shrinking.length = 1;
        return 7;
      },
    });
    // Copied as [7, undefined, undefined], which is rejected, never as [7, 0, 0].
    expect(() => call(shrinking, [7, 0, 0])).toThrow(RangeError);
  });

  it("rejects a Proxy array whose length lies", () => {
    const lying = new Proxy([1, 2, 3], {
      get(target, key, receiver) {
        return key === "length" ? 0 : Reflect.get(target, key, receiver);
      },
    });
    // The proxy claims to be empty; it is read as empty, consistently.
    expect(call(lying, [])).toBe(true);
    expect(call(lying, [1, 2, 3])).toBe(false);
  });

  it("accepts Node Buffers and respects their offset into the shared pool", () => {
    expect(ctEq(Buffer.from("a"), Buffer.from("a"))).toBe(true);
    expect(ctEq(Buffer.from("a"), Buffer.from("b"))).toBe(false);
  });

  it("rejects a view that fell out of bounds of a shrunken resizable buffer", () => {
    const resizable = new ArrayBuffer(4, { maxByteLength: 8 });
    const view = new Uint8Array(resizable, 0, 4);
    resizable.resize(2);
    expect(() => call(view, new Uint8Array())).toThrow(TypeError);
  });

  it("rejects a Proxy array whose length is not a non-negative integer", () => {
    for (const badLength of [1.5, "2", Number.NaN, -1, { valueOf: () => 1 }]) {
      const lying = new Proxy([1, 2], {
        get(target, key, receiver) {
          return key === "length" ? badLength : Reflect.get(target, key, receiver);
        },
      });
      expect(() => call(lying, [1])).toThrow(TypeError);
    }
  });

  it("rejects a SharedArrayBuffer passed directly", () => {
    expect(() => call(new SharedArrayBuffer(1), new SharedArrayBuffer(1))).toThrow(TypeError);
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
