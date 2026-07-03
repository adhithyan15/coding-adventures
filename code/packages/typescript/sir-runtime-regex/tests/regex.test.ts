import { describe, it, expect } from "vitest";
import { compile, isMatch, matchData, stripExtended } from "../src/index.js";

describe("compile — flag translation", () => {
  it("always includes the JS 'm' flag even with no flags", () => {
    // Ruby's ^/$ are always line anchors, so 'm' is unconditional.
    const r = compile("a");
    expect(r.flags).toContain("m");
    expect(r.flags).not.toContain("i");
    expect(r.flags).not.toContain("s");
  });

  it("maps 'i' to the JS 'i' flag", () => {
    const r = compile("a", "i");
    expect(r.flags).toContain("i");
    expect(r.flags).toContain("m"); // still always on
  });

  it("maps Ruby 'm' to the JS 's' (dotAll) flag, not JS 'm' semantics", () => {
    // Ruby /m means "dot matches newline" == JS dotAll ('s').
    const r = compile(".", "m");
    expect(r.flags).toContain("s");
    expect(r.test("\n")).toBe(true); // dotAll lets "." span a newline
    // Without Ruby /m, "." does not match a newline.
    expect(compile(".").test("\n")).toBe(false);
  });

  it("maps 'x' by stripping whitespace/comments (no JS flag added)", () => {
    const r = compile("a b", "x");
    // No 'x'-equivalent flag exists in JS, so flags stay just 'm'.
    expect(r.flags).not.toContain("x");
    // The space was stripped, so the pattern matches "ab".
    expect(r.test("ab")).toBe(true);
  });

  it("combines flags and de-duplicates", () => {
    const r = compile("a", "iimm");
    expect(r.flags).toContain("i");
    expect(r.flags).toContain("s");
    expect(r.flags).toContain("m");
    // No duplicate characters in the JS flag string.
    expect(new Set(r.flags).size).toBe(r.flags.length);
  });

  it("ignores unknown flag characters", () => {
    const r = compile("a", "zoiu");
    expect(r.flags).toContain("i");
    expect(r.flags).toContain("m");
  });
});

describe("stripExtended", () => {
  it("removes unescaped whitespace and # comments", () => {
    expect(stripExtended("a b\tc")).toBe("abc");
    expect(stripExtended("a # this is a comment\nb")).toBe("ab");
  });

  it("preserves escaped whitespace and escaped #", () => {
    expect(stripExtended("a\\ b")).toBe("a\\ b");
    expect(stripExtended("a\\#b")).toBe("a\\#b");
  });
});

describe("isMatch — unanchored search", () => {
  it("is true when the pattern matches anywhere", () => {
    // Ruby =~ is unanchored: a hit anywhere counts, not fullmatch.
    expect(isMatch("\\d+", "abc 42 xyz")).toBe(true);
  });

  it("is false when there is no match", () => {
    expect(isMatch("\\d+", "no digits here")).toBe(false);
  });

  it("accepts a precompiled RegExp without a lastIndex bug across calls", () => {
    // A global regex carries a mutable lastIndex; fresh() must clone away 'g'
    // so repeated calls are independent.
    const re = /\d+/g;
    expect(isMatch(re, "a1")).toBe(true);
    expect(isMatch(re, "a1")).toBe(true); // would flip to false if lastIndex leaked
    expect(isMatch(re, "none")).toBe(false);
  });
});

describe("matchData — group 0 or null", () => {
  it("returns the matched substring", () => {
    expect(matchData("\\d+", "abc 42 xyz")).toBe("42");
  });

  it("returns null on no match", () => {
    expect(matchData("\\d+", "no digits")).toBe(null);
  });

  it("accepts a precompiled RegExp", () => {
    expect(matchData(/[a-z]+/, "  HELLO world")).toBe("world");
  });
});
