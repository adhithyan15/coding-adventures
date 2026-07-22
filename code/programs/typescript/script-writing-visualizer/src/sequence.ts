// ---------------------------------------------------------------------------
// sequence.ts — the language CHAIN and the per-concept TEACHING SWEEP (HL03)
// ---------------------------------------------------------------------------
//
// The unified app (HL03) teaches along a fixed CHAIN of languages, each hop
// chosen because a bridge already exists to a language learned earlier:
//
//   Spanish → Latin (the roots beneath Spanish) → French → German (the English
//   twins) → Arabic → Hindi (where Arabic, Persian and Sanskrit meet) → Tamil
//   (first Dravidian) → Kannada (Tamil's cousin, + Sanskrit) → Telugu → Malayalam
//
// Learning moves CONCEPT BY CONCEPT. To learn a concept is to walk it across
// every language the learner has added so far — the "active" prefix of the
// chain — IN CHAIN ORDER, so the cognates, shared roots and false friends line
// up. That walk is the TEACHING SWEEP, and it is the one operation this module
// exists to compute.
//
// This module is the *sequencing* layer only: pure, deterministic, no UI, no
// review. Review (the randomised cumulative quiz) and the connections a sweep
// surfaces are later layers that build on this one. Everything here is a pure
// function of its inputs so it can be tested exactly, against fixtures and
// against the real curriculum.
// ---------------------------------------------------------------------------

import type { Lesson } from "./lessons";

/**
 * The chain, in the order languages are added. These are track directory names
 * under `code/learning/human-languages/`. The order is not arbitrary — it is a
 * connected path where each language reaches back to one already known.
 */
export const LANGUAGE_CHAIN = [
  "spanish",
  "latin",
  "french",
  "german",
  "arabic",
  "hindi",
  "tamil",
  "kannada",
  "telugu",
  "malayalam",
] as const;

export type ChainLanguage = (typeof LANGUAGE_CHAIN)[number];

/** Position of a language in the chain, or -1 if it is not on the chain. */
export function chainIndex(language: string): number {
  return (LANGUAGE_CHAIN as readonly string[]).indexOf(language);
}

export function isChainLanguage(language: string): language is ChainLanguage {
  return chainIndex(language) !== -1;
}

/**
 * The ACTIVE prefix: the first `count` languages of the chain — the ones the
 * learner has added so far. `count` is clamped to `[0, chain length]`, so
 * `activeChain(0)` is empty and any over-count is the whole chain. Adding a
 * language is `activeChain(count + 1)`.
 */
export function activeChain(count: number): ChainLanguage[] {
  const n = Math.max(0, Math.min(Math.floor(count), LANGUAGE_CHAIN.length));
  return LANGUAGE_CHAIN.slice(0, n);
}

/** One stop on a sweep: a language, and the lesson(s) there that teach the concept. */
export interface SweepStop {
  language: ChainLanguage;
  lessons: Lesson[];
}

/**
 * The teaching sweep for one concept: every ACTIVE language that teaches the
 * concept, IN CHAIN ORDER, each with its lesson(s).
 *
 * Three filters, in this order of intent:
 *   1. the concept — only lessons whose `concept` tag matches;
 *   2. active — only languages in the given active prefix;
 *   3. teaches-it — a language with no matching lesson simply does not appear
 *      (the sweep is the languages that *can* show the concept, not the whole
 *      active set).
 *
 * The result order is the CHAIN order, never the input order — the walk is
 * Spanish-first regardless of how the lessons were listed. Within a language,
 * lessons are ordered by chapter then id, so the sweep is fully deterministic.
 *
 * `active` is expected to be a CHAIN PREFIX (as `activeChain` returns) — ordered,
 * unique, and all on the chain. Chain order and active-filtering both come from
 * iterating it, so a hand-built non-prefix (out of order, or with duplicates)
 * would be reflected verbatim in the output; build `active` via `activeChain`.
 */
export function teachingSweep(
  concept: string,
  lessons: Lesson[],
  active: readonly ChainLanguage[],
): SweepStop[] {
  if (concept === "") return []; // "" means "no concept" (e.g. writing lessons)

  // Bucket every matching lesson by language. Active-filtering is NOT done here
  // on purpose: the output loop below walks `active` (a chain prefix), which
  // enforces both the active set AND chain order in one place — a single source
  // of truth, so the test that a language outside the active set is excluded can
  // actually fail if that loop is broken.
  const byLanguage = new Map<ChainLanguage, Lesson[]>();
  for (const lesson of lessons) {
    if (lesson.concept !== concept) continue;
    if (!isChainLanguage(lesson.language)) continue;
    const bucket = byLanguage.get(lesson.language) ?? [];
    bucket.push(lesson);
    byLanguage.set(lesson.language, bucket);
  }

  const stops: SweepStop[] = [];
  for (const language of active) {
    const found = byLanguage.get(language);
    if (!found || found.length === 0) continue;
    found.sort((a, b) => a.chapter - b.chapter || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
    stops.push({ language, lessons: found });
  }
  return stops;
}

/**
 * The concepts that can be swept given an active prefix — every concept taught
 * by at least one active language — in BOOK ORDER.
 *
 * "Book order" here is the order concepts first come up as you read forward: a
 * concept's key is the *earliest chapter* at which any active language teaches
 * it, tie-broken by the earliest chain position that teaches it, then the tag
 * itself for stability. Writing lessons (concept "") contribute nothing.
 */
export function sweepableConcepts(
  lessons: Lesson[],
  active: readonly ChainLanguage[],
): string[] {
  const activeSet = new Set<string>(active);
  const minChapter = new Map<string, number>();
  const minChainPos = new Map<string, number>();

  for (const lesson of lessons) {
    if (lesson.concept === "") continue;
    if (!activeSet.has(lesson.language)) continue;
    const c = lesson.concept;
    const prevCh = minChapter.get(c);
    if (prevCh === undefined || lesson.chapter < prevCh) minChapter.set(c, lesson.chapter);
    const pos = chainIndex(lesson.language);
    const prevPos = minChainPos.get(c);
    if (prevPos === undefined || pos < prevPos) minChainPos.set(c, pos);
  }

  return [...minChapter.keys()].sort((a, b) => {
    const ca = minChapter.get(a)!;
    const cb = minChapter.get(b)!;
    if (ca !== cb) return ca - cb;
    const pa = minChainPos.get(a)!;
    const pb = minChainPos.get(b)!;
    if (pa !== pb) return pa - pb;
    return a < b ? -1 : a > b ? 1 : 0;
  });
}
