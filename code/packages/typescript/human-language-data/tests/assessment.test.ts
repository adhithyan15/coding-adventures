import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  loadAssessmentPolicy,
  loadLanguageRegistry,
  listAssessmentContracts,
} from "../src/loader.js";
import { parseAssessmentContract, parseAssessmentPolicy } from "../src/assessment.js";

describe("assessment policy (HL16)", () => {
  it("pins five-minute lessons, four independent skills, and the complete writing ramp", () => {
    const policy = loadAssessmentPolicy();
    expect(policy.maxLessonMinutes).toBe(5);
    expect(policy.skills).toEqual(["reading", "listening", "writing", "speaking"]);
    expect(policy.writingStages.map((stage) => stage.id)).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
      "controlled-composition",
      "connected-composition",
      "timed-assessment-production",
    ]);
    expect(policy.passEvidence).toMatchObject({
      skillsPassIndependently: true,
      minimumFullMocksPerLevel: 2,
      requiresTimedMocks: true,
      requiresRubric: true,
      requiresAnswerKey: true,
      requiresHumanValidation: true,
    });
  });

  it("does not let policy drift above five minutes", () => {
    expect(() => parseAssessmentPolicy({
      version: 1,
      maxLessonMinutes: 6,
      levels: ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"],
      skills: ["reading", "listening", "writing", "speaking"],
      writingStages: [],
      passEvidence: {},
    })).toThrow(/maxLessonMinutes must be 5/);
  });

  it("discovers valid contracts in registry order without making every new track edit one exact list", () => {
    const contracts = listAssessmentContracts();
    const registryOrder = loadLanguageRegistry().languages.map((track) => track.id);
    expect(contracts).toEqual(expect.arrayContaining([
      "french",
      "marathi",
      "marwadi",
      "punjabi",
      "gujarati",
      "japanese",
    ]));
    expect(new Set(contracts).size).toBe(contracts.length);
    expect(contracts).toEqual([...contracts].sort(
      (left, right) => registryOrder.indexOf(left) - registryOrder.indexOf(right),
    ));
  });
});

describe("track assessment contracts", () => {
  const policy = loadAssessmentPolicy();

  it("loads Marathi's seven-rung independent four-skill destination", () => {
    const marathi = parseAssessmentContract(
      JSON.parse(readFileSync(join(defaultCurriculumRoot(), "marathi", "assessment.json"), "utf8")),
      "marathi",
      policy,
    );
    expect(marathi.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(marathi.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
    expect(marathi.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6)
    )).toBe(true);
    expect(marathi.levels[0]?.writingStages).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(marathi.levels.at(-1)?.writingStages).toEqual(policy.writingStages.map((stage) => stage.id));
    expect(marathi.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
  });

  it("loads Punjabi's seven-rung independent four-skill destination", () => {
    const punjabi = parseAssessmentContract(
      JSON.parse(readFileSync(join(defaultCurriculumRoot(), "punjabi", "assessment.json"), "utf8")),
      "punjabi",
      policy,
    );
    expect(punjabi.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(punjabi.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
    expect(punjabi.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6)
    )).toBe(true);
    expect(punjabi.levels[0]?.writingStages).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(punjabi.levels.at(-1)?.writingStages).toEqual(policy.writingStages.map((stage) => stage.id));
    expect(punjabi.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
  });

  it("loads Japanese's seven-rung independent four-skill destination", () => {
    const japanese = parseAssessmentContract(
      JSON.parse(readFileSync(join(defaultCurriculumRoot(), "japanese", "assessment.json"), "utf8")),
      "japanese",
      policy,
    );
    expect(japanese.levels.map((level) => level.level)).toEqual(policy.levels);
    expect(japanese.levels.every((level) => level.target.basis === "project-defined")).toBe(true);
    expect(japanese.levels.every((level) =>
      Object.values(level.skills).every((skill) => skill.passThreshold === 0.6)
    )).toBe(true);
    expect(japanese.levels[0]?.writingStages).toEqual([
      "observe-trace",
      "guided-copy",
      "delayed-copy",
      "dictation-transcription",
    ]);
    expect(japanese.levels.at(-1)?.writingStages).toEqual(policy.writingStages.map((stage) => stage.id));
    expect(japanese.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
  });

  it("accepts a complete seven-level contract with independent skills and full mocks", () => {
    const skill = { taskInventory: ["tasks.json"], passThreshold: 0.6 };
    const value = {
      version: 1,
      language: "alpha",
      levels: policy.levels.map((current, levelIndex) => ({
        level: current,
        target: { name: `Alpha ${current}`, basis: "project-defined", source: "assessment-spec.md" },
        skills: { reading: skill, listening: skill, writing: skill, speaking: skill },
        writingStages: policy.writingStages
          .filter((stage) => policy.levels.indexOf(stage.firstRequiredAt) <= levelIndex)
          .map((stage) => stage.id),
        fullMocks: [
          { id: `${current}-mock-1`, timed: true, rubric: "rubric.md", answerKey: "answers-1.md" },
          { id: `${current}-mock-2`, timed: true, rubric: "rubric.md", answerKey: "answers-2.md" },
        ],
      })),
    };
    const parsed = parseAssessmentContract(value, "alpha", policy);
    expect(parsed.levels).toHaveLength(7);
    expect(parsed.levels.every((level) => Object.keys(level.additionalComponents).length === 0)).toBe(true);
  });

  it("parses independently required provider components beyond the four-skill floor", () => {
    const skill = { taskInventory: ["tasks.json"], passThreshold: 0.66 };
    const value = {
      version: 1,
      language: "alpha",
      levels: policy.levels.map((current, levelIndex) => ({
        level: current,
        target: { name: `Alpha ${current}`, basis: "external", source: "assessment-spec.md" },
        skills: { reading: skill, listening: skill, writing: skill, speaking: skill },
        additionalComponents: {
          "lexis-grammar": {
            name: "Lexis. Grammar",
            taskInventory: [`task-shapes/${current}.json#lexis-grammar`],
            passThreshold: 0.66,
          },
        },
        writingStages: policy.writingStages
          .filter((stage) => policy.levels.indexOf(stage.firstRequiredAt) <= levelIndex)
          .map((stage) => stage.id),
        fullMocks: [
          { id: `${current}-mock-1`, timed: true, rubric: "rubric.md", answerKey: "answers-1.md" },
          { id: `${current}-mock-2`, timed: true, rubric: "rubric.md", answerKey: "answers-2.md" },
        ],
      })),
    };
    const parsed = parseAssessmentContract(value, "alpha", policy);
    expect(parsed.levels[0]?.additionalComponents["lexis-grammar"]).toEqual({
      name: "Lexis. Grammar",
      taskInventory: ["task-shapes/pre-A1.json#lexis-grammar"],
      passThreshold: 0.66,
    });
  });

  it("rejects additional components that shadow a universal skill", () => {
    const skill = { taskInventory: ["tasks.json"], passThreshold: 0.6 };
    const one = {
      version: 1,
      language: "alpha",
      levels: [{
        level: "pre-A1",
        target: { name: "Alpha checkpoint", basis: "external", source: "alpha.md" },
        skills: { reading: skill, listening: skill, writing: skill, speaking: skill },
        additionalComponents: {
          reading: { name: "Second reading", taskInventory: ["extra.json"], passThreshold: 0.6 },
        },
        writingStages: ["observe-trace", "guided-copy", "delayed-copy", "dictation-transcription"],
        fullMocks: [
          { id: "mock-1", timed: true, rubric: "rubric.md", answerKey: "answers-1.md" },
          { id: "mock-2", timed: true, rubric: "rubric.md", answerKey: "answers-2.md" },
        ],
      }],
    };
    expect(() => parseAssessmentContract(one, "alpha", policy)).toThrow(/duplicates a universal skill/);
  });

  it("rejects a declared provider component without a task inventory", () => {
    const skill = { taskInventory: ["tasks.json"], passThreshold: 0.6 };
    const one = {
      version: 1,
      language: "alpha",
      levels: [{
        level: "pre-A1",
        target: { name: "Alpha checkpoint", basis: "external", source: "alpha.md" },
        skills: { reading: skill, listening: skill, writing: skill, speaking: skill },
        additionalComponents: {
          "lexis-grammar": { name: "Lexis. Grammar", taskInventory: [], passThreshold: 0.66 },
        },
        writingStages: ["observe-trace", "guided-copy", "delayed-copy", "dictation-transcription"],
        fullMocks: [
          { id: "mock-1", timed: true, rubric: "rubric.md", answerKey: "answers-1.md" },
          { id: "mock-2", timed: true, rubric: "rubric.md", answerKey: "answers-2.md" },
        ],
      }],
    };
    expect(() => parseAssessmentContract(one, "alpha", policy)).toThrow(/taskInventory must be a non-empty string array/);
  });

  it("requires all four independently scored skills", () => {
    const skill = { taskInventory: [], passThreshold: 0.6 };
    const one = {
      version: 1,
      language: "alpha",
      levels: [{
        level: "pre-A1",
        target: { name: "Alpha checkpoint", basis: "project-defined", source: "alpha.md" },
        skills: { listening: skill, writing: skill, speaking: skill },
        writingStages: ["observe-trace", "guided-copy", "delayed-copy", "dictation-transcription"],
        fullMocks: [
          { id: "mock-1", timed: true, rubric: "rubric.md", answerKey: "answers-1.md" },
          { id: "mock-2", timed: true, rubric: "rubric.md", answerKey: "answers-2.md" },
        ],
      }],
    };
    expect(() => parseAssessmentContract(one, "alpha", policy)).toThrow(/skills\.reading must be an object/);
  });

  it("rejects a contract that calls a project equivalent external", () => {
    const one = {
      version: 1,
      language: "alpha",
      levels: [{
        level: "pre-A1",
        target: { name: "Alpha checkpoint", basis: "invented", source: "alpha.md" },
        skills: {},
        writingStages: [],
        fullMocks: [],
      }],
    };
    expect(() => parseAssessmentContract(one, "alpha", policy)).toThrow(/external or project-defined/);
  });
});
