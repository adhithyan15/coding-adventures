import { describe, expect, it } from "vitest";
import { loadAssessmentPolicy, listAssessmentContracts } from "../src/loader.js";
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

  it("loads Marwadi's complete project-defined contract and keeps the other tracks queued", () => {
    expect(listAssessmentContracts()).toEqual(["marwadi"]);
  });
});

describe("track assessment contracts", () => {
  const policy = loadAssessmentPolicy();

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
    expect(parseAssessmentContract(value, "alpha", policy).levels).toHaveLength(7);
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
