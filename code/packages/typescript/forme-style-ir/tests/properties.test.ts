/**
 * properties.test.ts — `StyleProperty` union, frozen kind list,
 * `isExtensionKind` predicate.
 */

import { describe, it, expect } from "vitest";
import { PROPERTY_KINDS, isExtensionKind } from "../src/index.js";

describe("PROPERTY_KINDS", () => {
  it("covers every kernel-known property kind", () => {
    expect(PROPERTY_KINDS).toEqual([
      "color", "background", "border-color", "outline-color",
      "font-family", "font-size", "font-weight", "font-style",
      "text-transform", "leading", "tracking", "text-decoration",
      "space-before", "space-after", "indent", "padding",
      "max-width", "min-height", "align", "vertical-align",
      "border", "border-radius", "shadow", "opacity",
      "column-break", "page-break", "widow-orphan",
      "display", "visible",
    ]);
  });

  it("is frozen", () => {
    expect(() => (PROPERTY_KINDS as unknown as string[]).push("zz")).toThrow();
  });

  it("matches the documented count (29 closed-list kinds)", () => {
    expect(PROPERTY_KINDS.length).toBe(29);
  });
});

describe("isExtensionKind", () => {
  it("accepts well-formed ext: namespaced kinds", () => {
    expect(isExtensionKind("ext:mask:image")).toBe(true);
    expect(isExtensionKind("ext:syntax-highlight:keyword")).toBe(true);
  });

  it("rejects bare ext: prefix without a name", () => {
    expect(isExtensionKind("ext:")).toBe(false);
  });

  it("rejects non-extension kinds", () => {
    expect(isExtensionKind("color")).toBe(false);
    expect(isExtensionKind("font-size")).toBe(false);
  });

  it("rejects ext-prefixed strings without the colon", () => {
    expect(isExtensionKind("extras")).toBe(false);
  });
});
