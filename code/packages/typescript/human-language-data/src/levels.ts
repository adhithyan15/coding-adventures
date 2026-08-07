// What level is this lesson building the reader toward?
//
// The project owner's ask: every lesson should be building toward a level, so that a
// "gentle introduction and ramp to A1" can be produced as a book. That is the same shape
// as the driving edition — ONE canonical corpus, filtered — and it is deliberately not a
// separate A1 corpus. `levelsUpTo("A1")` is the filter.
//
// NOTHING HERE IS AUTHORED, AND THAT IS THE POINT.
//
// A lesson's level is DERIVED from the shared spine: a lesson sits in a realization-path
// segment, the segment names a spine node, and the node declares a CEFR stage. HL08
// refused to write `modality:` into 1,134 frontmatter files because 1,134 authored copies
// of a computed fact are 1,134 places for it to go stale, and a level is the same kind of
// fact. Deriving it also means a track cannot claim A1 by editing frontmatter — it has to
// actually realize the A1 spine nodes.
//
// The honest gap: a lesson in no path segment has no derivable level. That is 170 of 1,134
// lessons today, all of them schema-v1 and unmapped. They are reported as `null`, never
// guessed at, because a wrong level is worse than a missing one — it would put material a
// reader is not ready for inside a book that promises a gentle ramp.

import type { CurriculumSpine, LanguageCurriculum } from "./types.js";
import type { ParsedLesson } from "./parse.js";

/**
 * The ladder, weakest first.
 *
 * `pre-A1` is NOT a CEFR level — CEFR begins at A1. It is this curriculum's own name for
 * the ramp below A1, the greetings-and-first-words stretch that an exam syllabus simply
 * assumes you already have. Naming it is what makes the ramp measurable instead of
 * invisible, and that ramp is the whole premise of the project.
 */
export const CEFR_LEVELS = ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"] as const;

export type CefrLevel = (typeof CEFR_LEVELS)[number];

/** Position on the ladder, for comparisons. Higher is further along. */
export function levelRank(level: CefrLevel): number {
  return CEFR_LEVELS.indexOf(level);
}

/** Every level at or below `ceiling` — the filter a "ramp to X" book applies. */
export function levelsUpTo(ceiling: CefrLevel): CefrLevel[] {
  return CEFR_LEVELS.slice(0, levelRank(ceiling) + 1);
}

/** One lesson's answer, with the reason it has that answer. */
export interface LessonLevel {
  lessonId: string;
  language: string;
  /** Null when the lesson is in no realization-path segment, so nothing can be derived. */
  level: CefrLevel | null;
  /** The spine node the level came from, or null when unmapped. */
  spineNode: string | null;
  reason: "spine-node" | "unmapped";
}

export interface LevelSummary {
  totalLessons: number;
  /** Lessons per level, in ladder order. Absent levels are present with 0. */
  byLevel: Record<CefrLevel, number>;
  /** Lessons no spine node claims, so no level could be derived. */
  unmapped: number;
  /** Share of the corpus whose level is known at all. */
  mappedPercent: number;
  tracks: TrackLevelCoverage[];
}

export interface TrackLevelCoverage {
  language: string;
  lessonCount: number;
  byLevel: Record<CefrLevel, number>;
  unmapped: number;
  /**
   * The highest level this track has any lesson at.
   *
   * This is the "how far is this track from Advanced" number, and today it is `A1` or
   * `pre-A1` for every track in the corpus — nothing has reached A2.
   */
  reach: CefrLevel | null;
}

function emptyHistogram(): Record<CefrLevel, number> {
  return Object.fromEntries(CEFR_LEVELS.map((level) => [level, 0])) as Record<
    CefrLevel,
    number
  >;
}

/**
 * Map every lesson id to the spine node whose path segment contains it.
 *
 * Built as a `Map`, not an object: the keys are lesson ids read out of committed JSON,
 * and `index["__proto__"] = x` on a plain object writes the prototype instead of a
 * property. The same reasoning is recorded on `modalityManifestById`.
 */
export function lessonSpineNodes(curricula: LanguageCurriculum[]): Map<string, string> {
  const nodes = new Map<string, string>();
  for (const curriculum of curricula) {
    for (const segment of curriculum.path) {
      for (const lessonId of segment.lessons) {
        // First writer wins. A lesson in two segments is a ledger bug that
        // `validateCurriculum` already reports; silently taking the last one here would
        // make this module's answer depend on file order.
        if (!nodes.has(lessonId)) nodes.set(lessonId, segment.spine_node);
      }
    }
  }
  return nodes;
}

/** Derive one lesson's level. */
export function deriveLessonLevel(
  lesson: ParsedLesson,
  spineNodes: Map<string, string>,
  stageOf: Map<string, CefrLevel>,
): LessonLevel {
  const lessonId = lesson.realization.lessonId;
  const node = spineNodes.get(lessonId);
  const level = node ? (stageOf.get(node) ?? null) : null;
  return {
    lessonId,
    language: lesson.language,
    level,
    spineNode: node ?? null,
    reason: level ? "spine-node" : "unmapped",
  };
}

/** Derive every lesson's level and roll it up per track and corpus-wide. */
export function summarizeLevels(
  lessons: ParsedLesson[],
  curricula: LanguageCurriculum[],
  spine: CurriculumSpine,
): LevelSummary {
  const spineNodes = lessonSpineNodes(curricula);
  const stageOf = new Map<string, CefrLevel>(
    spine.nodes.map((node) => [node.id, node.stage as CefrLevel]),
  );

  const byLevel = emptyHistogram();
  let unmapped = 0;
  const perTrack = new Map<string, TrackLevelCoverage>();

  for (const lesson of lessons) {
    const entry = deriveLessonLevel(lesson, spineNodes, stageOf);
    let track = perTrack.get(entry.language);
    if (!track) {
      track = {
        language: entry.language,
        lessonCount: 0,
        byLevel: emptyHistogram(),
        unmapped: 0,
        reach: null,
      };
      perTrack.set(entry.language, track);
    }
    track.lessonCount += 1;
    if (entry.level) {
      byLevel[entry.level] += 1;
      track.byLevel[entry.level] += 1;
      if (!track.reach || levelRank(entry.level) > levelRank(track.reach)) {
        track.reach = entry.level;
      }
    } else {
      unmapped += 1;
      track.unmapped += 1;
    }
  }

  const mapped = lessons.length - unmapped;
  return {
    totalLessons: lessons.length,
    byLevel,
    unmapped,
    mappedPercent: lessons.length === 0 ? 0 : Math.round((mapped / lessons.length) * 100),
    tracks: [...perTrack.values()].sort((a, b) => a.language.localeCompare(b.language)),
  };
}

/**
 * The lessons a "ramp to `ceiling`" edition would contain, in corpus order.
 *
 * Unmapped lessons are EXCLUDED rather than included-by-default. A book that promises a
 * gentle ramp to A1 must not quietly carry material nobody has placed on the ladder; the
 * honest failure is a shorter book, not a book with a surprise in it.
 */
export function lessonsUpToLevel(
  lessons: ParsedLesson[],
  curricula: LanguageCurriculum[],
  spine: CurriculumSpine,
  ceiling: CefrLevel,
): ParsedLesson[] {
  const spineNodes = lessonSpineNodes(curricula);
  const stageOf = new Map<string, CefrLevel>(
    spine.nodes.map((node) => [node.id, node.stage as CefrLevel]),
  );
  const allowed = new Set(levelsUpTo(ceiling));
  return lessons.filter((lesson) => {
    const entry = deriveLessonLevel(lesson, spineNodes, stageOf);
    return entry.level !== null && allowed.has(entry.level);
  });
}
