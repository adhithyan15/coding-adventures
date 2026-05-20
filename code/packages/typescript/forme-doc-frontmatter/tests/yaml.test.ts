/**
 * yaml.test.ts — tiny YAML subset parser.
 */

import { describe, it, expect } from "vitest";
import { parseYaml } from "../src/index.js";

describe("parseYaml — scalars", () => {
  it("bare string", () => {
    expect(parseYaml("title: Hello")).toEqual({ title: "Hello" });
  });
  it("double-quoted string", () => {
    expect(parseYaml(`title: "Hello, world"`)).toEqual({ title: "Hello, world" });
  });
  it("single-quoted string", () => {
    expect(parseYaml(`title: 'Hello'`)).toEqual({ title: "Hello" });
  });
  it("integer", () => {
    expect(parseYaml("position: 3")).toEqual({ position: 3 });
  });
  it("negative integer", () => {
    expect(parseYaml("count: -5")).toEqual({ count: -5 });
  });
  it("float", () => {
    expect(parseYaml("weight: 1.5")).toEqual({ weight: 1.5 });
  });
  it("true", () => {
    expect(parseYaml("draft: true")).toEqual({ draft: true });
  });
  it("false", () => {
    expect(parseYaml("draft: false")).toEqual({ draft: false });
  });
  it("null", () => {
    expect(parseYaml("author: null")).toEqual({ author: null });
  });
  it("~ as null", () => {
    expect(parseYaml("author: ~")).toEqual({ author: null });
  });
  it("date as string (we don't parse Date)", () => {
    expect(parseYaml("date: 2026-05-20")).toEqual({ date: "2026-05-20" });
  });
});

describe("parseYaml — inline arrays", () => {
  it("empty array", () => {
    expect(parseYaml("tags: []")).toEqual({ tags: [] });
  });
  it("scalar array", () => {
    expect(parseYaml("tags: [a, b, c]")).toEqual({ tags: ["a", "b", "c"] });
  });
  it("quoted array", () => {
    expect(parseYaml(`tags: ["a, b", "c"]`)).toEqual({ tags: ["a, b", "c"] });
  });
  it("mixed types", () => {
    expect(parseYaml("vals: [1, true, null, x]")).toEqual({ vals: [1, true, null, "x"] });
  });
});

describe("parseYaml — multi-line arrays", () => {
  it("list of strings", () => {
    expect(parseYaml("tags:\n  - a\n  - b\n  - c")).toEqual({ tags: ["a", "b", "c"] });
  });
  it("list with quoted", () => {
    expect(parseYaml(`tags:\n  - "a, b"\n  - c`)).toEqual({ tags: ["a, b", "c"] });
  });
});

describe("parseYaml — multiple keys", () => {
  it("title + date + tags", () => {
    expect(parseYaml(`title: Hello\ndate: 2026-05-20\ntags: [a, b]`))
      .toEqual({ title: "Hello", date: "2026-05-20", tags: ["a", "b"] });
  });
  it("ignores blank lines", () => {
    expect(parseYaml(`title: A\n\n\ndate: 2026-05-20`))
      .toEqual({ title: "A", date: "2026-05-20" });
  });
  it("ignores comments", () => {
    expect(parseYaml(`# top comment\ntitle: A\n# inner\nposition: 1`))
      .toEqual({ title: "A", position: 1 });
  });
});

describe("parseYaml — security defences", () => {
  it("__proto__ key rejected", () => {
    expect(() => parseYaml("__proto__: bad")).toThrow(/prototype-pollution/);
  });
  it("constructor key rejected", () => {
    expect(() => parseYaml("constructor: bad")).toThrow(/prototype-pollution/);
  });
  it("prototype key rejected", () => {
    expect(() => parseYaml("prototype: bad")).toThrow(/prototype-pollution/);
  });
  it("output object has null prototype (no Object.prototype walk)", () => {
    const out = parseYaml("title: x");
    expect(Object.getPrototypeOf(out)).toBeNull();
  });
  it("duplicate key rejected", () => {
    expect(() => parseYaml("a: 1\na: 2")).toThrow(/duplicated/);
  });
  it("toString rejected (widened reserved list)", () => {
    expect(() => parseYaml("toString: bad")).toThrow(/prototype-pollution/);
  });
  it("valueOf rejected", () => {
    expect(() => parseYaml("valueOf: bad")).toThrow(/prototype-pollution/);
  });
  it("hasOwnProperty rejected", () => {
    expect(() => parseYaml("hasOwnProperty: bad")).toThrow(/prototype-pollution/);
  });
  it("__defineGetter__ rejected", () => {
    expect(() => parseYaml("__defineGetter__: bad")).toThrow(/prototype-pollution/);
  });
  it("source > 1MB rejected", () => {
    const huge = "a: 1\n".repeat(300_000);  // ~1.5 MB
    expect(() => parseYaml(huge)).toThrow(/1048576-byte cap/);
  });
  it("> 1000 keys rejected", () => {
    const many = Array.from({ length: 1001 }, (_, i) => `k${i}: ${i}`).join("\n");
    expect(() => parseYaml(many)).toThrow(/1000-key cap/);
  });
  it("> 64 KB value rejected", () => {
    expect(() => parseYaml(`x: ${"a".repeat(70_000)}`)).toThrow(/65536-byte cap/);
  });
  it("error message truncates long bare scalars", () => {
    try {
      parseYaml(`x: ${"a".repeat(500)}{bad}`);
      expect.fail("expected throw");
    } catch (e) {
      const msg = (e as Error).message;
      expect(msg).toContain("…");
      expect(msg.length).toBeLessThan(500);
    }
  });
});

describe("parseYaml — error matrix", () => {
  it("indented continuation line", () => {
    expect(() => parseYaml("a: 1\n  b: 2")).toThrow(/indented continuation/);
  });
  it("line that is not key:value", () => {
    expect(() => parseYaml("just text")).toThrow(/not a "key: value" pair/);
  });
  it("invalid key (starts with digit)", () => {
    expect(() => parseYaml("1key: x")).toThrow(/not a "key: value" pair/);
  });
  it("empty scalar (bare colon)", () => {
    // Empty rhs would trigger multi-line array detection; with no
    // following `- item`, this throws.
    expect(() => parseYaml("a:")).toThrow(/empty value/);
  });
  it("safe-integer overflow", () => {
    expect(() => parseYaml("n: 99999999999999999999")).toThrow(/safe integer/);
  });
  it("structural chars in bare scalar", () => {
    expect(() => parseYaml("x: {bad}")).toThrow(/structural characters/);
  });
  it("unescaped quote inside double-quoted string", () => {
    expect(() => parseYaml(`x: "a"b"`)).toThrow(/unescaped/);
  });
  it("unsupported escape", () => {
    expect(() => parseYaml(`x: "\\n"`)).toThrow(/unsupported escape/);
  });
  it("supported escape: \\\\ for backslash", () => {
    expect(parseYaml(`x: "a\\\\b"`)).toEqual({ x: "a\\b" });
  });
  it("supported escape: \\\" inside double", () => {
    expect(parseYaml(`x: "a\\"b"`)).toEqual({ x: 'a"b' });
  });
  it("unterminated inline list string", () => {
    // rhs `["a, b]` — starts with `[`, ends with `]`, but contains
    // an unterminated `"a`; splitOutsideQuotes detects it.
    expect(() => parseYaml(`x: ["a, b]`)).toThrow(/unterminated quoted string in inline list/);
  });
  it("inline list with escape inside quoted item", () => {
    expect(parseYaml(`x: ["a\\\\b", c]`)).toEqual({ x: ["a\\b", "c"] });
  });
});
