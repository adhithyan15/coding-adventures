/**
 * contexts.test.ts — context constants and recognition helpers.
 */

import { describe, it, expect } from "vitest";
import {
  CONTEXT_PRINT, CONTEXT_SCREEN, CONTEXT_DARK,
  CONTEXT_NARROW, CONTEXT_WIDE,
  CONTEXT_REDUCED_MOTION, CONTEXT_HIGH_CONTRAST,
  STANDARD_CONTEXTS,
  isExtensionContext, isRecognisedContext,
} from "../src/index.js";

describe("standard context constants", () => {
  it("each is its expected literal string", () => {
    expect(CONTEXT_PRINT).toBe("print");
    expect(CONTEXT_SCREEN).toBe("screen");
    expect(CONTEXT_DARK).toBe("dark");
    expect(CONTEXT_NARROW).toBe("narrow");
    expect(CONTEXT_WIDE).toBe("wide");
    expect(CONTEXT_REDUCED_MOTION).toBe("reduced-motion");
    expect(CONTEXT_HIGH_CONTRAST).toBe("high-contrast");
  });
});

describe("STANDARD_CONTEXTS", () => {
  it("lists all seven kernel-blessed contexts", () => {
    expect(STANDARD_CONTEXTS).toEqual([
      "print", "screen", "dark",
      "narrow", "wide",
      "reduced-motion", "high-contrast",
    ]);
  });

  it("is frozen", () => {
    expect(() => (STANDARD_CONTEXTS as unknown as string[]).push("zz")).toThrow();
  });
});

describe("isExtensionContext", () => {
  it("accepts well-formed ext: contexts", () => {
    expect(isExtensionContext("ext:my-plugin:print-spread")).toBe(true);
  });

  it("rejects bare ext: prefix", () => {
    expect(isExtensionContext("ext:")).toBe(false);
  });

  it("rejects standard context names", () => {
    expect(isExtensionContext("print")).toBe(false);
  });
});

describe("isRecognisedContext", () => {
  it("accepts every standard context", () => {
    for (const name of STANDARD_CONTEXTS) {
      expect(isRecognisedContext(name)).toBe(true);
    }
  });

  it("accepts any ext:* context", () => {
    expect(isRecognisedContext("ext:plugin:context")).toBe(true);
  });

  it("rejects bare unknown contexts (typo guard)", () => {
    expect(isRecognisedContext("priint")).toBe(false);
    expect(isRecognisedContext("light")).toBe(false);
    expect(isRecognisedContext("")).toBe(false);
  });
});
