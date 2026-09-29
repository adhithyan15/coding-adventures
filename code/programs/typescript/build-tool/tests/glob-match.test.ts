/**
 * Tests for glob-match.ts -- Pure String-Based Glob Matching
 *
 * These tests cover the three wildcard types:
 *   - `*`  (zero or more characters within a segment)
 *   - `?`  (exactly one character)
 *   - double-star (zero or more path segments)
 *
 * Each test group focuses on a specific aspect of the matching algorithm
 * and includes both positive (should match) and negative (should not match)
 * cases to ensure correctness.
 */

import { describe, it, expect } from "vitest";
import {
  compilePatterns,
  GlobPatternError,
  matchCompiledPath,
  matchPath,
  matchPathWithStateCount,
  matchSegment,
  matchSegmentWithStateCount,
  parseSegmentWithStateCount,
} from "../src/glob-match.js";

// ---------------------------------------------------------------------------
// matchSegment -- single segment matching
// ---------------------------------------------------------------------------

describe("matchSegment", () => {
  it("matches exact strings", () => {
    expect(matchSegment("foo", "foo")).toBe(true);
    expect(matchSegment("foo", "bar")).toBe(false);
  });

  it("empty pattern matches empty text", () => {
    expect(matchSegment("", "")).toBe(true);
  });

  it("empty pattern does not match non-empty text", () => {
    expect(matchSegment("", "a")).toBe(false);
  });

  it("non-empty pattern does not match empty text (unless all wildcards)", () => {
    expect(matchSegment("a", "")).toBe(false);
    expect(matchSegment("*", "")).toBe(true);
  });

  it("? matches exactly one character", () => {
    expect(matchSegment("f?o", "foo")).toBe(true);
    expect(matchSegment("f?o", "fao")).toBe(true);
    expect(matchSegment("f?o", "fo")).toBe(false);
    expect(matchSegment("f?o", "fooo")).toBe(false);
  });

  it("* matches zero or more characters", () => {
    expect(matchSegment("*", "anything")).toBe(true);
    expect(matchSegment("*.py", "foo.py")).toBe(true);
    expect(matchSegment("*.py", ".py")).toBe(true);
    expect(matchSegment("*.py", "foo.rb")).toBe(false);
    expect(matchSegment("f*", "foo")).toBe(true);
    expect(matchSegment("f*", "f")).toBe(true);
    expect(matchSegment("f*o", "fo")).toBe(true);
    expect(matchSegment("f*o", "foooo")).toBe(true);
    expect(matchSegment("f*o", "foop")).toBe(false);
  });

  it("multiple wildcards in one segment", () => {
    expect(matchSegment("*.*", "foo.py")).toBe(true);
    expect(matchSegment("*.*", "foo")).toBe(false);
    expect(matchSegment("f?o*", "foo")).toBe(true);
    expect(matchSegment("f?o*", "foobar")).toBe(true);
    expect(matchSegment("f?o*", "fo")).toBe(false);
  });

  it("consecutive stars act like one", () => {
    expect(matchSegment("f**o", "foo")).toBe(true);
    expect(matchSegment("f**o", "fxxxo")).toBe(true);
  });

  it("matches portable character classes with Python fnmatchcase semantics", () => {
    const cases = [
      ["[^a].txt", "^.txt", "b.txt"],
      ["[]a].txt", "].txt", "b.txt"],
      ["[-a].txt", "-.txt", "b.txt"],
      ["[a-].txt", "-.txt", "b.txt"],
      ["[a-c].txt", "b.txt", "z.txt"],
      ["[!a-c].txt", "z.txt", "b.txt"],
    ] as const;

    for (const [pattern, matching, nonmatching] of cases) {
      expect(matchSegment(pattern, matching), pattern).toBe(true);
      expect(matchSegment(pattern, nonmatching), pattern).toBe(false);
    }

    expect(matchSegment("[^a].txt", "a.txt")).toBe(true);
    expect(matchSegment("[!]].txt", "a.txt")).toBe(true);
    expect(matchSegment("[!]].txt", "].txt")).toBe(false);
  });

  it("treats unmatched opening brackets and braces as literals", () => {
    for (const [pattern, value] of [
      ["[", "["],
      ["prefix[", "prefix["],
      ["[]", "[]"],
      ["[!]", "[!]"],
      ["{a,b}", "{a,b}"],
    ] as const) {
      expect(matchSegment(pattern, value), pattern).toBe(true);
    }
    expect(matchSegment("prefix[", "prefixx")).toBe(false);
    expect(matchSegment("{a,b}", "a")).toBe(false);
  });

  it("rejects ambiguous and descending classes with one stable typed error", () => {
    for (const pattern of [
      "[z-a].txt",
      "[a--b].txt",
      "[a&&b].txt",
      "[a~~b].txt",
      "[a||b].txt",
    ]) {
      expect(() => matchSegment(pattern, "a.txt")).toThrow(GlobPatternError);
      expect(() => matchSegment(pattern, "a.txt")).toThrow(
        "ambiguous or descending character class in glob pattern",
      );
    }
  });

  it("counts Unicode scalars rather than UTF-16 code units", () => {
    expect(matchSegment("?", "🐍")).toBe(true);
    expect(matchSegment("[🐀-🙏]", "🐍")).toBe(true);
    expect(matchSegment("[🐀-🙏]", "a")).toBe(false);
    expect(matchSegment("?", "e\u0301")).toBe(false);
    expect(matchSegment("??", "e\u0301")).toBe(true);
  });

  it("bounds unmatched-bracket parsing and segment matching states", () => {
    const longUnmatchedClass = "[".repeat(16_384);
    const parsed = parseSegmentWithStateCount(longUnmatchedClass);
    expect(parsed.tokens).toHaveLength(longUnmatchedClass.length);
    expect(parsed.visitedStates).toBe(2 * longUnmatchedClass.length);

    const segment = "*a*a*a*a*a*b";
    const value = "aaaaaaaaaaaaaaaaac";
    const result = matchSegmentWithStateCount(segment, value);
    const tokenCount = parseSegmentWithStateCount(segment).tokens.length;
    expect(result.matched).toBe(false);
    expect(result.visitedStates).toBe(
      (tokenCount + 1) * (Array.from(value).length + 1),
    );
  });
});

// ---------------------------------------------------------------------------
// matchPath -- full path matching
// ---------------------------------------------------------------------------

describe("matchPath", () => {
  // --- Exact matches ---
  it("exact path matches", () => {
    expect(matchPath("src/foo.py", "src/foo.py")).toBe(true);
    expect(matchPath("src/foo.py", "src/bar.py")).toBe(false);
  });

  it("single file at root", () => {
    expect(matchPath("README.md", "README.md")).toBe(true);
    expect(matchPath("README.md", "src/README.md")).toBe(false);
  });

  // --- Star wildcard ---
  it("star matches within a single segment", () => {
    expect(matchPath("src/*.py", "src/foo.py")).toBe(true);
    expect(matchPath("src/*.py", "src/bar.py")).toBe(true);
    expect(matchPath("src/*.py", "src/.py")).toBe(true);
  });

  it("star does NOT cross directory boundaries", () => {
    expect(matchPath("src/*.py", "src/sub/foo.py")).toBe(false);
  });

  it("star in middle of filename", () => {
    expect(matchPath("src/test_*.py", "src/test_foo.py")).toBe(true);
    expect(matchPath("src/test_*.py", "src/foo.py")).toBe(false);
  });

  // --- Question mark ---
  it("question mark matches one character", () => {
    expect(matchPath("src/f?o.py", "src/foo.py")).toBe(true);
    expect(matchPath("src/f?o.py", "src/fao.py")).toBe(true);
    expect(matchPath("src/f?o.py", "src/fo.py")).toBe(false);
  });

  // --- Double-star ---
  it("double-star matches zero segments", () => {
    expect(matchPath("**/foo.py", "foo.py")).toBe(true);
  });

  it("double-star matches one segment", () => {
    expect(matchPath("**/foo.py", "src/foo.py")).toBe(true);
  });

  it("double-star matches multiple segments", () => {
    expect(matchPath("**/foo.py", "a/b/c/foo.py")).toBe(true);
  });

  it("double-star at end matches any depth", () => {
    expect(matchPath("src/**", "src/foo.py")).toBe(true);
    expect(matchPath("src/**", "src/a/b/c.py")).toBe(true);
  });

  it("double-star in the middle", () => {
    expect(matchPath("src/**/test.py", "src/test.py")).toBe(true);
    expect(matchPath("src/**/test.py", "src/a/test.py")).toBe(true);
    expect(matchPath("src/**/test.py", "src/a/b/c/test.py")).toBe(true);
    expect(matchPath("src/**/test.py", "lib/test.py")).toBe(false);
  });

  it("double-star with star in final segment", () => {
    expect(matchPath("src/**/*.py", "src/foo.py")).toBe(true);
    expect(matchPath("src/**/*.py", "src/a/b/foo.py")).toBe(true);
    expect(matchPath("src/**/*.py", "src/a/b/foo.rb")).toBe(false);
  });

  it("double-star does not match partial segment names", () => {
    // The double-star matches whole segments, not partial names.
    expect(matchPath("src/**/foo.py", "src/xfoo.py")).toBe(false);
  });

  // --- Edge cases ---
  it("empty pattern matches empty path", () => {
    expect(matchPath("", "")).toBe(true);
  });

  it("pattern with trailing slash", () => {
    expect(matchPath("src/", "src/")).toBe(true);
  });

  it("consecutive double-stars collapse", () => {
    // Multiple consecutive double-star segments should behave like one.
    expect(matchPath("**/**/foo.py", "foo.py")).toBe(true);
    expect(matchPath("**/**/foo.py", "a/b/foo.py")).toBe(true);
  });

  it("only double-star matches everything", () => {
    expect(matchPath("**", "a")).toBe(true);
    expect(matchPath("**", "a/b/c")).toBe(true);
  });

  it("realistic Starlark source patterns", () => {
    // Patterns like you'd see in a BUILD file's srcs list.
    expect(matchPath("src/foo.py", "src/foo.py")).toBe(true);
    expect(matchPath("tests/foo.py", "tests/foo.py")).toBe(true);
    expect(matchPath("*.toml", "pyproject.toml")).toBe(true);
    expect(matchPath("*.toml", "src/pyproject.toml")).toBe(false);
  });

  it("no false positives on similar prefixes", () => {
    expect(matchPath("src/foo", "src/foobar")).toBe(false);
    expect(matchPath("src/foo", "src/foo/bar")).toBe(false);
  });

  it("complex nested pattern", () => {
    expect(matchPath("a/**/b/*/c.txt", "a/b/x/c.txt")).toBe(true);
    expect(matchPath("a/**/b/*/c.txt", "a/x/y/b/z/c.txt")).toBe(true);
    expect(matchPath("a/**/b/*/c.txt", "a/b/c.txt")).toBe(false);
  });

  it("double-star at the very start with nested dirs", () => {
    expect(matchPath("**/tests/*.py", "tests/test_foo.py")).toBe(true);
    expect(matchPath("**/tests/*.py", "pkg/tests/test_bar.py")).toBe(true);
    expect(matchPath("**/tests/*.py", "a/b/tests/test_baz.py")).toBe(true);
    expect(matchPath("**/tests/*.py", "tests/sub/test_baz.py")).toBe(false);
  });

  it("multiple stars in different segments", () => {
    expect(matchPath("*/src/*.ts", "lib/src/index.ts")).toBe(true);
    expect(matchPath("*/src/*.ts", "lib/src/utils.ts")).toBe(true);
    expect(matchPath("*/src/*.ts", "src/index.ts")).toBe(false);
  });

  it("question mark does not match empty string or slash", () => {
    expect(matchPath("?.py", "a.py")).toBe(true);
    expect(matchPath("?.py", ".py")).toBe(false);
    expect(matchPath("?.py", "ab.py")).toBe(false);
  });

  it("visits each adversarial globstar path state exactly once", () => {
    const pattern = [...Array.from({ length: 12 }, () => ["**", "a"]).flat(), "z"].join("/");
    const candidate = [...Array.from({ length: 24 }, () => "a"), "y"].join("/");
    const result = matchPathWithStateCount(pattern, candidate);

    expect(result.matched).toBe(false);
    expect(result.visitedStates).toBe(
      (pattern.split("/").length + 1) * (candidate.split("/").length + 1),
    );
  });

  it("keeps embedded double-stars segment-local and compiled forms reusable", () => {
    expect(matchPath("foo**bar", "fooxbar")).toBe(true);
    expect(matchPath("foo**bar", "foo/x/bar")).toBe(false);

    const compiled = compilePatterns(["src/**/*.ts", "tests/[a-c]?.ts"]);
    expect(Object.isFrozen(compiled)).toBe(true);
    expect(compiled.every((pattern) => Object.isFrozen(pattern))).toBe(true);
    expect(matchCompiledPath(compiled[0], "src/deep/file.ts")).toBe(true);
    expect(matchCompiledPath(compiled[0], "src/deep/file.rb")).toBe(false);
    expect(matchCompiledPath(compiled[1], "tests/b1.ts")).toBe(true);
  });
});
