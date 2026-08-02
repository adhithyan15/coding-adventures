// ---------------------------------------------------------------------------
// concepts.ts — cross-language study, and the gate that keeps it teachable.
//
// The curriculum tags almost every lesson with a `concept_tag`. Canonical tags
// (GREETING-HELLO, TIME-DAY) are shared across tracks *on purpose*: they are the
// join key that lets you ask "how does every language I'm learning say this?"
// The data package has shipped `languagesForConcept` for exactly that since the
// beginning, documented as "what the companion app calls" — with no caller.
// This module is the caller.
//
// It does two jobs:
//
//   1. CROSS-LANGUAGE GROUPS. Build a real `Dataset` and pull out the concepts
//      that more than one track realizes. Those are the study cards: one idea,
//      several languages, side by side.
//
//   2. PREREQUISITE GATING. Lessons declare `prerequisites`. Serving a learner
//      "the preterite of *comer*" before they have met *comer* is worse than
//      useless, and the scheduler — which is deliberately generic over a numeric
//      index — has no way to know that. So gating happens here, before the
//      scheduler ever sees the pool.
//
// Everything here is pure: data in, data out, no DOM and no storage. The impure
// edges stay in `main.ts`.
// ---------------------------------------------------------------------------

// Deep imports, for the reason spelled out at the top of `lessons.ts`: the
// package barrel drags `loader.ts` (node:fs) and `cli.ts` (process) into the
// browser bundle, and the build *succeeds* before the app dies on load. Both
// modules reached for here are pure — `parse.ts` does frontmatter → dataset,
// `queries.ts` imports nothing but types.
import {
  buildDataset,
  type ParsedLesson,
} from "@coding-adventures/human-language-data/src/parse.ts";
import { languagesForConcept } from "@coding-adventures/human-language-data/src/queries.ts";
import type {
  Dataset,
  Realization,
  Script,
  Taxonomy,
} from "@coding-adventures/human-language-data/src/types.ts";
import type { Lesson } from "./lessons.js";

/**
 * One study card: a concept, and every language's way of saying it.
 *
 * `realizations` is whatever `languagesForConcept` returned, so the shape is
 * the package's, not ours — headword, gloss, romanization and etymology hook
 * all come along for free.
 */
export interface ConceptCard {
  id: string;
  /** Human-readable gloss from the taxonomy, or the first realization's. */
  gloss: string;
  /** True when the tag is language-specific (ES-…) rather than canonical. */
  namespaced: boolean;
  realizations: Realization[];
}

/**
 * Concepts realized by at least `minLanguages` tracks, id-sorted.
 *
 * The floor matters. A concept only one track has ever tagged is a perfectly
 * good lesson but a *useless* cross-language card — there is nothing to compare
 * it with. Namespaced tags (ES-SUBJUNCTIVE-PRESENT) are almost always in that
 * position by construction, so this filter removes most of them without needing
 * a special case for them.
 */
export function crossLanguageConcepts(
  dataset: Dataset,
  minLanguages = 2,
): ConceptCard[] {
  const out: ConceptCard[] = [];
  for (const concept of dataset.concepts) {
    const realizations = languagesForConcept(dataset, concept.id);
    const languages = new Set(realizations.map((r) => r.language));
    if (languages.size < minLanguages) continue;
    out.push({
      id: concept.id,
      gloss: concept.gloss,
      namespaced: concept.namespaced,
      realizations,
    });
  }
  return out.sort((a, b) => a.id.localeCompare(b.id));
}

/**
 * Build the dataset the cross-language mode reads.
 *
 * `buildDataset` wants `ParsedLesson[]`, but this app has already mapped those
 * down to its own leaner `Lesson`. Re-parsing would mean holding every lesson
 * twice, so instead we hand `buildDataset` the realizations it actually reads.
 * Keeping that adaptation in one named function means the shape mismatch is
 * documented rather than scattered.
 */
export function datasetFromLessons(
  taxonomy: Taxonomy,
  lessons: Lesson[],
): Dataset {
  const parsed: ParsedLesson[] = lessons.map((l) => ({
    language: l.language,
    script: l.script as Script,
    // `buildDataset` reads only `.realization`; frontmatter is inert here, and
    // an empty object is honest about that rather than reconstructing a fake.
    frontmatter: {},
    body: l.body,
    realization: {
      concept: l.concept,
      language: l.language,
      lessonId: l.id,
      chapter: l.chapter,
      type: l.type,
      headword: l.headword,
      gloss: l.gloss,
      romanization: l.romanization,
      script: l.script as Script,
      // The app's leaner Lesson doesn't carry gender, and `buildDataset`
      // doesn't read it. `null` is the type's own "unknown", so this is
      // truthful rather than a placeholder.
      gender: null,
      sounds: [],
      roots: [],
      etymologyHook: l.etymologyHook,
    },
  }));
  return buildDataset(taxonomy, parsed);
}

// ---------------------------------------------------------------------------
// Prerequisite gating
// ---------------------------------------------------------------------------

/**
 * Is this lesson teachable yet?
 *
 * A lesson unlocks when every id in its `prerequisites` has been seen. Unknown
 * ids — a typo, or a prerequisite pointing at a lesson not yet written — fail
 * closed. The curriculum validator must surface the data bug; the app must not
 * teach material whose declared foundation it cannot prove was learned.
 */
export function isUnlocked(
  lesson: Lesson,
  seen: ReadonlySet<string>,
  known: ReadonlySet<string>,
): boolean {
  return lesson.prerequisites.every((id) => known.has(id) && seen.has(id));
}

/**
 * The indices of every lesson that is teachable right now.
 *
 * Returned as indices because that is the currency the scheduler speaks.
 *
 * NEVER-EMPTY GUARANTEE: on a fresh profile nothing has been seen, so any
 * lesson with prerequisites is locked — but chapter-1 lessons have none, so the
 * pool opens with those and widens as they are learned. If a curriculum change
 * ever did lock everything, the caller gets an empty array and must fall back;
 * `unlockedOrAll` below is that fallback, made explicit.
 */
export function unlockedIndices(
  lessons: Lesson[],
  seen: ReadonlySet<string>,
): number[] {
  const known = new Set(lessons.map((l) => l.id));
  const out: number[] = [];
  for (let i = 0; i < lessons.length; i += 1) {
    if (isUnlocked(lessons[i], seen, known)) out.push(i);
  }
  return out;
}

/**
 * Compatibility name retained for callers. It now fails closed exactly like
 * `unlockedIndices`; an empty result is a visible curriculum problem, never
 * permission to show material early.
 */
export function unlockedOrAll(
  lessons: Lesson[],
  seen: ReadonlySet<string>,
): number[] {
  return unlockedIndices(lessons, seen);
}

/**
 * Ids a lesson explicitly revisits, restricted to ones that exist.
 *
 * `reviews_of` is the curriculum's own statement of "answering this should
 * refresh those." Surfacing it lets the app bring the named lessons forward
 * instead of waiting for their Leitner interval — the difference between a
 * scheduler that merely repeats and one that follows the syllabus.
 */
export function reviewTargets(
  lesson: Lesson,
  byId: ReadonlyMap<string, number>,
): number[] {
  const out: number[] = [];
  for (const id of lesson.reviewsOf) {
    const index = byId.get(id);
    if (index !== undefined) out.push(index);
  }
  return out;
}

/** id → index, for `reviewTargets`. */
export function indexById(lessons: Lesson[]): Map<string, number> {
  const map = new Map<string, number>();
  lessons.forEach((l, i) => map.set(l.id, i));
  return map;
}
