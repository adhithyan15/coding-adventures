/**
 * context-mapper.test.ts — context name → CSS @media body.
 */

import { describe, it, expect } from "vitest";
import { contextToMedia } from "../src/index.js";

describe("contextToMedia", () => {
  it("standard contexts each map per FM04 §9.2", () => {
    expect(contextToMedia("print")).toBe("print");
    expect(contextToMedia("screen")).toBe("screen");
    expect(contextToMedia("dark")).toBe("(prefers-color-scheme: dark)");
    expect(contextToMedia("narrow")).toBe("(max-width: 40rem)");
    expect(contextToMedia("wide")).toBe("(min-width: 80rem)");
    expect(contextToMedia("reduced-motion")).toBe("(prefers-reduced-motion: reduce)");
    expect(contextToMedia("high-contrast")).toBe("(prefers-contrast: more)");
  });

  it("ext: contexts return null (translator emits warning + skip)", () => {
    expect(contextToMedia("ext:my-plugin:landscape")).toBeNull();
  });

  it("unknown names return null", () => {
    expect(contextToMedia("foo")).toBeNull();
    expect(contextToMedia("")).toBeNull();
  });
});
