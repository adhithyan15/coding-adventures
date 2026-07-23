// ---------------------------------------------------------------------------
// quiz.ts — the covered grid, foundation of the randomised cumulative quiz (HL03 phase 4)
// ---------------------------------------------------------------------------
//
// The teaching sweep (sequence.ts) moves the learner FORWARD, one concept across
// the chain. Review pulls the other way: the primary review in HL03 is a
// randomised cumulative quiz that draws, unpredictably, from EVERYTHING learned
// so far — "what is 5 in Telugu? 12 in Latin? 3 in Arabic?" — mixing concepts
// and languages so retrieval is interleaved, which is what makes it stick.
//
// Before anything can be drawn, the pool has to be enumerated: every
// (concept, language) the learner has actually covered, each tied to the real
// word that answers it. That pool is the COVERED GRID, and building it is this
// file's job. The weighted draw over the grid (SRS-biased) is the next layer.
//
// The grid is built by REUSING the sweep, not by re-deriving the
// concept→language join: a cell exists exactly where the teaching sweep for a
// covered concept has a stop in an active language. So the review can only ever
// ask about a (concept, language) the teaching side actually presents — the two
// halves stay consistent by construction.
//
// Pure and deterministic: same inputs, same grid, same order.
// ---------------------------------------------------------------------------

import type { Lesson } from "./lessons";
import { activeChain, teachingSweep, type ChainLanguage } from "./sequence";

/**
 * One quizzable item: a concept, the language it is asked in, and the lesson
 * (the real word — its `headword`/`gloss`) that is the answer. A (concept,
 * language) with several lessons yields one cell per lesson.
 */
export interface GridCell {
  concept: string;
  language: ChainLanguage;
  lesson: Lesson;
}

/**
 * Every (concept, language) cell the learner has covered, as a flat list.
 *
 * `covered` is the set of concept tags studied so far; `activeCount` is how far
 * along the chain the learner has reached. For each covered concept we take its
 * teaching sweep over the active chain prefix and emit one cell per lesson at
 * each stop. The result is deterministic: concepts in sorted order, then chain
 * order (from the sweep), then the sweep's own chapter/id lesson order.
 */
export function coveredGrid(
  covered: Iterable<string>,
  lessons: Lesson[],
  activeCount: number,
): GridCell[] {
  const active = activeChain(activeCount);
  const concepts = [...new Set(covered)].filter((c) => c !== "").sort();

  const grid: GridCell[] = [];
  for (const concept of concepts) {
    for (const stop of teachingSweep(concept, lessons, active)) {
      for (const lesson of stop.lessons) {
        grid.push({ concept, language: stop.language, lesson });
      }
    }
  }
  return grid;
}

/** The distinct concepts present in a grid. */
export function conceptsIn(grid: GridCell[]): string[] {
  return [...new Set(grid.map((c) => c.concept))].sort();
}

/** The distinct languages present in a grid. */
export function languagesIn(grid: GridCell[]): ChainLanguage[] {
  return [...new Set(grid.map((c) => c.language))];
}

/** A stable key for a cell — used later by the SRS to track state per item. */
export function cellKey(cell: GridCell): string {
  // Encoded as JSON of the tuple rather than a delimiter-joined string: the repo
  // has been bitten by composite keys where a separator inside a field made two
  // distinct keys collide, and typed spaces here were mangled to NUL bytes.
  return JSON.stringify([cell.concept, cell.language, cell.lesson.id]);
}
