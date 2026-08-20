// ---------------------------------------------------------------------------
// gentle-ramp.ts -- one corpus-wide view of the learner-facing cliffs.
// ---------------------------------------------------------------------------
//
// Human Languages already measures duration, knowledge-atom load, script load,
// continuity, script closure, modality and chapter payoffs. Before HL17 those
// measurements lived in separate report sections. That made each number honest
// and the backlog difficult to prioritize: an author had to mentally join seven
// tables before answering the simple question "where is this track least gentle?"
//
// This module performs that join. It deliberately does NOT collapse unlike debt
// into a score. Ten untaught glyphs are not "better" than eleven unrevisited
// atoms, and a weighted average would let a catastrophic first-lesson cliff hide
// behind hundreds of calm later lessons. Instead every finding keeps its unit and
// a fixed learner-first priority orders the work queue.

import type { ChapterGateReport } from "./chapters.js";
import type { ContinuityReport } from "./continuity.js";
import type { ModalitySummary } from "./modality.js";
import type { ParsedLesson } from "./parse.js";
import { readingOrder } from "./ramp.js";
import type { RampReport } from "./ramp.js";
import type { ScriptClosureReport } from "./script-closure.js";

export const GENTLE_RAMP_PRIORITIES = [
  "duration",
  "order-integrity",
  "forward-language",
  "script-closure",
  "writing-ramp",
  "atom-step",
  "glyph-step",
  "reinforcement",
  "payoff-surprise",
  "measurement-blind",
] as const;

export type GentleRampFindingKind = (typeof GENTLE_RAMP_PRIORITIES)[number];

export interface GentleRampFinding {
  kind: GentleRampFindingKind;
  language: string;
  /** Count in the named unit. Unlike debt is never added together. */
  count: number;
  unit: string;
  detail: string;
}

export interface TrackGentleRamp {
  language: string;
  lessonCount: number;
  /** Lessons with declared knowledge atoms; the atom ramp can judge these. */
  atomMeasurableLessons: number;
  /** A blind spot, never evidence that the remaining lessons are gentle. */
  atomMeasurementBlindLessons: number;
  durationViolations: number;
  atomLessonSpikes: number;
  atomChapterSpikes: number;
  glyphLessonSpikes: number;
  scriptSystemSpikes: number;
  scriptClosureViolations: number;
  neverTaughtGlyphs: number;
  orderDefects: number;
  unknownPrerequisites: number;
  forwardPrerequisites: number;
  forwardReviews: number;
  forwardReferences: number;
  atomsTaught: number;
  atomsNeverRevisited: number;
  reinforcementWindowMisses: number;
  payoffSurprises: number;
  writingPracticeLessons: number;
  /** Zero-based reading position, or null when the track never asks for a pen. */
  firstWritingPracticeAt: number | null;
  lessonsBeforeWritingPractice: number;
  findings: GentleRampFinding[];
  /** The first named debt in learner-first order, or null when none was detected. */
  next: GentleRampFinding | null;
}

export interface GentleRampReport {
  schemaVersion: 1;
  rule: {
    maxLessonSeconds: 300;
    durationBoundary: "strictly-greater-than";
    atomMeasurement: "declared-atoms-only";
    ranking: "learner-first-named-debt-no-composite-score";
  };
  tracks: TrackGentleRamp[];
  /** One item per (track, debt kind), ready to become a remediation tranche. */
  workQueue: GentleRampFinding[];
  summary: {
    tracks: number;
    tracksWithDetectedCliffs: number;
    tracksWithNoWritingPractice: number;
    tracksWhereWritingStartsLate: number;
    atomMeasurementBlindLessons: number;
    findings: number;
  };
}

export interface GentleRampInput {
  languages: readonly string[];
  lessons: readonly ParsedLesson[];
  durationViolations: readonly { language: string }[];
  unknownPrerequisites: readonly { language: string }[];
  ramp: RampReport;
  continuity: ContinuityReport;
  scriptClosure: ScriptClosureReport;
  modality: ModalitySummary;
  chapters?: ChapterGateReport;
}

const priority = new Map(GENTLE_RAMP_PRIORITIES.map((kind, index) => [kind, index]));

function isWritingPractice(lesson: ParsedLesson): boolean {
  if (lesson.realization.type === "writing") return true;
  if (lesson.frontmatter.delivery === "script") return true;
  return (lesson.blocks ?? []).some((block) => block.type === "writing" || block.type === "script");
}

function finding(
  language: string,
  kind: GentleRampFindingKind,
  count: number,
  unit: string,
  detail: string,
): GentleRampFinding | null {
  return count > 0 ? { language, kind, count, unit, detail } : null;
}

function byLanguage<T extends { language: string }>(entries: readonly T[]): Map<string, T[]> {
  const out = new Map<string, T[]>();
  for (const entry of entries) {
    const bucket = out.get(entry.language);
    if (bucket) bucket.push(entry);
    else out.set(entry.language, [entry]);
  }
  return out;
}

/** Join the existing gates without changing or averaging their units. */
export function summarizeGentleRamp(input: GentleRampInput): GentleRampReport {
  const lessons = byLanguage(input.lessons);
  const durations = byLanguage(input.durationViolations);
  const unknownPrerequisites = byLanguage(input.unknownPrerequisites);
  const order = byLanguage(input.continuity.order);
  const reinforcement = byLanguage(input.continuity.reinforcement);
  const forward = byLanguage(input.continuity.forwardReferences);
  const payoff = byLanguage(
    (input.chapters?.findings ?? []).filter(
      (entry) =>
        entry.code === "chapter-payoff-not-closed" ||
        entry.code === "chapter-payoff-not-representative" ||
        entry.code === "chapter-missing-capability",
    ),
  );
  const rampByTrack = new Map(input.ramp.tracks.map((track) => [track.language, track]));
  const scriptRampByTrack = new Map(input.ramp.script.tracks.map((track) => [track.language, track]));
  const continuityByTrack = new Map(input.continuity.tracks.map((track) => [track.language, track]));
  const closureByTrack = new Map(input.scriptClosure.tracks.map((track) => [track.language, track]));
  const modalityByTrack = new Map(input.modality.tracks.map((track) => [track.language, track]));

  const tracks: TrackGentleRamp[] = [];
  for (const language of [...new Set(input.languages)].sort()) {
    const ordered = [...(lessons.get(language) ?? [])].sort(readingOrder);
    const ramp = rampByTrack.get(language);
    const scriptRamp = scriptRampByTrack.get(language);
    const continuity = continuityByTrack.get(language);
    const closure = closureByTrack.get(language);
    const modality = modalityByTrack.get(language);
    const trackOrder = order.get(language) ?? [];
    const writingPositions = ordered.flatMap((lesson, index) => (isWritingPractice(lesson) ? [index] : []));
    const firstWritingPracticeAt = writingPositions[0] ?? null;
    const orderDefects = trackOrder.length;
    const forwardPrerequisites = trackOrder.filter((entry) => entry.kind === "forward-prerequisite").length;
    const forwardReviews = trackOrder.filter((entry) => entry.kind === "forward-review").length;
    const reinforcementWindowMisses = (reinforcement.get(language) ?? []).reduce(
      (sum, entry) => sum + entry.missed.length,
      0,
    );

    const findings = [
      finding(
        language,
        "duration",
        durations.get(language)?.length ?? 0,
        "lesson(s)",
        "split lessons whose effective duration exceeds the five-minute maximum",
      ),
      finding(
        language,
        "order-integrity",
        orderDefects + (unknownPrerequisites.get(language)?.length ?? 0),
        "order/dependency defect(s)",
        "declare a unique reading order and close every prerequisite and review before use",
      ),
      finding(
        language,
        "forward-language",
        forward.get(language)?.length ?? 0,
        "use(s)",
        "teach target-language material before load-bearing use",
      ),
      finding(
        language,
        "script-closure",
        closure?.neverTaughtGlyphs ?? 0,
        "glyph(s)",
        "teach every load-bearing glyph before asking the learner to decode it",
      ),
      finding(
        language,
        "writing-ramp",
        ordered.length === 0 ? 0 : firstWritingPracticeAt === null ? ordered.length : firstWritingPracticeAt,
        "opening lesson(s)",
        firstWritingPracticeAt === null
          ? "add observable, guided writing practice from the opening lesson"
          : "move the first gentle writing microstep to lesson one",
      ),
      finding(
        language,
        "atom-step",
        (ramp?.lessonViolations ?? 0) + (ramp?.chapterViolations ?? 0),
        "lesson/chapter spike(s)",
        "split knowledge-atom spikes into smaller prerequisite-safe steps",
      ),
      finding(
        language,
        "glyph-step",
        (scriptRamp?.lessonViolations ?? 0) + (scriptRamp?.systemViolations ?? 0),
        "lesson/system spike(s)",
        "split new glyphs and writing systems across gentler steps",
      ),
      finding(
        language,
        "reinforcement",
        reinforcementWindowMisses,
        "missed window(s)",
        "add retrieval at the expanding R1-R4 intervals",
      ),
      finding(
        language,
        "payoff-surprise",
        payoff.get(language)?.length ?? 0,
        "chapter payoff finding(s)",
        "make each chapter payoff assess only taught, representative material",
      ),
      finding(
        language,
        "measurement-blind",
        ramp?.unmeasurable ?? ordered.length,
        "lesson(s)",
        "migrate undeclared lesson knowledge so gentleness can be measured",
      ),
    ].filter((entry): entry is GentleRampFinding => entry !== null);
    findings.sort(
      (a, b) => (priority.get(a.kind) ?? Number.MAX_SAFE_INTEGER) - (priority.get(b.kind) ?? Number.MAX_SAFE_INTEGER),
    );

    tracks.push({
      language,
      lessonCount: ordered.length,
      atomMeasurableLessons: ramp?.measurable ?? 0,
      atomMeasurementBlindLessons: ramp?.unmeasurable ?? ordered.length,
      durationViolations: durations.get(language)?.length ?? 0,
      atomLessonSpikes: ramp?.lessonViolations ?? 0,
      atomChapterSpikes: ramp?.chapterViolations ?? 0,
      glyphLessonSpikes: scriptRamp?.lessonViolations ?? 0,
      scriptSystemSpikes: scriptRamp?.systemViolations ?? 0,
      scriptClosureViolations: closure?.violations ?? 0,
      neverTaughtGlyphs: closure?.neverTaughtGlyphs ?? 0,
      orderDefects,
      unknownPrerequisites: unknownPrerequisites.get(language)?.length ?? 0,
      forwardPrerequisites,
      forwardReviews,
      forwardReferences: forward.get(language)?.length ?? 0,
      atomsTaught: continuity?.atomsTaught ?? 0,
      atomsNeverRevisited: continuity?.atomsNeverRevisited ?? 0,
      reinforcementWindowMisses,
      payoffSurprises: payoff.get(language)?.length ?? 0,
      writingPracticeLessons: writingPositions.length,
      firstWritingPracticeAt,
      lessonsBeforeWritingPractice: firstWritingPracticeAt ?? ordered.length,
      findings,
      next: findings[0] ?? null,
    });
  }

  const workQueue = tracks
    .flatMap((track) => track.findings)
    .sort(
      (a, b) =>
        (priority.get(a.kind) ?? Number.MAX_SAFE_INTEGER) - (priority.get(b.kind) ?? Number.MAX_SAFE_INTEGER) ||
        b.count - a.count ||
        a.language.localeCompare(b.language),
    );

  return {
    schemaVersion: 1,
    rule: {
      maxLessonSeconds: 300,
      durationBoundary: "strictly-greater-than",
      atomMeasurement: "declared-atoms-only",
      ranking: "learner-first-named-debt-no-composite-score",
    },
    tracks,
    workQueue,
    summary: {
      tracks: tracks.length,
      tracksWithDetectedCliffs: tracks.filter((track) => track.findings.length > 0).length,
      tracksWithNoWritingPractice: tracks.filter((track) => track.lessonCount > 0 && track.firstWritingPracticeAt === null).length,
      tracksWhereWritingStartsLate: tracks.filter((track) => (track.firstWritingPracticeAt ?? 0) > 0).length,
      atomMeasurementBlindLessons: tracks.reduce((sum, track) => sum + track.atomMeasurementBlindLessons, 0),
      findings: workQueue.length,
    },
  };
}

/** Compact corpus-wide queue: every row keeps its own unit. */
export function renderGentleRamp(report: GentleRampReport): string[] {
  const lines = [
    "Super-gentle ramp queue (HL17)",
    "==============================",
    `${report.summary.tracks} tracks; ${report.summary.tracksWithDetectedCliffs} with detected or unmeasured debt`,
    `${report.summary.tracksWithNoWritingPractice} track(s) never practise writing; ` +
      `${report.summary.tracksWhereWritingStartsLate} start after lesson one`,
    `${report.summary.atomMeasurementBlindLessons} lesson(s) remain atom-measurement blind`,
  ];
  for (const item of report.workQueue) {
    lines.push(`  ${item.kind.padEnd(20)} ${item.language.padEnd(14)} ${item.count} ${item.unit}: ${item.detail}`);
  }
  return lines;
}
