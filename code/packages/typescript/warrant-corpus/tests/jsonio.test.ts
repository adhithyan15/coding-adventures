import { mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { CorpusError } from "../src/errors.ts";
import {
  MAX_JSON_DEPTH,
  decodeUtf8,
  isObject,
  parseJson,
  readBytes,
  renderJson,
  sha256Hex,
} from "../src/jsonio.ts";
import { tempDir } from "./helpers.ts";

const encode = (text: string): Uint8Array => new TextEncoder().encode(text);
const parse = (text: string): unknown => parseJson(encode(text));
const refuses = (text: string): void => {
  expect(() => parse(text)).toThrow(CorpusError);
};

describe("parseJson", () => {
  it("parses every JSON kind", () => {
    expect(parse('{"a": [1, -2, true, false, null, "x\\n\\u00e9\\"\\\\\\/\\b\\f\\r\\t"], "b": {}}')).toEqual({
      a: [1, -2, true, false, null, 'x\né"\\/\b\f\r\t'],
      b: {},
    });
    expect(parse(" \t\r\n[ ] ")).toEqual([]);
    expect(parse("0")).toBe(0);
  });

  it("rejects a duplicate key, even with different values", () => {
    refuses('{"a": 1, "a": 2}');
    refuses('{"a": {"b": 1, "b": 1}}');
  });

  it("refuses numbers JSON.parse would silently round", () => {
    refuses("9007199254740993");
    refuses("18446744073709551615");
    refuses("1.5");
    refuses("1e3");
    refuses("-0");
    expect(parse("9007199254740991")).toBe(9007199254740991);
  });

  it("refuses malformed documents", () => {
    for (const bad of [
      "",
      "{",
      "[1,]",
      "[1 2]",
      '{"a" 1}',
      '{"a": 1,}',
      "{a: 1}",
      '"abc',
      '"\\x"',
      '"\\u12"',
      '"a\u0001b"',
      "tru",
      "nul",
      "NaN",
      "Infinity",
      "01",
      "1 2",
      "[] x",
      "{",
      '{"a":',
      "[",
    ]) {
      refuses(bad);
    }
  });

  it("treats __proto__ as plain data on a prototype-less object", () => {
    const value = parse('{"__proto__": {"polluted": true}}') as Record<string, unknown>;
    expect(Object.getPrototypeOf(value)).toBeNull();
    expect(Object.keys(value)).toEqual(["__proto__"]);
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });

  it("bounds nesting depth", () => {
    expect(() => parse("[".repeat(MAX_JSON_DEPTH + 2) + "]".repeat(MAX_JSON_DEPTH + 2))).toThrow(CorpusError);
    expect(parse("[".repeat(20) + "]".repeat(20))).toBeInstanceOf(Array);
  });

  it("refuses invalid UTF-8 and keeps a BOM as data (so it fails)", () => {
    expect(() => parseJson(Uint8Array.of(0x22, 0xff, 0x22))).toThrow(CorpusError);
    expect(() => parseJson(Uint8Array.of(0xef, 0xbb, 0xbf, 0x31))).toThrow(CorpusError);
    expect(() => decodeUtf8(Uint8Array.of(0xc0), "tfv")).toThrow(/not valid UTF-8/);
  });
});

describe("renderJson", () => {
  it("sorts keys, indents by two, ends with one LF and escapes non-ASCII", () => {
    const text = renderJson({ b: [1, { z: 1, a: "é\u{1f600}" }], a: null, c: [], d: {} });
    expect(text).toBe(
      '{\n  "a": null,\n  "b": [\n    1,\n    {\n      "a": "\\u00e9\\ud83d\\ude00",\n      "z": 1\n    }\n  ],\n  "c": [],\n  "d": {}\n}\n',
    );
    expect(text.includes("\r")).toBe(false);
  });
});

describe("helpers", () => {
  it("hashes and recognises objects", () => {
    expect(sha256Hex(new Uint8Array(0))).toBe("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    expect(isObject({})).toBe(true);
    expect(isObject([])).toBe(false);
    expect(isObject(null)).toBe(false);
  });
});

describe("readBytes", () => {
  it("reads a regular file within the limit", () => {
    const path = join(tempDir(), "a.json");
    writeFileSync(path, "abc");
    expect(new TextDecoder().decode(readBytes(path, 3))).toBe("abc");
    expect(readBytes(path, 100).length).toBe(3);
  });

  it("refuses an oversized file before reading it", () => {
    const path = join(tempDir(), "big.json");
    writeFileSync(path, "abcd");
    expect(() => readBytes(path, 3)).toThrow(/exceeds 3 bytes/);
  });

  it("refuses a missing file and a directory", () => {
    const dir = tempDir();
    expect(() => readBytes(join(dir, "missing"), 10)).toThrow(/cannot read/);
    mkdirSync(join(dir, "d"));
    expect(() => readBytes(join(dir, "d"), 10)).toThrow(/not a regular file/);
  });

  // A symlink needs a privilege Windows grants only to administrators or to
  // developer mode, so creating one there would fail for the wrong reason.
  it.skipIf(process.platform === "win32")("refuses a symlink to a regular file", () => {
    const dir = tempDir();
    writeFileSync(join(dir, "real"), "x");
    symlinkSync(join(dir, "real"), join(dir, "link"));
    expect(() => readBytes(join(dir, "link"), 10)).toThrow(/not a regular file/);
  });
});
