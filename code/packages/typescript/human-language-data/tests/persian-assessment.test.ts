import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  listExternalExamCapstones,
  loadAssessmentPolicy,
} from "../src/loader.js";
import { parseAssessmentContract } from "../src/assessment.js";

describe("Persian assessment contract", () => {
  const policy = loadAssessmentPolicy();
  const contract = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "persian", "assessment.json"), "utf8")),
    "persian",
    policy,
  );

  it("defines all seven curriculum rungs as independent four-skill destinations", () => {
    expect(contract.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(contract.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
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
    expect(contract.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
  });

  it("requires SAMFA Academic after C2 without claiming that SAMFA is C2", () => {
    expect(contract.externalCapstones).toHaveLength(1);
    const samfa = contract.externalCapstones[0]!;
    expect(samfa).toMatchObject({
      id: "samfa-academic",
      requiredAfterLevel: "C2",
      cefrRelation: "not-mapped",
      target: { name: "SAMFA Academic", basis: "external", edition: "11th administration, 2025" },
    });
    expect(Object.values(samfa.skills).map((skill) => skill.passThreshold)).toEqual([
      0.5,
      0.5,
      0.5,
      0.5,
    ]);
  });

  it("keeps the SAMFA task and mock artifacts visible as incomplete backlog", () => {
    expect(listExternalExamCapstones().find((item) => item.language === "persian")).toMatchObject({
      id: "samfa-academic",
      complete: false,
      missingArtifacts: [
        "fullMocks[samfa-academic-mock-1].humanValidation (not declared)",
        "fullMocks[samfa-academic-mock-2].humanValidation (not declared)",
        "capstones/samfa-academic.json",
        "mocks/samfa-academic/rubric.md",
        "mocks/samfa-academic/mock-1-answer-key.md",
        "mocks/samfa-academic/mock-2-answer-key.md",
      ],
    });
  });
});
