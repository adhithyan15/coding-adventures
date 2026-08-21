import { readFileSync } from "node:fs";
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
import {
  MODALITY_MANIFEST_DIR,
  buildModalityManifest,
  serializeModalityManifest,
} from "../../src/modality-manifest.js";
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
  const expected = serializeModalityManifest(buildModalityManifest(loadTrackLessons(language, root)));
  const actual = readFileSync(resolve(root, MODALITY_MANIFEST_DIR, `${language}.json`), "utf8");
  expect(actual, `${language} modality manifest`).toBe(expected);
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
