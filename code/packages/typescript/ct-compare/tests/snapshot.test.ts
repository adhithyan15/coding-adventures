/**
 * Regression tests for SEC finding S1 (LV-007b security review).
 *
 * `toBytes` used to wrap an `ArrayBuffer` in a view instead of copying it.
 * `ctEq(left, right)` converts `left` first and `right` second, and converting
 * a plain-array `right` runs caller code (a Proxy `length` trap or an element
 * getter). That code could detach, shrink or rewrite `left`'s buffer after
 * `left` was "converted", so two different inputs compared equal. Every
 * accepted input must now be a snapshot taken at conversion time.
 */
import { describe, expect, it } from "vitest";
import { ctEq, ctSelectBytes } from "../src/index";

/** A plain array whose element getter runs `sideEffect` once, then yields `values`. */
function trapArray(values: number[], sideEffect: () => void): number[] {
  const array = [...values];
  let fired = false;
  Object.defineProperty(array, 0, {
    get() {
      if (!fired) {
        fired = true;
        sideEffect();
      }
      return values[0];
    },
  });
  return array;
}

describe("ArrayBuffer inputs are snapshots (S1)", () => {
  it("detaching the left buffer while the right is read does not make them equal", () => {
    // An empty array whose `length` trap detaches the left buffer: a wrapped
    // (not copied) left would shrink to 0 bytes and "equal" the empty right.
    const buffer = new Uint8Array([1, 2, 3, 4]).buffer;
    const right = new Proxy([] as number[], {
      get(target, key, receiver) {
        if (key === "length") structuredClone(buffer, { transfer: [buffer] });
        return Reflect.get(target, key, receiver);
      },
    });
    expect(ctEq(buffer, right)).toBe(false);
  });

  it("shrinking a resizable left buffer while the right is read does not make them equal", () => {
    const buffer = new ArrayBuffer(4, { maxByteLength: 8 });
    new Uint8Array(buffer).set([9, 9, 9, 9]);
    const right = trapArray([9], () => buffer.resize(1));
    expect(ctEq(buffer, right)).toBe(false);
  });

  it("rewriting the left buffer while the right is read does not change the result", () => {
    const buffer = new Uint8Array([1, 2, 3, 4]).buffer;
    const right = trapArray([5, 6, 7, 8], () => new Uint8Array(buffer).set([5, 6, 7, 8]));
    expect(ctEq(buffer, right)).toBe(false);
  });

  it("ctSelectBytes selects the bytes the buffer held when it was converted", () => {
    const buffer = new Uint8Array([1, 2, 3]).buffer;
    const right = trapArray([7, 7, 7], () => new Uint8Array(buffer).fill(0));
    expect([...ctSelectBytes(buffer, right, true)]).toEqual([1, 2, 3]);
  });

  it("a buffer detached before the call is refused, not read as empty", () => {
    const buffer = new Uint8Array([1]).buffer;
    structuredClone(buffer, { transfer: [buffer] });
    expect(() => ctEq(buffer, [])).toThrow(TypeError);
  });
});
