import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  listAssessmentContracts,
  listHumanValidationStatuses,
  loadAssessmentPolicy,
  loadTaskShapeInventory,
} from "../src/loader.js";
import { parseAssessmentContract } from "../src/assessment.js";

describe("Bengali assessment contract", () => {
  const policy = loadAssessmentPolicy();
  const contract = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "bengali", "assessment.json"), "utf8")),
    "bengali",
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

  it("binds every skill to its rung-owned inventory and two real timed mock artifacts", () => {
    for (const level of contract.levels) {
      const inventory = loadTaskShapeInventory("bengali", level.level);
      expect(inventory.target).toEqual({ name: level.target.name, basis: "project-defined" });
      for (const [skill, binding] of Object.entries(level.skills)) {
        expect(binding.taskInventory).toEqual([`task-shapes/${level.level.toLowerCase()}.json#${skill}`]);
      }
      expect(level.fullMocks.every((mock) => mock.timed)).toBe(true);
      expect(level.fullMocks.every((mock) => mock.humanValidation === undefined)).toBe(true);
    }
  });

  it("starts writing at pre-A1 and accumulates the entire gentle ramp by C2", () => {
    expect(contract.levels[0]?.writingStages).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(contract.levels[1]?.writingStages).toContain("controlled-composition");
    expect(contract.levels[2]?.writingStages).toContain("connected-composition");
    expect(contract.levels.at(-1)?.writingStages).toEqual(
      policy.writingStages.map((stage) => stage.id),
    );
  });

  it("clears the contract gap while retaining every human-validation gap", () => {
    expect(listAssessmentContracts()).toContain("bengali");
    expect(listHumanValidationStatuses().filter((status) => status.language === "bengali"))
      .toEqual(contract.levels.map((level) => ({
        language: "bengali",
        level: level.level,
        requiredMocks: 2,
        validatedMocks: 0,
        missingMocks: level.fullMocks.map((mock) => mock.id),
      })));
  });
});
