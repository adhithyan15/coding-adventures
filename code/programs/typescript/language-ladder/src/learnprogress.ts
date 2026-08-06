// Per-language Learn progression and focused-before-mixed eligibility.
//
// Progress is stored by stable lesson id, not by array index. On load, each
// language is reduced to the longest completed prefix of its validated local
// path. If the curriculum inserts a new prerequisite, that new lesson becomes
// the frontier instead of allowing stale saved data to jump over it.

import {
  nextCurriculumLesson,
  orderedCurriculumLessonIds,
} from "@coding-adventures/human-language-data/src/plans.ts";
import type { LanguageCurriculum } from "@coding-adventures/human-language-data/src/types.ts";
import type { Lesson } from "./lessons.ts";
import type { GridCell } from "./quiz.ts";
import { type StorageLike, browserStorage } from "./progress.ts";

export { browserStorage };
export type { StorageLike };

export const LEARN_PROGRESS_SCHEMA_VERSION = 1;
export const LEARN_PROGRESS_STORAGE_KEY = "language-ladder:learn-progress:v1";

export type LearnCompletion = Map<string, Set<string>>;

export interface SavedLearnProgress {
  version: number;
  completed: Array<[string, string[]]>;
}

/** Keep only the longest completed prefix of one validated local path. */
export function completedPrefix(
  curriculum: LanguageCurriculum,
  candidate: ReadonlySet<string>,
): Set<string> {
  const prefix = new Set<string>();
  for (const lessonId of orderedCurriculumLessonIds(curriculum)) {
    if (!candidate.has(lessonId)) break;
    prefix.add(lessonId);
  }
  return prefix;
}

/** Normalize untrusted completion data against the currently bundled maps. */
export function normalizeLearnCompletion(
  candidates: ReadonlyMap<string, ReadonlySet<string>>,
  curricula: readonly LanguageCurriculum[],
): LearnCompletion {
  const normalized: LearnCompletion = new Map();
  for (const curriculum of curricula) {
    const prefix = completedPrefix(
      curriculum,
      candidates.get(curriculum.language) ?? new Set<string>(),
    );
    if (prefix.size > 0) normalized.set(curriculum.language, prefix);
  }
  return normalized;
}

/** Convert the persisted transport shape into safe in-memory sets. */
export function fromSavedLearnProgress(
  saved: SavedLearnProgress,
  curricula: readonly LanguageCurriculum[],
): LearnCompletion {
  if (saved.version !== LEARN_PROGRESS_SCHEMA_VERSION || !Array.isArray(saved.completed)) {
    return new Map();
  }
  const candidates = new Map<string, Set<string>>();
  for (const entry of saved.completed) {
    if (!Array.isArray(entry) || entry.length !== 2) continue;
    const [language, ids] = entry;
    if (typeof language !== "string" || !Array.isArray(ids)) continue;
    const strings = ids.filter((id): id is string => typeof id === "string");
    candidates.set(language, new Set(strings));
  }
  return normalizeLearnCompletion(candidates, curricula);
}

/** Serialize only safe prefixes, in curriculum registry order. */
export function toSavedLearnProgress(
  completion: ReadonlyMap<string, ReadonlySet<string>>,
  curricula: readonly LanguageCurriculum[],
): SavedLearnProgress {
  const normalized = normalizeLearnCompletion(completion, curricula);
  const completed: Array<[string, string[]]> = [];
  for (const curriculum of curricula) {
    const prefix = normalized.get(curriculum.language);
    if (prefix && prefix.size > 0) completed.push([curriculum.language, [...prefix]]);
  }
  return { version: LEARN_PROGRESS_SCHEMA_VERSION, completed };
}

/** Load fail-closed: malformed, unavailable, or wrong-version storage is empty. */
export function loadLearnProgress(
  storage: StorageLike | null,
  curricula: readonly LanguageCurriculum[],
): LearnCompletion {
  if (!storage) return new Map();
  try {
    const raw = storage.getItem(LEARN_PROGRESS_STORAGE_KEY);
    if (!raw) return new Map();
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return new Map();
    return fromSavedLearnProgress(parsed as SavedLearnProgress, curricula);
  } catch {
    return new Map();
  }
}

/** Persist safe prefixes immediately; storage failure never blocks learning. */
export function saveLearnProgress(
  storage: StorageLike | null,
  completion: ReadonlyMap<string, ReadonlySet<string>>,
  curricula: readonly LanguageCurriculum[],
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(
      LEARN_PROGRESS_STORAGE_KEY,
      JSON.stringify(toSavedLearnProgress(completion, curricula)),
    );
    return true;
  } catch {
    return false;
  }
}

export interface CompletionResult {
  completion: LearnCompletion;
  changed: boolean;
}

/** Complete exactly the current frontier lesson; reject skips and replays. */
export function completeFrontierLesson(
  completion: ReadonlyMap<string, ReadonlySet<string>>,
  curriculum: LanguageCurriculum,
  lessonId: string,
): CompletionResult {
  const safe = completedPrefix(
    curriculum,
    completion.get(curriculum.language) ?? new Set<string>(),
  );
  if (nextCurriculumLesson(curriculum, safe)?.lessonId !== lessonId) {
    const unchanged: LearnCompletion = new Map();
    for (const [language, ids] of completion) unchanged.set(language, new Set(ids));
    unchanged.set(curriculum.language, safe);
    return { completion: unchanged, changed: false };
  }
  safe.add(lessonId);
  const next: LearnCompletion = new Map();
  for (const [language, ids] of completion) next.set(language, new Set(ids));
  next.set(curriculum.language, safe);
  return { completion: next, changed: true };
}

/** Completed and total local lesson counts for a progress read-out. */
export function localPathProgress(
  curriculum: LanguageCurriculum,
  completion: ReadonlyMap<string, ReadonlySet<string>>,
): { completed: number; total: number } {
  const total = orderedCurriculumLessonIds(curriculum).length;
  const completed = completedPrefix(
    curriculum,
    completion.get(curriculum.language) ?? new Set<string>(),
  ).size;
  return { completed, total };
}

/**
 * The mixed review pool contains only lessons that independently passed their
 * focused check in their own language. A later saved id cannot leak through a
 * missing prerequisite because completion is reduced to a local prefix first.
 */
export function eligibleReviewGrid(
  lessons: readonly Lesson[],
  curricula: readonly LanguageCurriculum[],
  selectedLanguages: readonly string[],
  completion: ReadonlyMap<string, ReadonlySet<string>>,
): GridCell[] {
  const curriculumByLanguage = new Map(
    curricula.map((curriculum) => [curriculum.language, curriculum]),
  );
  const lessonById = new Map(lessons.map((lesson) => [lesson.id, lesson]));
  const grid: GridCell[] = [];
  for (const language of selectedLanguages) {
    const curriculum = curriculumByLanguage.get(language);
    if (!curriculum) continue;
    const prefix = completedPrefix(
      curriculum,
      completion.get(language) ?? new Set<string>(),
    );
    for (const lessonId of orderedCurriculumLessonIds(curriculum)) {
      if (!prefix.has(lessonId)) break;
      const lesson = lessonById.get(lessonId);
      if (!lesson || lesson.concept === "") continue;
      grid.push({ concept: lesson.concept, language, lesson });
    }
  }
  return grid;
}

/** A mixed question needs at least two visually distinct eligible answers. */
export function mixedReviewReady(grid: readonly GridCell[]): boolean {
  return new Set(grid.map((cell) => cell.lesson.headword)).size >= 2;
}
