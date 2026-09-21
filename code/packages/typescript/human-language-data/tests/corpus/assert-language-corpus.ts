import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadAssessmentPolicy,
  loadChapterPolicy,
  loadEverything,
  loadTrackLessons,
} from "../../src/loader.js";
import { measureLessonBudgets } from "../../src/lesson-budgets.js";
import type { ParsedLesson } from "../../src/parse.js";
import {
  MODALITY_MANIFEST_DIR,
  buildModalityManifest,
} from "../../src/modality-manifest.js";
import { modalityOwnerContents } from "../../src/modality-shards.js";
import { measureRamp } from "../../src/ramp.js";
import { measureWritingStages, type TrackWritingStageCoverage } from "../../src/writing-stages.js";

export function expectLanguageContinuity(language: string): void {
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons(language, root);
  const report = measureContinuity(lessons);
  const ramp = measureRamp(lessons, loadChapterPolicy(root));
  const atomRamp = ramp.tracks[0]!;
  const scriptRamp = ramp.script.tracks[0]!;
  const track = report.tracks[0]!;
  expect(report.tracks, `${language} continuity track closure`).toHaveLength(1);
  expect(track.language).toBe(language);
  expect(track.lessonCount).toBe(lessons.length);
  expect(report.order, `${language} order/dependency defects`).toEqual([]);
  expect(track).toMatchObject({
    lessonsWithoutSequence: 0,
    forwardPrerequisites: 0,
    forwardReviews: 0,
  });
  expect(atomRamp.measurable + atomRamp.unmeasurable).toBe(lessons.length);
  expect(scriptRamp.systemViolations, `${language} writing systems per lesson`).toBe(0);
  expect(
    Object.values(report.summary.missedByWindow).reduce((sum, count) => sum + count, 0),
    `${language} reinforcement-window arithmetic`,
  ).toBe(report.reinforcement.reduce((sum, finding) => sum + finding.missed.length, 0));
}

export function expectLanguageModality(language: string): void {
  const root = defaultCurriculumRoot();
  const expected = modalityOwnerContents(
    buildModalityManifest(loadTrackLessons(language, root)),
  );
  const ownerDirectory = `${language}.d`;
  const expectedNames = [...expected.keys()]
    .map((relative) => relative.slice(`${ownerDirectory}/`.length))
    .sort((left, right) => left.localeCompare(right));
  const actualNames = readdirSync(resolve(root, MODALITY_MANIFEST_DIR, ownerDirectory)).sort(
    (left, right) => left.localeCompare(right),
  );

  expect(actualNames, `${language} modality owner names`).toEqual(expectedNames);
  for (const [relative, contents] of expected) {
    const actual = readFileSync(resolve(root, MODALITY_MANIFEST_DIR, relative), "utf8");
    expect(actual, `${relative} canonical modality owner`).toBe(contents);
  }
}

export interface LanguageLessonBudgetExpectation {
  /**
   * Optional historical pins. Omit generated totals when the invariant is
   * complete measurement and zero excess: the helper derives the lesson count
   * from canonical owners, so adding an independent chapter does not require a
   * shared counter edit.
   */
  readonly lessons?: number;
  readonly idioms?: number;
  readonly senses?: number;
  readonly cultureClaims?: number;
  /** Stable prefix for every declared unit id, for example `GE`. */
  readonly unitPrefix: string;
}

/**
 * Pin one track's completed lesson-content review in that track's own test.
 *
 * The filter is load-bearing: schema-v1 lessons have no declaration contract,
 * so counting them as reviewed zeroes would certify debt that was never read.
 * Keeping the expectation in a language-owned suite lets independent backfill
 * lanes advance without editing one corpus-wide counter. High-churn tracks may
 * omit generated totals and retain the stronger complete-measurement/zero-excess
 * invariant, so independent chapters do not share a per-language counter either.
 */
export function expectLanguageLessonBudgets(
  language: string,
  expected: LanguageLessonBudgetExpectation,
  candidates?: ParsedLesson[],
): void {
  const root = defaultCurriculumRoot();
  const lessons = (candidates ?? loadTrackLessons(language, root)).filter(
    (lesson) =>
      lesson.language === language &&
      (lesson.frontmatter as Record<string, unknown>).schema_version === "2",
  );
  const policy = loadChapterPolicy(root);
  const report = measureLessonBudgets(lessons, {
    idioms: policy.maxNewIdiomsPerLesson ?? 1,
    senses: policy.maxNewSensesPerLesson ?? 1,
    cultureClaims: policy.maxNewCultureClaimsPerLesson ?? 2,
  });

  const lessonCount = expected.lessons ?? lessons.length;
  expect(report.summary, `${language} lesson-content budget coverage`).toMatchObject({
    lessons: lessonCount,
    measuredLessons: lessonCount,
    idiomMeasuredLessons: lessonCount,
    senseMeasuredLessons: lessonCount,
    cultureClaimMeasuredLessons: lessonCount,
    overBudgetLessons: 0,
  });
  if (expected.idioms !== undefined) expect(report.summary.idioms).toBe(expected.idioms);
  if (expected.senses !== undefined) expect(report.summary.senses).toBe(expected.senses);
  if (expected.cultureClaims !== undefined) {
    expect(report.summary.cultureClaims).toBe(expected.cultureClaims);
  }
  expect(report.excesses, `${language} lesson-content budget excesses`).toEqual([]);
  expect(
    report.findings.every((finding) => finding.unitId.startsWith(`${expected.unitPrefix}-`)),
    `${language} lesson-content unit ids use the track prefix`,
  ).toBe(true);
}

export function languageWritingStages(language: string): TrackWritingStageCoverage {
  const { lessons, curricula, spine } = loadEverything();
  return measureWritingStages(
    loadAssessmentPolicy(),
    [language],
    lessons,
    curricula.filter((curriculum) => curriculum.language === language),
    spine,
  ).tracks[0]!;
}
