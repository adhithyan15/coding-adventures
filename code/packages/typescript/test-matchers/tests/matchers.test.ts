import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  expect,
  AssertionError,
  deepEqual,
  valuesEqual,
  toValue,
  canonicalF64,
  numberToF64Bits,
  f64BitsToNumber,
  encodeValue,
} from "../dist/index.js";

describe("@coding-adventures/test-matchers", () => {
  describe("toBe (identity)", () => {
    it("compares primitive identity", () => {
      expect(42).toBe(42);
      expect("hello").toBe("hello");
      expect(true).toBe(true);
      expect(null).toBe(null);
      expect(undefined).toBe(undefined);
      expect(NaN).toBe(NaN); // Object.is(NaN, NaN) is true

      expect(42).not.toBe(43);
      expect("hello").not.toBe("world");
      expect(true).not.toBe(false);
      expect(null).not.toBe(undefined);
    });

    it("distinguishes +0 and -0 per Object.is", () => {
      expect(+0).toBe(+0);
      expect(-0).toBe(-0);
      expect(+0).not.toBe(-0);
      expect(-0).not.toBe(+0);
    });

    it("compares reference identity for objects", () => {
      const obj = { a: 1 };
      expect(obj).toBe(obj);
      expect({ a: 1 }).not.toBe({ a: 1 });
    });

    it("throws AssertionError on failure", () => {
      assert.throws(
        () => expect(1).toBe(2),
        (err: unknown) => {
          assert(err instanceof AssertionError);
          assert.equal(err.name, "AssertionError");
          assert.equal(err.actual, 1);
          assert.equal(err.expected, 2);
          assert.equal(err.operator, "toBe");
          return true;
        }
      );

      assert.throws(
        () => expect(1).not.toBe(1),
        (err: unknown) => {
          assert(err instanceof AssertionError);
          assert.equal(err.message, "not expected 1 to be 1");
          return true;
        }
      );
    });
  });

  describe("toEqual & deepEqual", () => {
    it("compares deeply equal primitives and objects", () => {
      expect({ a: 1, b: [2, 3] }).toEqual({ a: 1, b: [2, 3] });
      expect([1, 2, 3]).toEqual([1, 2, 3]);
      expect(new Uint8Array([1, 2, 3])).toEqual(new Uint8Array([1, 2, 3]));
      expect(12345678901234567890n).toEqual(12345678901234567890n);

      expect({ a: 1 }).not.toEqual({ a: 2 });
      expect([1, 2]).not.toEqual([1, 2, 3]);
      expect([1, 2]).not.toEqual([1, 4]);
      expect(new Uint8Array([1, 2])).not.toEqual(new Uint8Array([1, 3]));
      expect(new Uint8Array([1, 2])).not.toEqual(new Uint8Array([1, 2, 3]));
      expect(1).not.toEqual("1");
      expect(null).not.toEqual(false);
      expect(null).not.toEqual(undefined);
      expect(undefined).not.toEqual(null);
      expect("text").not.toEqual(new Uint8Array([116]));
    });

    it("compares Date, RegExp, and Error objects correctly", () => {
      expect(new Date("2026-01-01T00:00:00Z")).toEqual(new Date("2026-01-01T00:00:00Z"));
      expect(new Date("2026-01-01T00:00:00Z")).not.toEqual(new Date("2020-01-01T00:00:00Z"));

      expect(/abc/g).toEqual(/abc/g);
      expect(/abc/g).not.toEqual(/abc/i);
      expect(/abc/g).not.toEqual(/xyz/g);

      expect(new Error("test error")).toEqual(new Error("test error"));
      expect(new Error("test error")).not.toEqual(new Error("other error"));
      expect(new TypeError("test error")).not.toEqual(new RangeError("test error"));
    });

    it("handles DAGs with shared diamond references in O(N) time", () => {
      let node: Record<string, unknown> = { val: 0 };
      for (let i = 0; i < 20; i++) {
        node = { left: node, right: node, val: i + 1 };
      }
      expect(node).toEqual(node);
    });

    it("enforces maximum nesting depth ceiling", () => {
      let deep: Record<string, unknown> = { val: 0 };
      for (let i = 0; i < 70; i++) {
        deep = { next: deep };
      }
      assert.throws(() => toValue(deep), /Maximum nesting depth 64 exceeded/);
    });

    it("handles Sets and Maps", () => {
      const setA = new Set([1, 2, 3]);
      const setB = new Set([1, 2, 3]);
      expect(setA).toEqual(setB);

      const mapA = new Map<unknown, unknown>([["k1", "v1"], ["k2", 2]]);
      const mapB = new Map<unknown, unknown>([["k2", 2], ["k1", "v1"]]);
      expect(mapA).toEqual(mapB);

      const mapC = new Map<unknown, unknown>([["k1", "v1"], ["k2", 3]]);
      expect(mapA).not.toEqual(mapC);

      const mapD = new Map<unknown, unknown>([["k1", "v1"]]);
      expect(mapA).not.toEqual(mapD);

      const mapE = new Map<unknown, unknown>([["unmatched", "v1"], ["k2", 2]]);
      expect(mapA).not.toEqual(mapE);
    });

    it("handles deepEqual standalone helper", () => {
      assert.equal(deepEqual({ x: 1 }, { x: 1 }), true);
      assert.equal(deepEqual([1, 2], [1, 3]), false);
      assert.equal(deepEqual(1, "1"), false);
    });

    it("handles circular references gracefully", () => {
      const circA: Record<string, unknown> = { x: 1 };
      circA.self = circA;
      const circB: Record<string, unknown> = { x: 1 };
      circB.self = circB;

      expect(circA).toEqual(circB);
    });

    it("throws on toEqual mismatch", () => {
      assert.throws(
        () => expect({ x: 1 }).toEqual({ x: 2 }),
        (err: unknown) => {
          assert(err instanceof AssertionError);
          assert.equal(err.operator, "toEqual");
          return true;
        }
      );
    });
  });

  describe("toStrictEqual", () => {
    it("verifies structural and prototype equality including nested objects", () => {
      class Foo {
        val: number;
        constructor(val: number) {
          this.val = val;
        }
      }
      class Bar {
        val: number;
        constructor(val: number) {
          this.val = val;
        }
      }

      const foo1 = new Foo(10);
      const foo2 = new Foo(10);
      const bar = new Bar(10);

      expect(foo1).toStrictEqual(foo2);
      expect(foo1).not.toStrictEqual(bar);

      // Nested prototype mismatch must be caught
      expect({ child: foo1 }).not.toStrictEqual({ child: bar });
    });

    it("throws when prototypes differ", () => {
      class A {
        x = 1;
      }
      class B {
        x = 1;
      }
      assert.throws(
        () => expect(new A()).toStrictEqual(new B()),
        (err: unknown) => {
          assert(err instanceof AssertionError);
          assert.match(err.message, /matching prototypes/);
          return true;
        }
      );
    });
  });

  describe("truthiness & presence matchers", () => {
    it("toBeTruthy and toBeFalsy", () => {
      expect(true).toBeTruthy();
      expect(1).toBeTruthy();
      expect("str").toBeTruthy();
      expect({}).toBeTruthy();

      expect(false).toBeFalsy();
      expect(0).toBeFalsy();
      expect("").toBeFalsy();
      expect(null).toBeFalsy();
      expect(undefined).toBeFalsy();

      expect(false).not.toBeTruthy();
      expect(true).not.toBeFalsy();

      assert.throws(() => expect(false).toBeTruthy(), AssertionError);
      assert.throws(() => expect(true).toBeFalsy(), AssertionError);
    });

    it("toBeNull, toBeUndefined, toBeDefined", () => {
      expect(null).toBeNull();
      expect(42).not.toBeNull();

      expect(undefined).toBeUndefined();
      expect(null).not.toBeUndefined();
      expect(0).not.toBeUndefined();

      expect(0).toBeDefined();
      expect("").toBeDefined();
      expect(null).toBeDefined();
      expect(undefined).not.toBeDefined();

      assert.throws(() => expect(42).toBeNull(), AssertionError);
      assert.throws(() => expect(null).toBeUndefined(), AssertionError);
      assert.throws(() => expect(undefined).toBeDefined(), AssertionError);
    });

    it("toBeNaN", () => {
      expect(NaN).toBeNaN();
      expect(42).not.toBeNaN();
      expect("hello").not.toBeNaN();

      assert.throws(() => expect(42).toBeNaN(), AssertionError);
      assert.throws(() => expect(NaN).not.toBeNaN(), AssertionError);
    });

    it("toBeInstanceOf", () => {
      class Animal {}
      class Dog extends Animal {}
      const dog = new Dog();

      expect(dog).toBeInstanceOf(Dog);
      expect(dog).toBeInstanceOf(Animal);
      expect(dog).not.toBeInstanceOf(RegExp);

      assert.throws(() => expect(dog).toBeInstanceOf(RegExp), AssertionError);
    });
  });

  describe("numeric relations", () => {
    it("handles number comparisons", () => {
      expect(10).toBeGreaterThan(5);
      expect(10).toBeGreaterThanOrEqual(10);
      expect(10).toBeGreaterThanOrEqual(9);
      expect(5).toBeLessThan(10);
      expect(5).toBeLessThanOrEqual(5);
      expect(4).toBeLessThanOrEqual(5);

      expect(5).not.toBeGreaterThan(10);
      expect(10).not.toBeLessThan(5);

      assert.throws(() => expect(5).toBeGreaterThan(10), AssertionError);
      assert.throws(() => expect(10).toBeLessThan(5), AssertionError);
    });

    it("handles BigInt comparisons", () => {
      expect(100n).toBeGreaterThan(50n);
      expect(100n).toBeGreaterThanOrEqual(100n);
      expect(50n).toBeLessThan(100n);
      expect(50n).toBeLessThanOrEqual(50n);
    });

    it("toBeCloseTo floating-point comparisons", () => {
      expect(0.1 + 0.2).toBeCloseTo(0.3, 2);
      expect(Math.PI).toBeCloseTo(3.14, 2);
      expect(Math.PI).toBeCloseTo(3.1416, 3);
      expect(10).not.toBeCloseTo(20, 1);

      assert.throws(() => expect(10).toBeCloseTo(20, 1), AssertionError);
    });
  });

  describe("collection & string matchers", () => {
    it("toContain for strings, arrays, sets, maps", () => {
      expect("hello world").toContain("world");
      expect("hello world").not.toContain("moon");

      expect([1, 2, 3]).toContain(2);
      expect([1, 2, 3]).not.toContain(4);

      const s = new Set(["apple", "banana"]);
      expect(s).toContain("apple");
      expect(s).not.toContain("orange");

      const m = new Map([["key1", 100]]);
      expect(m).toContain("key1");
      expect(m).not.toContain("key2");

      assert.throws(() => expect([1, 2]).toContain(3), AssertionError);
    });

    it("toHaveLength", () => {
      expect([1, 2, 3]).toHaveLength(3);
      expect("hello").toHaveLength(5);
      expect([]).toHaveLength(0);
      expect([1]).not.toHaveLength(2);

      assert.throws(() => expect([1]).toHaveLength(2), AssertionError);
    });

    it("toMatch strings and regex without stateful lastIndex pollution", () => {
      expect("hello world").toMatch("world");
      expect("hello world").toMatch(/^hello/);
      expect("hello world").not.toMatch(/planet/);

      const globalRegex = /hello/g;
      expect("hello world").toMatch(globalRegex);
      expect("hello world").toMatch(globalRegex); // second invocation must still match

      assert.throws(() => expect("hello").toMatch(/planet/), AssertionError);
      assert.throws(() => expect(123 as unknown as string).toMatch("123"), AssertionError);
    });

    it("toHaveProperty presence and value checks without prototype leakage", () => {
      const obj = { foo: "bar", count: 42, nested: { x: 1 } };
      expect(obj).toHaveProperty("foo");
      expect(obj).toHaveProperty("foo", "bar");
      expect(obj).toHaveProperty("count", 42);
      expect(obj).toHaveProperty("nested", { x: 1 });

      expect(obj).not.toHaveProperty("nonexistent");
      expect(obj).not.toHaveProperty("foo", "baz");

      // Inherited Object.prototype properties must NOT pass as own properties
      expect({}).not.toHaveProperty("toString");
      expect({}).not.toHaveProperty("constructor");
      expect({}).not.toHaveProperty("__proto__");

      assert.throws(() => expect(obj).toHaveProperty("missing"), AssertionError);
      assert.throws(() => expect(obj).toHaveProperty("foo", "wrong"), AssertionError);
      assert.throws(() => expect(null).toHaveProperty("foo"), AssertionError);
    });
  });

  describe("toThrow exceptions", () => {
    it("catches thrown errors and validates message/type", () => {
      const throwError = () => {
        throw new Error("unauthorized access to resource");
      };
      const throwTypeError = () => {
        throw new TypeError("invalid type specified");
      };
      const throwStr = () => {
        throw "custom string error";
      };
      const noThrow = () => 42;

      expect(throwError).toThrow();
      expect(throwError).toThrow("unauthorized");
      expect(throwError).toThrow(/access to resource/);
      expect(throwError).toThrow(Error);
      expect(throwError).toThrow(new Error("unauthorized access to resource"));
      assert.throws(
        () => expect(throwError).toThrow(new Error("wrong message")),
        AssertionError
      );
      assert.throws(
        () => expect(throwError).toThrow(new TypeError("unauthorized access to resource")),
        AssertionError
      );

      expect(throwTypeError).toThrow(TypeError);
      expect(throwStr).toThrow("custom string");

      expect(noThrow).not.toThrow();

      assert.throws(() => expect(noThrow).toThrow(), AssertionError);
      assert.throws(() => expect(throwError).not.toThrow(), AssertionError);
      assert.throws(() => expect(throwError).toThrow("missing text"), AssertionError);
      assert.throws(() => expect(throwError).toThrow(/missing regex/), AssertionError);
      assert.throws(() => expect(throwError).toThrow(TypeError), AssertionError);
      assert.throws(() => expect(123 as unknown as () => void).toThrow(), AssertionError);
    });
  });

  describe("async matchers (resolves & rejects)", () => {
    it("resolves handles async values across all matchers", async () => {
      await expect(Promise.resolve(42)).resolves.toBe(42);
      await expect(Promise.resolve({ a: 1 })).resolves.toEqual({ a: 1 });
      await expect(Promise.resolve({ a: 1 })).resolves.toStrictEqual({ a: 1 });
      await expect(Promise.resolve("hello")).resolves.toBeTruthy();
      await expect(Promise.resolve("")).resolves.toBeFalsy();
      await expect(Promise.resolve(null)).resolves.toBeNull();
      await expect(Promise.resolve(undefined)).resolves.toBeUndefined();
      await expect(Promise.resolve(10)).resolves.toBeDefined();
      await expect(Promise.resolve(new Error("msg"))).resolves.toBeInstanceOf(Error);
      await expect(Promise.resolve(NaN)).resolves.toBeNaN();
      await expect(Promise.resolve(10)).resolves.toBeGreaterThan(5);
      await expect(Promise.resolve(10)).resolves.toBeGreaterThanOrEqual(10);
      await expect(Promise.resolve(5)).resolves.toBeLessThan(10);
      await expect(Promise.resolve(5)).resolves.toBeLessThanOrEqual(5);
      await expect(Promise.resolve(3.14)).resolves.toBeCloseTo(3.14, 2);
      await expect(Promise.resolve([1, 2])).resolves.toHaveLength(2);
      await expect(Promise.resolve([1, 2])).resolves.toContain(2);
      await expect(Promise.resolve("foobar")).resolves.toMatch(/bar/);
      await expect(Promise.resolve({ key: 1 })).resolves.toHaveProperty("key");
      await expect(Promise.resolve({ key: 1 })).resolves.toHaveProperty("key", 1);

      await expect(Promise.resolve(42)).resolves.not.toBe(43);
    });

    it("rejects handles rejected promises", async () => {
      await expect(Promise.reject(new Error("failure"))).rejects.toBeInstanceOf(Error);
      await expect(Promise.reject(new Error("failure"))).rejects.toThrow("failure");
      await expect(Promise.reject(new Error("failure"))).rejects.toThrow(/fail/);
      await expect(Promise.reject(new Error("failure"))).rejects.toThrow(Error);
      await expect(Promise.reject(new Error("failure"))).rejects.toThrow(new Error("failure"));

      await assert.rejects(
        async () => await expect(Promise.resolve(42)).resolves.toThrow(),
        AssertionError
      );
    });
  });

  describe("deep equality utilities", () => {
    it("canonicalizes NaN patterns", () => {
      const qNaNBits = 0x7ff8_0000_0000_0001n;
      const canonicalBits = canonicalF64(qNaNBits);
      assert.equal(canonicalBits, 0x7ff8_0000_0000_0000n);

      const floatBits = numberToF64Bits(3.14);
      assert.equal(canonicalF64(floatBits), floatBits);
      assert.equal(f64BitsToNumber(floatBits), 3.14);
    });

    it("encodes values accurately", () => {
      assert.equal(encodeValue({ kind: "null" }), "null");
      assert.equal(encodeValue({ kind: "bool", value: true }), "true");
      assert.equal(encodeValue({ kind: "int", text: "42" }), "42n");
      assert.equal(encodeValue({ kind: "text", value: "hi" }), '"hi"');
      assert.equal(
        encodeValue({ kind: "bytes", value: new Uint8Array([0xde, 0xad]) }),
        "Uint8Array([dead])"
      );
      assert.equal(
        encodeValue({
          kind: "list",
          items: [{ kind: "int", text: "1" }],
        }),
        "[1n]"
      );
    });
  });
});
