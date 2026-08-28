import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  listExternalExamCapstones,
  loadAssessmentPolicy,
} from "../src/loader.js";
import { parseAssessmentContract } from "../src/assessment.js";

const temporaryRoots: string[] = [];

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) rmSync(root, { recursive: true, force: true });
});

function contractWith(capstone: Record<string, unknown>) {
  const policy = loadAssessmentPolicy();
  const skill = { taskInventory: ["task-shapes/level.json"], passThreshold: 0.6 };
  return {
    policy,
    value: {
      version: 1,
      language: "alpha",
      externalCapstones: [capstone],
      levels: policy.levels.map((level, levelIndex) => ({
        level,
        target: { name: `Alpha ${level}`, basis: "project-defined", source: "assessment-spec.md" },
        skills: { reading: skill, listening: skill, writing: skill, speaking: skill },
        writingStages: policy.writingStages
          .filter((stage) => policy.levels.indexOf(stage.firstRequiredAt) <= levelIndex)
          .map((stage) => stage.id),
        fullMocks: [
          { id: `${level}-1`, timed: true, rubric: "rubric.md", answerKey: "one.md" },
          { id: `${level}-2`, timed: true, rubric: "rubric.md", answerKey: "two.md" },
        ],
      })),
    },
  };
}

function validCapstone() {
  const skill = { taskInventory: ["capstones/provider.json"], passThreshold: 0.5 };
  return {
    id: "provider-academic",
    target: {
      name: "Provider Academic Exam",
      basis: "external",
      source: "https://provider.example/spec",
      edition: "2026",
      accessed: "2026-08-21",
    },
    requiredAfterLevel: "C2",
    cefrRelation: "not-mapped",
    skills: { reading: skill, listening: skill, writing: skill, speaking: skill },
    fullMocks: [
      {
        id: "provider-1",
        timed: true,
        rubric: "mocks/provider/rubric.md",
        answerKey: "mocks/provider/one.md",
        humanValidation: "mocks/provider/one-validation.md",
      },
      {
        id: "provider-2",
        timed: true,
        rubric: "mocks/provider/rubric.md",
        answerKey: "mocks/provider/two.md",
        humanValidation: "mocks/provider/two-validation.md",
      },
    ],
  };
}

describe("non-CEFR-mapped external exam capstones", () => {
  it("parses a four-skill capstone without turning its dependency level into an equivalence", () => {
    const { policy, value } = contractWith(validCapstone());
    const parsed = parseAssessmentContract(value, "alpha", policy);
    expect(parsed.externalCapstones[0]).toMatchObject({
      id: "provider-academic",
      requiredAfterLevel: "C2",
      cefrRelation: "not-mapped",
      target: { basis: "external" },
    });
    expect(Object.values(parsed.externalCapstones[0]!.skills).map((skill) => skill.passThreshold)).toEqual([
      0.5,
      0.5,
      0.5,
      0.5,
    ]);
  });

  it("rejects an invented CEFR relation", () => {
    const capstone = validCapstone();
    capstone.cefrRelation = "C2";
    const { policy, value } = contractWith(capstone);
    expect(() => parseAssessmentContract(value, "alpha", policy)).toThrow(/cefrRelation must be 'not-mapped'/);
  });

  it("rejects a missing universal skill and an unsafe artifact path", () => {
    const missing = validCapstone();
    delete (missing.skills as Partial<typeof missing.skills>).speaking;
    const one = contractWith(missing);
    expect(() => parseAssessmentContract(one.value, "alpha", one.policy)).toThrow(/skills\.speaking must be an object/);

    const unsafe = validCapstone();
    unsafe.skills.reading.taskInventory = ["../outside.json"];
    const two = contractWith(unsafe);
    expect(() => parseAssessmentContract(two.value, "alpha", two.policy)).toThrow(/safe relative artifact reference/);
  });

  it("keeps a declaration incomplete until every referenced artifact exists", () => {
    const root = mkdtempSync(join(tmpdir(), "hl-external-capstone-"));
    temporaryRoots.push(root);
    mkdirSync(join(root, "core"), { recursive: true });
    mkdirSync(join(root, "alpha", "capstones"), { recursive: true });
    mkdirSync(join(root, "alpha", "mocks", "provider"), { recursive: true });
    copyFileSync(
      join(defaultCurriculumRoot(), "core", "assessment-policy.json"),
      join(root, "core", "assessment-policy.json"),
    );
    writeFileSync(
      join(root, "core", "languages.json"),
      JSON.stringify({ version: 1, languages: [{ id: "alpha" }] }),
    );
    const { value } = contractWith(validCapstone());
    writeFileSync(join(root, "alpha", "assessment.json"), JSON.stringify(value));
    writeFileSync(join(root, "alpha", "capstones", "provider.json"), "{}");
    writeFileSync(join(root, "alpha", "mocks", "provider", "rubric.md"), "# Rubric\n");

    expect(listExternalExamCapstones(root)[0]).toMatchObject({
      language: "alpha",
      id: "provider-academic",
      complete: false,
      missingArtifacts: [
        "mocks/provider/one.md",
        "mocks/provider/one-validation.md",
        "mocks/provider/two.md",
        "mocks/provider/two-validation.md",
      ],
    });

    writeFileSync(join(root, "alpha", "mocks", "provider", "one.md"), "# Key one\n");
    writeFileSync(join(root, "alpha", "mocks", "provider", "two.md"), "# Key two\n");
    expect(listExternalExamCapstones(root)[0]).toMatchObject({ complete: false });
    writeFileSync(join(root, "alpha", "mocks", "provider", "one-validation.md"), "# Pilot one\n");
    writeFileSync(join(root, "alpha", "mocks", "provider", "two-validation.md"), "# Pilot two\n");
    expect(listExternalExamCapstones(root)[0]).toMatchObject({ complete: true, missingArtifacts: [] });
  });
});
