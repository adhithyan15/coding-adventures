/**
 * collisions.test.ts — deterministic resolution of repeated slugs.
 */

import { describe, it, expect } from "vitest";
import { resolveCollisions } from "../src/index.js";

describe("resolveCollisions — basic numbering", () => {
  it("no collisions → identity", () => {
    expect(resolveCollisions(["a", "b", "c"])).toEqual(["a", "b", "c"]);
  });

  it("two collisions → first unsuffixed, second -2", () => {
    expect(resolveCollisions(["setup", "setup"])).toEqual(["setup", "setup-2"]);
  });

  it("three collisions → -2, -3", () => {
    expect(resolveCollisions(["setup", "setup", "setup"])).toEqual(
      ["setup", "setup-2", "setup-3"],
    );
  });

  it("interleaved collisions counted independently", () => {
    expect(resolveCollisions(["setup", "intro", "setup", "intro", "setup"])).toEqual(
      ["setup", "intro", "setup-2", "intro-2", "setup-3"],
    );
  });

  it("preserves first occurrence as unsuffixed (GitHub idiom)", () => {
    const out = resolveCollisions(["x", "y", "x"]);
    expect(out[0]).toBe("x");
    expect(out[2]).toBe("x-2");
  });
});

describe("resolveCollisions — skip naturally-taken suffixes", () => {
  it("skips an already-taken numeric suffix", () => {
    // 1st heading naturally slugs to "setup-2" (e.g. "Setup 2").
    // Then "Setup" appears — needs a suffix.  Must NOT use -2.
    expect(resolveCollisions(["setup", "setup-2", "setup"])).toEqual(
      ["setup", "setup-2", "setup-3"],
    );
  });

  it("skips multiple taken suffixes in a row", () => {
    expect(resolveCollisions(["x", "x-2", "x-3", "x"])).toEqual(
      ["x", "x-2", "x-3", "x-4"],
    );
  });

  it("non-contiguous taken suffixes still pick the smallest gap", () => {
    // "x-3" exists; next "x" collision should be "x-2".
    expect(resolveCollisions(["x", "x-3", "x"])).toEqual(["x", "x-3", "x-2"]);
  });
});

describe("resolveCollisions — determinism", () => {
  it("same input → byte-identical output", () => {
    const input = ["a", "b", "a", "c", "b", "a"];
    expect(resolveCollisions(input)).toEqual(resolveCollisions(input));
  });

  it("does not mutate input array", () => {
    const input = ["a", "a", "a"];
    const before = [...input];
    resolveCollisions(input);
    expect(input).toEqual(before);
  });

  it("returns a fresh array each call", () => {
    const input = ["x"];
    expect(resolveCollisions(input)).not.toBe(input);
  });
});

describe("resolveCollisions — edge cases", () => {
  it("empty input → empty output", () => {
    expect(resolveCollisions([])).toEqual([]);
  });

  it("single-element input → identity", () => {
    expect(resolveCollisions(["only"])).toEqual(["only"]);
  });

  it("output length always equals input length", () => {
    const inputs: string[][] = [
      [],
      ["a"],
      ["a", "a"],
      ["a", "b", "a", "a", "b"],
    ];
    for (const input of inputs) {
      expect(resolveCollisions(input).length).toBe(input.length);
    }
  });

  it("preserves empty-string slugs (caller responsibility to feed 'section' fallback)", () => {
    expect(resolveCollisions(["", "", ""])).toEqual(["", "-2", "-3"]);
  });
});
