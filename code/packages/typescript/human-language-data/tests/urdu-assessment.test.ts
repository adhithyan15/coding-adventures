import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  listAssessmentContracts,
  loadAssessmentPolicy,
} from "../src/loader.js";
import { parseAssessmentContract } from "../src/assessment.js";

describe("Urdu assessment contract", () => {
  const policy = loadAssessmentPolicy();
  const contract = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "urdu", "assessment.json"), "utf8")),
    "urdu",
    policy,
  );

  it("defines all seven rungs as independent project-owned four-skill destinations", () => {
    expect(contract.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(contract.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
    expect(contract.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6),
    )).toBe(true);
    expect(contract.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
  });

  it("starts writing at pre-A1 and accumulates the entire writing ramp by C2", () => {
    expect(contract.levels[0]?.writingStages).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(contract.levels.at(-1)?.writingStages).toEqual(
      policy.writingStages.map((stage) => stage.id),
    );
  });

  it("is discoverable without adding Urdu to a shared exact-list assertion", () => {
    expect(listAssessmentContracts()).toContain("urdu");
  });
});
