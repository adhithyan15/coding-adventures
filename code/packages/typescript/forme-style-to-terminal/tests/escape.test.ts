/**
 * escape.test.ts — ANSI-unsafe stripping + TS-string-literal escaping.
 */

import { describe, it, expect } from "vitest";
import {
  stripAnsiUnsafe, escapeTsString, sanitiseKey,
} from "../src/index.js";

describe("stripAnsiUnsafe", () => {
  it("strips ESC (0x1B)", () => {
    expect(stripAnsiUnsafe("a\x1bb")).toBe("ab");
  });

  it("strips C1 CSI (0x9B)", () => {
    expect(stripAnsiUnsafe("a\x9bb")).toBe("ab");
  });

  it("strips C1 OSC (0x9D)", () => {
    expect(stripAnsiUnsafe("a\x9db")).toBe("ab");
  });

  it("strips ASCII control characters 0x00-0x1F", () => {
    const s = Array.from({ length: 32 }, (_, i) => String.fromCharCode(i)).join("");
    expect(stripAnsiUnsafe("a" + s + "b")).toBe("ab");
  });

  it("strips DEL (0x7F)", () => {
    expect(stripAnsiUnsafe("a\x7fb")).toBe("ab");
  });

  it("strips C1 range 0x80-0x9F", () => {
    const s = Array.from({ length: 32 }, (_, i) => String.fromCharCode(0x80 + i)).join("");
    expect(stripAnsiUnsafe("a" + s + "b")).toBe("ab");
  });

  it("passes through printable ASCII unchanged", () => {
    expect(stripAnsiUnsafe("Hello World 123")).toBe("Hello World 123");
  });

  it("passes through Unicode > 0x9F", () => {
    expect(stripAnsiUnsafe("café — résumé")).toBe("café — résumé");
  });
});

describe("escapeTsString", () => {
  it("escapes backslash to double-backslash", () => {
    expect(escapeTsString("a\\b")).toBe("a\\\\b");
  });

  it("escapes double-quote to backslash-quote", () => {
    expect(escapeTsString(`a"b`)).toBe(`a\\"b`);
  });

  it("escapes both, backslash-first (no collision)", () => {
    expect(escapeTsString(`a\\"b`)).toBe(`a\\\\\\"b`);
  });

  it("strips ANSI-unsafe bytes before escaping", () => {
    expect(escapeTsString("a\x1bb")).toBe("ab");
  });

  it("passes through plain ASCII unchanged", () => {
    expect(escapeTsString("Hello World")).toBe("Hello World");
  });

  it("does not escape single quote (not a TS string-literal special in double-quoted form)", () => {
    expect(escapeTsString("a'b")).toBe("a'b");
  });
});

describe("sanitiseKey", () => {
  it("is functionally identical to escapeTsString (same defences)", () => {
    const samples = [`hello`, `a\\b`, `a"b`, `a\x1bb`, ``];
    for (const s of samples) {
      expect(sanitiseKey(s)).toBe(escapeTsString(s));
    }
  });
});
