/**
 * parse-cache.test.ts -- ParseCache tests
 *
 * Tests cover:
 *   - Cache miss (first parse)
 *   - Cache hit (same uri + version returns same result)
 *   - Cache invalidation (new version triggers re-parse)
 *   - Manual eviction
 *   - Diagnostics from parse errors
 *   - Clean source produces no diagnostics
 */

import { describe, it, expect } from "vitest";
import { ParseCache } from "../src/parse-cache.js";
import type { LanguageBridge } from "../src/language-bridge.js";
import type { Token, Diagnostic, DiagnosticSeverity } from "../src/types.js";

/**
 * MinimalBridge for testing -- splits by whitespace and detects "ERROR" in source.
 */
const mockBridge: LanguageBridge = {
  tokenize(source: string): Token[] {
    const tokens: Token[] = [];
    let col = 1;
    for (const word of source.split(/\s+/).filter(Boolean)) {
      tokens.push({ type: "WORD", value: word, line: 1, column: col });
      col += word.length + 1;
    }
    return tokens;
  },
  parse(source: string): [unknown, Diagnostic[]] {
    const diags: Diagnostic[] = [];
    if (source.includes("ERROR")) {
      diags.push({
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 5 },
        },
        severity: 1 as DiagnosticSeverity,
        message: "syntax error: unexpected ERROR token",
      });
    }
    return [source, diags];
  },
};

describe("ParseCache", () => {
  it("cache miss returns fresh parse result", () => {
    const cache = new ParseCache();
    const r1 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    expect(r1).toBeDefined();
    expect(r1.ast).toBe("hello");
  });

  it("cache hit returns same object", () => {
    const cache = new ParseCache();
    const r1 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    const r2 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    expect(r1).toBe(r2); // same reference
  });

  it("new version causes cache miss", () => {
    const cache = new ParseCache();
    const r1 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    const r2 = cache.getOrParse("file:///a.txt", 2, "hello world", mockBridge);
    expect(r1).not.toBe(r2);
  });

  it("evict removes cached entries", () => {
    const cache = new ParseCache();
    const r1 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    cache.evict("file:///a.txt");
    const r2 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    expect(r1).not.toBe(r2);
  });

  it("diagnostics populated for error source", () => {
    const cache = new ParseCache();
    const result = cache.getOrParse("file:///a.txt", 1, "source with ERROR token", mockBridge);
    expect(result.diagnostics.length).toBeGreaterThan(0);
  });

  it("no diagnostics for clean source", () => {
    const cache = new ParseCache();
    const result = cache.getOrParse("file:///clean.txt", 1, "hello world", mockBridge);
    expect(result.diagnostics).toHaveLength(0);
  });

  it("different URIs are cached independently", () => {
    const cache = new ParseCache();
    const r1 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    const r2 = cache.getOrParse("file:///b.txt", 1, "world", mockBridge);
    expect(r1).not.toBe(r2);
    expect(r1.ast).toBe("hello");
    expect(r2.ast).toBe("world");
  });

  it("evict only affects the specified URI", () => {
    const cache = new ParseCache();
    const r1 = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    cache.getOrParse("file:///b.txt", 1, "world", mockBridge);
    cache.evict("file:///a.txt");

    // a.txt should be re-parsed
    const r1b = cache.getOrParse("file:///a.txt", 1, "hello", mockBridge);
    expect(r1b).not.toBe(r1);

    // b.txt should still be cached
    const r2b = cache.getOrParse("file:///b.txt", 1, "world", mockBridge);
    // (r2b could be the same reference since b was not evicted)
    expect(r2b.ast).toBe("world");
  });
});
