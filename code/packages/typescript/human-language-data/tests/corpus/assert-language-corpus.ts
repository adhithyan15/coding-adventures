import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect } from "vitest";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../src/continuity.js";
import type { TrackGentleRamp } from "../../src/gentle-ramp.js";
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
  const snapshot = JSON.parse(
    readFileSync(resolve(root, "core", "gentle-ramp-snapshots", `${language}.json`), "utf8"),
  ) as TrackGentleRamp;
  const reinforcementMissesByWindow = Object.fromEntries(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, report.summary.missedByWindow[window.name]]),
  );

  expect(
    {
      language: track.language,
      lessonCount: track.lessonCount,
      orderDefects: report.order.length,
      lessonsWithoutSequence: track.lessonsWithoutSequence,
      forwardPrerequisites: track.forwardPrerequisites,
      forwardReviews: track.forwardReviews,
      forwardReferences: track.forwardReferences,
      atomsTaught: track.atomsTaught,
      atomsNeverRevisited: track.atomsNeverRevisited,
      reinforcementWindowMisses: Object.values(report.summary.missedByWindow).reduce(
        (sum, count) => sum + count,
        0,
      ),
      reinforcementMissesByWindow,
      atomMeasurementBlindLessons: atomRamp.unmeasurable,
      atomLessonSpikes: atomRamp.lessonViolations,
      atomChapterSpikes: atomRamp.chapterViolations,
      glyphLessonSpikes: scriptRamp.lessonViolations,
      scriptSystemSpikes: scriptRamp.systemViolations,
    },
    `${language} continuity ledger`,
  ).toEqual({
    language: snapshot.language,
    lessonCount: snapshot.lessonCount,
    orderDefects: snapshot.orderDefects,
    lessonsWithoutSequence: snapshot.lessonsWithoutSequence,
    forwardPrerequisites: snapshot.forwardPrerequisites,
    forwardReviews: snapshot.forwardReviews,
    forwardReferences: snapshot.forwardReferences,
    atomsTaught: snapshot.atomsTaught,
    atomsNeverRevisited: snapshot.atomsNeverRevisited,
    reinforcementWindowMisses: snapshot.reinforcementWindowMisses,
    reinforcementMissesByWindow: snapshot.reinforcementMissesByWindow,
    atomMeasurementBlindLessons: snapshot.atomMeasurementBlindLessons,
    atomLessonSpikes: snapshot.atomLessonSpikes,
    atomChapterSpikes: snapshot.atomChapterSpikes,
    glyphLessonSpikes: snapshot.glyphLessonSpikes,
    scriptSystemSpikes: snapshot.scriptSystemSpikes,
  });
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
  /** The schema-v2 lessons owned by this track and therefore reviewable. */
  readonly lessons: number;
  readonly idioms: number;
  readonly senses: number;
  readonly cultureClaims: number;
  /** Stable prefix for every declared unit id, for example `GE`. */
  readonly unitPrefix: string;
}

/**
 * Pin one track's completed lesson-content review in that track's own test.
 *
 * The filter is load-bearing: schema-v1 lessons have no declaration contract,
 * so counting them as reviewed zeroes would certify debt that was never read.
 * Keeping the expectation in `<track>.test.ts` lets six independent backfill
 * lanes advance without editing one corpus-wide counter.
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

  expect(report.summary, `${language} lesson-content budget coverage`).toEqual({
    lessons: expected.lessons,
    measuredLessons: expected.lessons,
    idiomMeasuredLessons: expected.lessons,
    senseMeasuredLessons: expected.lessons,
    cultureClaimMeasuredLessons: expected.lessons,
    idioms: expected.idioms,
    senses: expected.senses,
    cultureClaims: expected.cultureClaims,
    overBudgetLessons: 0,
  });
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
