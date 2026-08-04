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
import { resolveActiveLanguages, teachingSweep, type ChainLanguage, type LanguageSelection } from "./sequence";

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
  activeSelection: LanguageSelection,
): GridCell[] {
  const active = resolveActiveLanguages(activeSelection);
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

// ---------------------------------------------------------------------------
// Part 2 — the SRS-weighted draw.
//
// The quiz is a randomised draw over the covered grid, but not a UNIFORM one:
// it leans on what the learner is weakest at. Each cell keeps a little Leitner
// state (the same box/interval math as scheduler.ts, keyed by cellKey instead
// of a letter index), and the draw weights a cell by how much it is owed:
//   • never seen  → high (introduce it),
//   • DUE, and the more overdue / the lower its box / the more it has lapsed →
//     highest (this is the missed material the review exists for),
//   • not yet due → low (it can still show up, so review stays interleaved).
//
// The draw is deterministic given a seeded PRNG, so the whole thing is testable.
// ---------------------------------------------------------------------------

import { MAX_BOX } from "./scheduler";

/** Per-cell review state — Leitner box + when it next comes due. */
export interface QuizState {
  box: number;
  dueAtSession: number;
  lapses: number;
  reps: number;
}

/** Is a cell due to be reviewed at (or before) this session? */
export function cellDue(state: QuizState, session: number): boolean {
  return state.dueAtSession <= session;
}

/**
 * The draw weight for a cell given its state (or undefined = never seen).
 * Higher = more likely to be drawn. A mastered, not-yet-due cell sinks to the
 * floor; an overdue, low-box, lapsed cell rises far above it.
 */
export function cellWeight(state: QuizState | undefined, session: number): number {
  if (state === undefined) return 6; // new material — worth asking
  if (!cellDue(state, session)) return 1; // not due — rare interleaving only
  const overdue = Math.max(0, session - state.dueAtSession);
  const box = Math.max(0, Math.min(MAX_BOX, state.box));
  return 4 + overdue + (MAX_BOX - box) + state.lapses;
}

/**
 * Draw the next quiz cell from the grid, weighted by SRS state, using a seeded
 * PRNG (`rng()` in [0, 1)). Returns null for an empty grid. Deterministic:
 * same grid + states + session + rng sequence → same cell.
 */
export function pickNext(
  grid: GridCell[],
  states: Map<string, QuizState>,
  session: number,
  rng: () => number,
): GridCell | null {
  if (grid.length === 0) return null;
  const weights = grid.map((cell) => cellWeight(states.get(cellKey(cell)), session));
  const total = weights.reduce((a, w) => a + w, 0);
  if (total <= 0) return grid[0] ?? null;
  let point = rng() * total;
  for (let i = 0; i < grid.length; i++) {
    point -= weights[i]!;
    if (point < 0) return grid[i]!;
  }
  return grid[grid.length - 1]!; // float slop — return the last cell
}

/**
 * A small deterministic PRNG (a linear congruential generator), matching the
 * app's practice of never depending on Math.random. `makeRng(seed)()` yields a
 * reproducible stream in [0, 1).
 */
export function makeRng(seed: number): () => number {
  let s = (seed | 0) || 1;
  return () => {
    // Math.imul keeps the 32-bit multiply exact (a plain * overflows 2^53 and
    // rounds away low bits, shrinking the LCG period to ~11k); this restores it.
    s = (Math.imul(s, 1103515245) + 12345) & 0x7fffffff;
    return s / 0x7fffffff;
  };
}
