/**
 * context-mapper.test.ts — terminal context-relevance verdict.
 */

import { describe, it, expect } from "vitest";
import { STANDARD_CONTEXTS } from "@coding-adventures/forme-style-ir";
import { contextRecognised } from "../src/index.js";

describe("contextRecognised", () => {
  it("returns true for every kernel-blessed context", () => {
    for (const c of STANDARD_CONTEXTS) {
      expect(contextRecognised(c)).toBe(true);
    }
  });

  it("returns false for ext: contexts", () => {
    expect(contextRecognised("ext:plugin:custom")).toBe(false);
  });

  it("returns false for unknown / typo contexts", () => {
    expect(contextRecognised("prnit")).toBe(false);
    expect(contextRecognised("")).toBe(false);
  });
});
