import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../src/loader.js";
import {
  buildSpanishA1MockAudit,
  runSpanishA1MockAudit,
} from "../src/spanish-a1-mock-audit-cli.js";

describe("Spanish A1 book-bounded mock audit", () => {
  it("pins the current whole-item residual and its reproducible credit policy", () => {
    const audit = buildSpanishA1MockAudit();
    expect(audit.objectiveFailed).toBe(7);
    expect(audit.mocks.map(({ reading, listening, objectiveFailed }) => ({
      reading,
      listening,
      objectiveFailed,
    }))).toEqual([
      { reading: 24, listening: 25, objectiveFailed: 1 },
      { reading: 22, listening: 22, objectiveFailed: 6 },
    ]);
    expect(audit.missingObjectiveLexemes).toHaveLength(14);
    expect(audit.policy.citationFormCredits).toContain("llamarse");
  });

  it("keeps the committed report canonical and current", () => {
    expect(runSpanishA1MockAudit(["--check"], defaultCurriculumRoot())).toBe(0);
  });
});
