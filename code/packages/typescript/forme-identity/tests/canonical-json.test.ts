/**
 * forme-identity — canonical JSON tests
 *
 * Verifies RFC 8785 conformance for every observable behaviour and
 * pins the divergences from naïve `JSON.stringify`.
 */

import { describe, it, expect } from "vitest";
import { canonicalJson } from "../src/index.js";

describe("primitives", () => {
  it("null", () => expect(canonicalJson(null)).toBe("null"));
  it("true", () => expect(canonicalJson(true)).toBe("true"));
  it("false", () => expect(canonicalJson(false)).toBe("false"));
  it("integer", () => expect(canonicalJson(42)).toBe("42"));
  it("negative integer", () => expect(canonicalJson(-42)).toBe("-42"));
  it("float", () => expect(canonicalJson(0.1)).toBe("0.1"));
  it("zero", () => expect(canonicalJson(0)).toBe("0"));
  it("negative zero serialises like positive zero", () => {
    expect(canonicalJson(-0)).toBe("0");
  });
  it("simple string", () => expect(canonicalJson("hello")).toBe('"hello"'));
  it("empty string", () => expect(canonicalJson("")).toBe('""'));
});

describe("number rejection", () => {
  it("throws RangeError on NaN", () => {
    expect(() => canonicalJson(NaN)).toThrow(RangeError);
  });
  it("throws RangeError on Infinity", () => {
    expect(() => canonicalJson(Infinity)).toThrow(RangeError);
  });
  it("throws RangeError on -Infinity", () => {
    expect(() => canonicalJson(-Infinity)).toThrow(RangeError);
  });
});

describe("string escaping", () => {
  it("escapes the standard short forms", () => {
    expect(canonicalJson("\b\t\n\f\r\"\\")).toBe('"\\b\\t\\n\\f\\r\\"\\\\"');
  });
  it("escapes other control characters as lower-case \\uXXXX", () => {
    expect(canonicalJson("\x00\x01\x1f")).toBe('"\\u0000\\u0001\\u001f"');
  });
  it("does not escape characters >= U+0020", () => {
    expect(canonicalJson(" !#$%&'()*+,-./0:;<=>?@A[]^_`a{|}~")).toBe(
      '" !#$%&\'()*+,-./0:;<=>?@A[]^_`a{|}~"',
    );
  });
  it("preserves non-ASCII characters as-is", () => {
    expect(canonicalJson("héllo — 漢字 — 🦀")).toBe('"héllo — 漢字 — 🦀"');
  });
});

describe("arrays", () => {
  it("empty", () => expect(canonicalJson([])).toBe("[]"));
  it("single element", () => expect(canonicalJson([1])).toBe("[1]"));
  it("comma-separated, no spaces", () => {
    expect(canonicalJson([1, 2, 3])).toBe("[1,2,3]");
  });
  it("mixed types", () => {
    expect(canonicalJson([null, true, "a", 1])).toBe('[null,true,"a",1]');
  });
  it("nested", () => {
    expect(canonicalJson([[1, 2], [3, [4]]])).toBe("[[1,2],[3,[4]]]");
  });
});

describe("objects", () => {
  it("empty", () => expect(canonicalJson({})).toBe("{}"));
  it("single key", () => expect(canonicalJson({ a: 1 })).toBe('{"a":1}'));
  it("sorts keys lexicographically", () => {
    expect(canonicalJson({ b: 2, a: 1, c: 3 })).toBe('{"a":1,"b":2,"c":3}');
  });
  it("UTF-16 code-unit ordering for non-ASCII keys", () => {
    // U+00DF "ß" (0xC3 0x9F in UTF-8) should sort AFTER plain "z"
    // in UTF-16 code-unit order.
    expect(canonicalJson({ "ß": 1, z: 2 })).toBe('{"z":2,"ß":1}');
  });
  it("nested objects", () => {
    expect(canonicalJson({ outer: { b: 2, a: 1 } })).toBe(
      '{"outer":{"a":1,"b":2}}',
    );
  });
  it("escapes keys", () => {
    expect(canonicalJson({ "with\"quote": 1 })).toBe('{"with\\"quote":1}');
  });
});

describe("determinism — equal inputs produce equal output", () => {
  it("object key order does not matter", () => {
    const a = { x: 1, y: 2, z: 3 };
    const b = { z: 3, y: 2, x: 1 };
    expect(canonicalJson(a)).toBe(canonicalJson(b));
  });
  it("logically equal nested structures match byte-for-byte", () => {
    const a = { posts: [{ title: "Hi", date: "2026-05-15" }] };
    const b = { posts: [{ date: "2026-05-15", title: "Hi" }] };
    expect(canonicalJson(a)).toBe(canonicalJson(b));
  });
});

describe("cycle detection", () => {
  it("throws TypeError on a self-referential array", () => {
    const a: unknown[] = [];
    a.push(a);
    expect(() => canonicalJson(a as never)).toThrow(TypeError);
  });
  it("throws TypeError on a self-referential object", () => {
    const o: Record<string, unknown> = {};
    o.self = o;
    expect(() => canonicalJson(o as never)).toThrow(TypeError);
  });
  it("non-cyclic shared subtrees are NOT misclassified as cycles", () => {
    const shared = { x: 1 };
    const v = { a: shared, b: shared };
    expect(canonicalJson(v as never)).toBe('{"a":{"x":1},"b":{"x":1}}');
  });
});
