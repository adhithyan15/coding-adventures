// Is the ramp actually gentle? — HL08's budgets, finally measured.
//
// `core/chapter-policy.json` has declared `maxNewAtomsPerLesson: 3` and
// `maxNewAtomsPerChapter: 12` since HL08, with a rationale saying they sit at the corpus's
// own p90 so "only genuine spikes are flagged". **Nothing read either number.** They were
// policy in the sense that a sign is policy: written down, and enforced by nobody.
//
// That is not a small gap. "A very gentle ramp" is the project's founding promise, and
// HL-C18 exists to burn down the lessons that break it — but the figure everyone quoted
// ("52 over-budget lessons") came from an ad-hoc count that no test reproduces, and a fresh
// count returns something different because most of the corpus is schema-v1 and carries no
// machine-readable atoms at all. You cannot burn down a list you cannot recompute.
//
// This module recomputes it. Report-only, like the HL05 chapter gates: the debt predates
// the measurement, so it is measured and made visible rather than turned into a build
// failure on a corpus nobody regressed.
//
// THE HONEST LIMIT, stated because it changes how the number should be read: a lesson only
// counts atoms it declares. Schema-v1 lessons declare none, so they read as 0 and are
// reported separately as `unmeasurable` rather than silently counted as compliant. A track
// with a low violation count and a high unmeasurable count has not proved it is gentle; it
// has proved it is unmigrated.

import type { ChapterPolicy } from "./types.js";
import type { ParsedLesson } from "./parse.js";

/** One lesson that introduces more than the budget allows. */
export interface RampViolation {
  lessonId: string;
  language: string;
  chapter: number | null;
  /** Atoms this lesson introduces. */
  atoms: number;
  /** The budget it exceeded. */
  budget: number;
}

/** One chapter that introduces more than the chapter budget allows. */
export interface ChapterRampViolation {
  language: string;
  chapter: number;
  atoms: number;
  budget: number;
  /** Lessons in the chapter, so a splitter knows what it is working with. */
  lessonCount: number;
}

export interface TrackRampCoverage {
  language: string;
  lessonCount: number;
  /** Lessons declaring at least one atom — the ones this can actually judge. */
  measurable: number;
  /** Lessons declaring none, almost always schema-v1. NOT evidence of gentleness. */
  unmeasurable: number;
  lessonViolations: number;
  chapterViolations: number;
}

export interface RampReport {
  policy: { maxNewAtomsPerLesson: number; maxNewAtomsPerChapter: number };
  lessons: RampViolation[];
  chapters: ChapterRampViolation[];
  tracks: TrackRampCoverage[];
  summary: {
    /** Lessons above `maxNewAtomsPerLesson`. The HL-C18 burn-down list. */
    lessonViolations: number;
    /** Chapters above `maxNewAtomsPerChapter`, so splitting cannot game the lesson rule. */
    chapterViolations: number;
    /** Lessons declaring no atoms at all — the measurement's blind spot, named. */
    unmeasurableLessons: number;
    /** Share of the corpus this measurement can actually see. */
    measurablePercent: number;
    /** The steepest single lesson, which is where a burn-down starts. */
    steepestLesson: RampViolation | null;
  };
}

function frontmatterList(lesson: ParsedLesson, key: string): string[] {
  const value = lesson.frontmatter[key];
  if (Array.isArray(value)) return value.filter((item): item is string => typeof item === "string");
  return typeof value === "string" && value.trim() ? [value.trim()] : [];
}

/**
 * Atoms a lesson introduces — frontmatter and block directives unioned.
 *
 * The frontmatter key is FLAT and dotted (`introduces.knowledge`); reading it as a nested
 * object returns undefined for every lesson in the corpus. That mistake once made the
 * chapter gates report all 279 authored chapters as broken, so it is worth restating
 * wherever atoms are counted.
 */
function introducedAtoms(lesson: ParsedLesson): string[] {
  const atoms = new Set(frontmatterList(lesson, "introduces.knowledge"));
  for (const block of lesson.blocks ?? []) {
    for (const atom of block.knowledge?.introduces ?? []) atoms.add(atom);
  }
  return [...atoms];
}

/** Measure the gentle-ramp budgets across the corpus. */
export function measureRamp(lessons: ParsedLesson[], policy: ChapterPolicy): RampReport {
  const perLesson = policy.maxNewAtomsPerLesson;
  const perChapter = policy.maxNewAtomsPerChapter;

  const violations: RampViolation[] = [];
  const chapterAtoms = new Map<string, Set<string>>();
  const chapterLessons = new Map<string, number>();
  const tracks = new Map<string, TrackRampCoverage>();

  for (const lesson of lessons) {
    const language = lesson.language;
    const chapter = typeof lesson.realization?.chapter === "number" ? lesson.realization.chapter : null;
    const atoms = introducedAtoms(lesson);

    let track = tracks.get(language);
    if (!track) {
      track = {
        language,
        lessonCount: 0,
        measurable: 0,
        unmeasurable: 0,
        lessonViolations: 0,
        chapterViolations: 0,
      };
      tracks.set(language, track);
    }
    track.lessonCount += 1;
    if (atoms.length === 0) track.unmeasurable += 1;
    else track.measurable += 1;

    if (atoms.length > perLesson) {
      violations.push({
        lessonId: lesson.realization.lessonId,
        language,
        chapter,
        atoms: atoms.length,
        budget: perLesson,
      });
      track.lessonViolations += 1;
    }

    if (chapter !== null) {
      const key = `${language}:${chapter}`;
      let set = chapterAtoms.get(key);
      if (!set) chapterAtoms.set(key, (set = new Set()));
      for (const atom of atoms) set.add(atom);
      chapterLessons.set(key, (chapterLessons.get(key) ?? 0) + 1);
    }
  }

  const chapters: ChapterRampViolation[] = [];
  for (const [key, atoms] of chapterAtoms) {
    if (atoms.size <= perChapter) continue;
    const [language, chapterText] = key.split(":");
    chapters.push({
      language: language!,
      chapter: Number(chapterText),
      atoms: atoms.size,
      budget: perChapter,
      lessonCount: chapterLessons.get(key) ?? 0,
    });
    const track = tracks.get(language!);
    if (track) track.chapterViolations += 1;
  }

  // Steepest first, then by id, so the list is a stable work queue rather than a set.
  violations.sort((a, b) => b.atoms - a.atoms || a.lessonId.localeCompare(b.lessonId));
  chapters.sort(
    (a, b) => b.atoms - a.atoms || a.language.localeCompare(b.language) || a.chapter - b.chapter,
  );

  const unmeasurable = [...tracks.values()].reduce((sum, track) => sum + track.unmeasurable, 0);
  return {
    policy: { maxNewAtomsPerLesson: perLesson, maxNewAtomsPerChapter: perChapter },
    lessons: violations,
    chapters,
    tracks: [...tracks.values()].sort((a, b) => a.language.localeCompare(b.language)),
    summary: {
      lessonViolations: violations.length,
      chapterViolations: chapters.length,
      unmeasurableLessons: unmeasurable,
      measurablePercent:
        lessons.length === 0 ? 0 : Math.round(((lessons.length - unmeasurable) / lessons.length) * 100),
      steepestLesson: violations[0] ?? null,
    },
  };
}
