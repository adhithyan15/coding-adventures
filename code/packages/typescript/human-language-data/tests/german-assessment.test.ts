import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  listAssessmentContracts,
  loadAssessmentPolicy,
  loadTaskShapeInventory,
} from "../src/loader.js";
import { parseAssessmentContract } from "../src/assessment.js";

describe("German assessment contract", () => {
  const policy = loadAssessmentPolicy();
  const contract = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "german", "assessment.json"), "utf8")),
    "german",
    policy,
  );

  it("maps pre-A1 to a project bridge and A1-C2 to adult Goethe destinations", () => {
    expect(contract.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(contract.levels.map((level) => level.target.basis)).toEqual([
      "project-defined", "external", "external", "external", "external", "external", "external",
    ]);
    expect(contract.levels.map((level) => level.target.name)).toEqual([
      "Coding Adventures German pre-A1 Assessment — project-defined Goethe precursor",
      "Goethe-Zertifikat A1: Start Deutsch 1",
      "Goethe-Zertifikat A2",
      "Goethe-Zertifikat B1",
      "Goethe-Zertifikat B2",
      "Goethe-Zertifikat C1",
      "Goethe-Zertifikat C2: Großes Deutsches Sprachdiplom",
    ]);
  });

  it("requires two mocks and an independent 60-percent readiness floor per skill", () => {
    expect(contract.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
    expect(contract.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6),
    )).toBe(true);
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

  it("binds the contract to the existing external A1 task inventory", () => {
    expect(loadTaskShapeInventory("german", "A1").target).toEqual({
      name: contract.levels[1]?.target.name,
      basis: "external",
    });
    expect(listAssessmentContracts()).toContain("german");
  });
});
