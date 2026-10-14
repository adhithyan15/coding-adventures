import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { auditTrackArtifacts } from "../src/assessment-artifacts.js";
import { parseAssessmentContract } from "../src/assessment.js";
import {
  defaultCurriculumRoot,
  listAssessmentContracts,
  listHumanValidationStatuses,
  loadAssessmentPolicy,
  loadTaskShapeInventory,
} from "../src/loader.js";

describe("Sanskrit assessment contract", () => {
  const root = defaultCurriculumRoot();
  const policy = loadAssessmentPolicy();
  const contract = parseAssessmentContract(
    JSON.parse(readFileSync(join(root, "sanskrit", "assessment.json"), "utf8")),
    "sanskrit",
    policy,
  );

  it("defines all seven policy rungs with independent four-skill thresholds", () => {
    expect(contract.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(contract.externalCapstones).toEqual([]);
    expect(contract.levels.every((level) =>
      Object.keys(level.additionalComponents).length === 0
    )).toBe(true);
    expect(contract.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
    expect(contract.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6)
    )).toBe(true);
  });

  it("binds every skill and timed mock to its real level-owned source artifact", () => {
    for (const level of contract.levels) {
      const slug = level.level.toLowerCase();
      const inventory = loadTaskShapeInventory("sanskrit", level.level);
      expect(inventory.target).toEqual({ name: level.target.name, basis: "project-defined" });
      expect(level.target.source).toBe(`assessment-spec.md#${slug}`);
      for (const [skill, binding] of Object.entries(level.skills)) {
        expect(binding.taskInventory).toEqual([`task-shapes/${slug}.json#${skill}`]);
      }
      expect(level.fullMocks).toEqual([1, 2].map((form) => ({
        id: `${slug}-mock-${form}`,
        timed: true,
        rubric: `mocks/${slug}/rubric.md`,
        answerKey: `mocks/${slug}/mock-${form}-answer-key.md`,
      })));
    }
  });

  it("accumulates exactly the policy writing stages at each rung", () => {
    for (const level of contract.levels) {
      expect(level.writingStages).toEqual(policy.writingStages
        .filter((stage) => policy.levels.indexOf(stage.firstRequiredAt)
          <= policy.levels.indexOf(level.level))
        .map((stage) => stage.id));
    }
  });

  it("has no dangling source promises and leaves human-validation debt explicit", () => {
    expect(auditTrackArtifacts(root, "sanskrit", contract).missing).toEqual([]);
    expect(listAssessmentContracts()).toContain("sanskrit");
    expect(listHumanValidationStatuses().filter((status) => status.language === "sanskrit"))
      .toEqual(contract.levels.map((level) => ({
        language: "sanskrit",
        level: level.level,
        requiredMocks: 2,
        validatedMocks: 0,
        missingMocks: level.fullMocks.map((mock) => mock.id),
      })));
  });

  it("keeps every rubric explicit that activation is not readiness evidence", () => {
    const specification = readFileSync(
      join(root, "sanskrit", "assessment-spec.md"),
      "utf8",
    ).replace(/\s+/gu, " ");
    expect(specification).toContain("source-artifact contract active");
    expect(specification).toContain("not an external qualification");
    expect(specification).toContain("not qualified recordings");
    expect(specification).toContain("independent human review");
    expect(specification).toContain("standard setting");
    expect(specification).toContain("book-only validation");
    expect(specification).toContain("readiness");
    expect(specification).not.toContain("No `assessment.json` is checked in");
    expect(specification).not.toContain("`assessment.json` remains absent");
    for (const level of policy.levels) {
      const rubric = readFileSync(
        join(root, "sanskrit", "mocks", level.toLowerCase(), "rubric.md"),
        "utf8",
      ).replace(/\s+/gu, " ");
      expect(rubric).toContain("`assessment.json` is active as a source-artifact contract");
      expect(rubric).not.toContain("`assessment.json` remains absent");
    }
  });
});
