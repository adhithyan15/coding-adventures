import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("keeps the Punjabi changelog free of literal patch markup", () => {
  const path = "code/learning/human-languages/punjabi/CHANGELOG.md";
  const plan = DOC_SHARD_PLANS.find((candidate) => candidate.path === path);
  expect(plan).toBeDefined();
  const changelog = unshardDocContents(
    defaultRepoRoot(),
    plan!,
  );
  expect(changelog).not.toMatch(/^@@$/m);
  expect(changelog).not.toMatch(/^\+##/m);
  expect(changelog.indexOf("Punjabi A1 phone-field writing ladder"))
    .toBeLessThan(changelog.indexOf("Punjabi A1 age-field writing ladder"));
});
