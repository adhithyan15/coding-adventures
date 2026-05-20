/**
 * toml.test.ts — tiny TOML subset parser.
 */

import { describe, it, expect } from "vitest";
import { parseToml } from "../src/index.js";

describe("parseToml — scalars", () => {
  it("basic string", () => {
    expect(parseToml(`title = "Hello"`)).toEqual({ title: "Hello" });
  });
  it("literal string", () => {
    expect(parseToml(`title = 'C:\\path'`)).toEqual({ title: "C:\\path" });
  });
  it("string with escape sequences", () => {
    expect(parseToml(`s = "a\\nb\\tc\\r"`)).toEqual({ s: "a\nb\tc\r" });
  });
  it("integer", () => {
    expect(parseToml("n = 42")).toEqual({ n: 42 });
  });
  it("positive-signed integer", () => {
    expect(parseToml("n = +42")).toEqual({ n: 42 });
  });
  it("negative integer", () => {
    expect(parseToml("n = -7")).toEqual({ n: -7 });
  });
  it("float", () => {
    expect(parseToml("f = 3.14")).toEqual({ f: 3.14 });
  });
  it("boolean true", () => {
    expect(parseToml("draft = true")).toEqual({ draft: true });
  });
  it("boolean false", () => {
    expect(parseToml("draft = false")).toEqual({ draft: false });
  });
  it("RFC 3339 date", () => {
    expect(parseToml("d = 2026-05-20")).toEqual({ d: "2026-05-20" });
  });
  it("RFC 3339 datetime with Z", () => {
    expect(parseToml("d = 2026-05-20T12:00:00Z")).toEqual({ d: "2026-05-20T12:00:00Z" });
  });
});

describe("parseToml — arrays", () => {
  it("empty", () => {
    expect(parseToml("tags = []")).toEqual({ tags: [] });
  });
  it("strings", () => {
    expect(parseToml(`tags = ["a", "b"]`)).toEqual({ tags: ["a", "b"] });
  });
  it("integers", () => {
    expect(parseToml("ns = [1, 2, 3]")).toEqual({ ns: [1, 2, 3] });
  });
});

describe("parseToml — multiple keys + comments", () => {
  it("multiple", () => {
    expect(parseToml(`title = "Hello"\ndate = 2026-05-20\ntags = ["a"]`))
      .toEqual({ title: "Hello", date: "2026-05-20", tags: ["a"] });
  });
  it("ignores blank lines", () => {
    expect(parseToml(`a = 1\n\n\nb = 2`)).toEqual({ a: 1, b: 2 });
  });
  it("ignores comment lines", () => {
    expect(parseToml(`# c\na = 1\n# c2`)).toEqual({ a: 1 });
  });
  it("strips inline comment", () => {
    expect(parseToml(`a = 1 # inline`)).toEqual({ a: 1 });
  });
  it("inline # inside string preserved", () => {
    expect(parseToml(`a = "hello # world"`)).toEqual({ a: "hello # world" });
  });
});

describe("parseToml — security defences", () => {
  it("__proto__ rejected", () => {
    expect(() => parseToml("__proto__ = 1")).toThrow(/prototype-pollution/);
  });
  it("constructor rejected", () => {
    expect(() => parseToml(`constructor = "bad"`)).toThrow(/prototype-pollution/);
  });
  it("prototype rejected", () => {
    expect(() => parseToml("prototype = 1")).toThrow(/prototype-pollution/);
  });
  it("null prototype output", () => {
    expect(Object.getPrototypeOf(parseToml("a = 1"))).toBeNull();
  });
  it("duplicate key rejected", () => {
    expect(() => parseToml("a = 1\na = 2")).toThrow(/duplicated/);
  });
  it("toString rejected (widened reserved list)", () => {
    expect(() => parseToml("toString = 1")).toThrow(/prototype-pollution/);
  });
  it("valueOf rejected", () => {
    expect(() => parseToml("valueOf = 1")).toThrow(/prototype-pollution/);
  });
  it("__lookupGetter__ rejected", () => {
    expect(() => parseToml("__lookupGetter__ = 1")).toThrow(/prototype-pollution/);
  });
  it("source > 1MB rejected", () => {
    const huge = "a = 1\n".repeat(300_000);
    expect(() => parseToml(huge)).toThrow(/1048576-byte cap/);
  });
  it("> 1000 keys rejected", () => {
    const many = Array.from({ length: 1001 }, (_, i) => `k${i} = ${i}`).join("\n");
    expect(() => parseToml(many)).toThrow(/1000-key cap/);
  });
  it("> 64 KB value rejected", () => {
    expect(() => parseToml(`x = "${"a".repeat(70_000)}"`)).toThrow(/65536-byte cap/);
  });
});

describe("parseToml — error matrix", () => {
  it("table syntax [section] rejected", () => {
    expect(() => parseToml("[section]\na = 1")).toThrow(/tables.*not supported/);
  });
  it("non key=value line", () => {
    expect(() => parseToml("just text")).toThrow(/not a "key = value" pair/);
  });
  it("unrecognised scalar", () => {
    expect(() => parseToml("a = ???")).toThrow(/not recognised/);
  });
  it("safe-integer overflow", () => {
    expect(() => parseToml("n = 99999999999999999999")).toThrow(/safe integer/);
  });
  it("unescaped \" in basic string", () => {
    expect(() => parseToml(`a = "x"y"`)).toThrow();
  });
  it("unescaped ' in literal string", () => {
    expect(() => parseToml(`a = 'x'y'`)).toThrow();
  });
  it("unsupported escape", () => {
    expect(() => parseToml(`a = "\\z"`)).toThrow(/unsupported escape/);
  });
  it("unterminated string", () => {
    expect(() => parseToml(`a = "unterminated`)).toThrow();
  });
  it("unterminated string in inline array", () => {
    expect(() => parseToml(`a = ["a, "b"]`)).toThrow();
  });
  it("empty value", () => {
    expect(() => parseToml("a =")).toThrow(/empty value/);
  });
  it("inline array with escaped chars", () => {
    expect(parseToml(`x = ["a\\nb", "c"]`)).toEqual({ x: ["a\nb", "c"] });
  });
  it("string ending in escaped char on inline-comment scan", () => {
    expect(parseToml(`a = "a\\\\b" # comment`)).toEqual({ a: "a\\b" });
  });
  it("unterminated string detected by inline-comment scan", () => {
    expect(() => parseToml(`a = "unterminated`)).toThrow(/unterminated quoted string/);
  });
});
