import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadAssessmentPolicy } from "../src/loader.js";
import { parseAssessmentContract } from "../src/assessment.js";

describe("Latin assessment contract", () => {
  const policy = loadAssessmentPolicy();
  const contract = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "latin", "assessment.json"), "utf8")),
    "latin",
    policy,
  );

  it("defines every rung as an honest project-defined four-skill destination", () => {
    expect(contract.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(contract.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
    expect(contract.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6),
    )).toBe(true);
    expect(contract.levels.every((level) => Object.keys(level.additionalComponents).length === 0)).toBe(true);
  });

  it("requires the gentle writing ramp cumulatively and two timed mocks per rung", () => {
    expect(contract.levels[0]?.writingStages).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(contract.levels[1]?.writingStages).toContain("controlled-composition");
    expect(contract.levels.at(-1)?.writingStages).toEqual(
      policy.writingStages.map((stage) => stage.id),
    );
    expect(contract.levels.every((level) =>
      level.fullMocks.length === 2 && level.fullMocks.every((mock) => mock.timed),
    )).toBe(true);
  });

  it("keeps the existing A1 inventory connected without claiming readiness", () => {
    const a1 = contract.levels.find((level) => level.level === "A1");
    expect(a1?.skills.reading.taskInventory).toEqual(["task-shapes/a1.json#reading"]);
    expect(a1?.skills.listening.taskInventory).toEqual(["task-shapes/a1.json#listening"]);
    expect(a1?.skills.writing.taskInventory).toEqual(["task-shapes/a1.json#writing"]);
    expect(a1?.skills.speaking.taskInventory).toEqual(["task-shapes/a1.json#speaking"]);
  });
});
