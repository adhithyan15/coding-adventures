import { describe, expect, it } from "vitest";

import { parseMockManifest } from "../src/mock-manifest.js";
import type { TaskShapeInventory } from "../src/task-shapes.js";

const shape = {
  language: "spanish",
  level: "A1",
  sections: [
    { skill: "reading", minutes: 10, parts: [{ id: "r1" }, { id: "r2" }], variants: [] },
    { skill: "listening", minutes: { minimum: 10, maximum: 15 }, parts: [{ id: "l1" }, { id: "l2" }], variants: [
      { id: "short", partIds: ["l1"] }, { id: "long", partIds: ["l2"] },
    ] },
    { skill: "writing", minutes: 20, parts: [{ id: "w1" }], variants: [] },
    { skill: "speaking", minutes: 5, parts: [{ id: "s1" }], variants: [] },
  ],
} as TaskShapeInventory;

function manifest(): Record<string, unknown> {
  return {
    version: 1, language: "spanish", level: "A1", mockId: "a1-mock-1",
    sections: [
      { skill: "reading", minutes: 10, partIds: ["r1", "r2"] },
      { skill: "listening", minutes: 12, variantId: "short", partIds: ["l1"] },
      { skill: "writing", minutes: 20, partIds: ["w1"] },
      { skill: "speaking", minutes: 5, partIds: ["s1"] },
    ],
  };
}

describe("full-mock structure manifest", () => {
  it("accepts one complete form per skill, including a selected variant", () => {
    expect(parseMockManifest(manifest(), "spanish", "A1", "a1-mock-1", shape).sections).toHaveLength(4);
  });

  it.each([
    ["wrong identity", (value: Record<string, unknown>) => { value.mockId = "a1-mock-2"; }, /mockId/],
    ["missing skill", (value: Record<string, unknown>) => { (value.sections as unknown[]).pop(); }, /exactly.*four skills/],
    ["omitted part", (value: Record<string, unknown>) => { ((value.sections as Record<string, unknown>[])[0]!).partIds = ["r1"]; }, /part IDs/],
    ["duplicate part", (value: Record<string, unknown>) => { ((value.sections as Record<string, unknown>[])[0]!).partIds = ["r1", "r1"]; }, /part IDs/],
    ["cross-skill part", (value: Record<string, unknown>) => { ((value.sections as Record<string, unknown>[])[0]!).partIds = ["r1", "l1"]; }, /part IDs/],
    ["wrong variant", (value: Record<string, unknown>) => { ((value.sections as Record<string, unknown>[])[1]!).variantId = "unknown"; }, /variant/],
    ["no variant", (value: Record<string, unknown>) => { delete ((value.sections as Record<string, unknown>[])[1]!).variantId; }, /variant/],
    ["wrong minutes", (value: Record<string, unknown>) => { ((value.sections as Record<string, unknown>[])[0]!).minutes = 11; }, /minutes/],
  ])("rejects %s", (_name, mutate, diagnostic) => {
    const value = manifest();
    mutate(value);
    expect(() => parseMockManifest(value, "spanish", "A1", "a1-mock-1", shape)).toThrow(diagnostic);
  });
});
