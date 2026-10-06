import { describe, expect, it } from "vitest";

import { planMacosSeaCopy } from "../scripts/build-worker-sea.mjs";

describe("macOS SEA executable preparation", () => {
  it("copies a matching thin executable without asking lipo to thin it", () => {
    expect(planMacosSeaCopy("arm64", "arm64\n")).toEqual({ kind: "copy" });
  });

  it("thins a universal executable using lipo's architecture spelling", () => {
    expect(planMacosSeaCopy("x64", "x86_64 arm64\n")).toEqual({
      kind: "thin",
      architecture: "x86_64",
    });
  });

  it("rejects an executable that lacks the current architecture", () => {
    expect(() => planMacosSeaCopy("arm64", "x86_64\n")).toThrow(
      "selected Node.js executable does not contain architecture arm64",
    );
  });
});
