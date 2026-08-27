import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { parseAssessmentContract } from "../../src/assessment.js";
import {
  defaultCurriculumRoot,
  listAssessmentContracts,
  loadAssessmentPolicy,
} from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";

it("pins Russian continuity", () => expectLanguageContinuity("russian"));
it("pins Russian modality", () => expectLanguageModality("russian"));
it("pins Russian lesson-content budgets", () =>
  expectLanguageLessonBudgets("russian", {
    lessons: 88,
    idioms: 0,
    senses: 4,
    cultureClaims: 9,
    unitPrefix: "RU",
  }));

it("pins Russia's project pre-A1 bridge and external A1-to-C2 TORFL targets", () => {
  const policy = loadAssessmentPolicy();
  const russian = parseAssessmentContract(
    JSON.parse(readFileSync(join(defaultCurriculumRoot(), "russian", "assessment.json"), "utf8")),
    "russian",
    policy,
  );

  expect(listAssessmentContracts()).toContain("russian");
  expect(russian.levels.map((level) => level.level)).toEqual(policy.levels);
  expect(russian.levels[0]?.target.basis).toBe("project-defined");
  expect(russian.levels.slice(1).every((level) => level.target.basis === "external")).toBe(true);
  expect(russian.levels[0]?.skills.reading.passThreshold).toBe(0.6);
  expect(russian.levels[0]?.additionalComponents).toEqual({});
  expect(russian.levels.slice(1).every((level) =>
    Object.values(level.skills).every((skill) => skill.passThreshold === 0.66)
  )).toBe(true);
  expect(russian.levels.slice(1).every((level) =>
    level.additionalComponents["lexis-grammar"]?.passThreshold === 0.66
  )).toBe(true);
  expect(russian.levels[0]?.writingStages).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
  expect(russian.levels.at(-1)?.writingStages).toEqual(policy.writingStages.map((stage) => stage.id));
  expect(russian.levels.every((level) => level.fullMocks.length === 2)).toBe(true);
});
