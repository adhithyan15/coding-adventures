import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { expect } from "vitest";
import {
  measureContinuity,
  REINFORCEMENT_WINDOWS,
  type ContinuityReport,
  type ReinforcementDefect,
} from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadAssessmentPolicy,
  loadChapterPolicy,
  loadCurriculumSpine,
  loadLanguageCurricula,
  loadTrackLanguageCurriculum,
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

/**
 * Atoms that no later lesson ever practises, although the track carried on for a
 * whole R1 window after the lesson that introduced them.
 *
 * `atomsNeverRevisited` cannot be gated directly, and the shared continuity test
 * explains why: every atom introduced by a track's last lessons is unrevisited
 * until something is appended after it, so a ceiling on the raw count would fail
 * on a legitimate new chapter. The R1 window (n+1 … n+3) is the honest line to
 * draw. An atom with fewer than three lessons after it is the tail, still waiting
 * for its first retrieval; an atom with three or more lessons after it and no
 * retrieval anywhere is an absence — HL09 §7.2's "not a scheduling problem".
 *
 * | lessons after the introducer | practised later? | reported here? |
 * |------------------------------|------------------|----------------|
 * | fewer than 3                 | no               | no — the tail  |
 * | 3 or more                    | no               | **yes**        |
 * | any                          | yes, anywhere    | no             |
 *
 * `report.reinforcement` already lists every atom that misses a judged window,
 * with its total `revisits`; a zero-revisit atom past the tail always misses R1,
 * so filtering that list is enough.
 */
export function atomsNeverRevisitedPastTheTail(
  report: ContinuityReport,
): ReinforcementDefect[] {
  const r1 = REINFORCEMENT_WINDOWS[0].to;
  const lastIndex = new Map(report.tracks.map((track) => [track.language, track.lessonCount - 1]));
  return report.reinforcement.filter(
    (finding) =>
      finding.revisits === 0 && finding.introducedAt + r1 <= (lastIndex.get(finding.language) ?? -1),
  );
}

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
  // CEILING at zero, per track: every atom the track has moved three lessons past
  // must be practised again somewhere. This was 133 corpus-wide before the
  // revisit pass that declared the later lessons already using these atoms and
  // added short spoken recalls where none did; the one atom left
  // (ZH-SKILL-CONNECTED-READING) is introduced by the Chinese track's final lesson,
  // which is the tail this gate deliberately does not judge. A new chapter whose
  // non-final lessons introduce an atom nothing retrieves fails here — practise it
  // in a later lesson of the chapter rather than loosening this.
  expect(
    atomsNeverRevisitedPastTheTail(report).map((finding) => `${finding.atom} (${finding.introducedBy})`),
    `${language} atoms never practised again although the track went on past R1`,
  ).toEqual([]);
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
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons(language, root);
  const curriculum = loadTrackLanguageCurriculum(language, root);
  const curricula = curriculum ? [curriculum] : [];
  return measureWritingStages(
    loadAssessmentPolicy(root),
    [language],
    lessons,
    curricula,
    loadCurriculumSpine(root),
  ).tracks[0]!;
}
