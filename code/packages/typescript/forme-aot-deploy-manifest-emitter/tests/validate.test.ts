/**
 * validate.test.ts — output-path + string validators.
 */

import { describe, it, expect } from "vitest";
import { validateOutputPath, validateString } from "../src/index.js";

describe("validateOutputPath — accept", () => {
  it("simple filename", () => { expect(validateOutputPath("favicon.ico", "f")).toBe("favicon.ico"); });
  it("nested", () => { expect(validateOutputPath("assets/img/logo.png", "f")).toBe("assets/img/logo.png"); });
  it(".well-known/", () => {
    expect(validateOutputPath(".well-known/security.txt", "f")).toBe(".well-known/security.txt");
  });
  it("hyphens/underscores/digits", () => {
    expect(validateOutputPath("a-b_c123.txt", "f")).toBe("a-b_c123.txt");
  });
  it("deep nesting", () => {
    expect(validateOutputPath("a/b/c/d/e/f/g.txt", "f")).toBe("a/b/c/d/e/f/g.txt");
  });
});

describe("validateOutputPath — reject", () => {
  it("non-string", () => { expect(() => validateOutputPath(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null", () => { expect(() => validateOutputPath(null, "f")).toThrow(/got null/); });
  it("empty", () => { expect(() => validateOutputPath("", "f")).toThrow(/non-empty/); });
  it("over 2048", () => {
    expect(() => validateOutputPath("a".repeat(2050), "f")).toThrow(/≤ 2048/);
  });
  it("leading /", () => { expect(() => validateOutputPath("/abs/path", "f")).toThrow(/must be relative/); });
  it("leading ~", () => { expect(() => validateOutputPath("~/secret", "f")).toThrow(/home-dir/); });
  it("contains \\", () => { expect(() => validateOutputPath("a\\b", "f")).toThrow(/must not contain/); });
  it(".. segment", () => { expect(() => validateOutputPath("a/../b", "f")).toThrow(/path traversal/); });
  it(".. only", () => { expect(() => validateOutputPath("..", "f")).toThrow(/path traversal/); });
  it(". segment", () => { expect(() => validateOutputPath("a/./b", "f")).toThrow(/"\." segment/); });
  it(". only", () => { expect(() => validateOutputPath(".", "f")).toThrow(/"\." segment/); });
  it("empty mid-segment a//b", () => { expect(() => validateOutputPath("a//b", "f")).toThrow(/empty segments/); });
  it("trailing /", () => { expect(() => validateOutputPath("a/", "f")).toThrow(/empty segments/); });
  it("Windows drive C:/...", () => {
    expect(() => validateOutputPath("C:/Windows", "f"))
      .toThrow(/disallowed characters/);
  });
  it("URL-scheme-like first segment https:/...", () => {
    expect(() => validateOutputPath("https:/evil", "f"))
      .toThrow(/disallowed characters/);
  });
  it("colon in any segment rejected (Windows / HFS+ reserved)", () => {
    expect(() => validateOutputPath("a/b:c", "f")).toThrow(/disallowed characters/);
  });
  it("whitespace", () => { expect(() => validateOutputPath("a b.txt", "f")).toThrow(/disallowed characters/); });
  it("?", () => { expect(() => validateOutputPath("a?.txt", "f")).toThrow(/disallowed characters/); });
  it("#", () => { expect(() => validateOutputPath("a#b.txt", "f")).toThrow(/disallowed characters/); });
  it("NUL", () => { expect(() => validateOutputPath("a\x00b", "f")).toThrow(/disallowed characters/); });
  it("percent encoded %2e%2e", () => {
    expect(() => validateOutputPath("%2e%2e/etc", "f")).toThrow(/disallowed characters/);
  });
  it("unicode", () => { expect(() => validateOutputPath("café.txt", "f")).toThrow(/disallowed characters/); });
  it("error contains field name", () => {
    expect(() => validateOutputPath("../etc", "extraFiles[2].outputPath"))
      .toThrow(/extraFiles\[2\]\.outputPath/);
  });
  it("per-segment 255-byte cap", () => {
    expect(() => validateOutputPath("a".repeat(256), "f"))
      .toThrow(/exceeds 255-byte filesystem limit/);
  });
  it("255-byte segment at limit OK", () => {
    expect(validateOutputPath("a".repeat(255), "f")).toBe("a".repeat(255));
  });
  it("Windows reserved 'CON'", () => {
    expect(() => validateOutputPath("CON", "f")).toThrow(/Windows reserved device name/);
  });
  it("Windows reserved 'con' (case-insensitive)", () => {
    expect(() => validateOutputPath("con", "f")).toThrow(/Windows reserved device name/);
  });
  it("Windows reserved 'CON.txt' (with extension)", () => {
    expect(() => validateOutputPath("CON.txt", "f")).toThrow(/Windows reserved device name/);
  });
  it("Windows reserved 'PRN'", () => {
    expect(() => validateOutputPath("PRN", "f")).toThrow(/Windows reserved/);
  });
  it("Windows reserved 'AUX'", () => {
    expect(() => validateOutputPath("AUX", "f")).toThrow(/Windows reserved/);
  });
  it("Windows reserved 'NUL'", () => {
    expect(() => validateOutputPath("NUL", "f")).toThrow(/Windows reserved/);
  });
  it("Windows reserved 'COM1'", () => {
    expect(() => validateOutputPath("COM1", "f")).toThrow(/Windows reserved/);
  });
  it("Windows reserved 'LPT9.log'", () => {
    expect(() => validateOutputPath("LPT9.log", "f")).toThrow(/Windows reserved/);
  });
  it("Windows reserved nested 'a/CON/b'", () => {
    expect(() => validateOutputPath("a/CON/b", "f")).toThrow(/Windows reserved/);
  });
  it("'CONSOLE' (not reserved) OK", () => {
    expect(validateOutputPath("CONSOLE", "f")).toBe("CONSOLE");
  });
  it("trailing dot 'foo.'", () => {
    expect(() => validateOutputPath("foo.", "f")).toThrow(/must not end in "\." or " "/);
  });
  it("trailing space 'foo '", () => {
    // space isn't in PATH_SEGMENT_RE so it triggers "disallowed" first.
    expect(() => validateOutputPath("foo ", "f")).toThrow(/disallowed characters|end in/);
  });
  it("trailing dot in nested segment", () => {
    expect(() => validateOutputPath("a/b./c", "f")).toThrow(/must not end in "\." or " "/);
  });
  it("__proto__ as segment rejected", () => {
    expect(() => validateOutputPath("__proto__", "f")).toThrow(/prototype-pollution/);
  });
  it("constructor as segment rejected", () => {
    expect(() => validateOutputPath("constructor", "f")).toThrow(/prototype-pollution/);
  });
  it("prototype as segment rejected", () => {
    expect(() => validateOutputPath("prototype", "f")).toThrow(/prototype-pollution/);
  });
  it("nested __proto__ rejected", () => {
    expect(() => validateOutputPath("a/__proto__/b", "f")).toThrow(/prototype-pollution/);
  });
});

describe("validateString", () => {
  it("string passes", () => { expect(validateString("ok", "f")).toBe("ok"); });
  it("empty allowed", () => { expect(validateString("", "f")).toBe(""); });
  it("non-string rejected", () => { expect(() => validateString(42 as unknown as string, "f")).toThrow(/must be a string/); });
  it("null rejected", () => { expect(() => validateString(null, "f")).toThrow(/got null/); });
});
