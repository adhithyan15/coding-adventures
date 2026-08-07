// ---------------------------------------------------------------------------
// session.ts — the SESSION ORCHESTRATOR (HL03 phase 3)
// ---------------------------------------------------------------------------
//
// Phase 2 (`sequence.ts`) computed the TEACHING SWEEP: for one concept, the
// active languages that teach it, walked in chain order. That is the skeleton
// of a study session. This module puts flesh on it — the CONNECTIONS that make
// interleaving worth doing.
//
// As the sweep moves forward through the chain (Spanish, then Latin, then
// French, …), each new language is presented alongside the links back to the
// languages already seen this pass: where a word shares an etymological ROOT
// with one the learner just met, the app says so. Meeting "thank you" in Telugu
// right after Kannada and Hindi, and being shown that all three carry the
// Sanskrit root *dhanya*, is the moment the three stop being three separate
// facts and become one.
//
// The rule this module keeps: a connection is only ever asserted when the DATA
// says so. A connection between two stops exists iff their lessons literally
// share a root string (from `lesson.roots`, sourced from the curriculum). No
// connection is inferred, guessed, or invented — if two languages' words for a
// concept share no root in the data, the app shows no link between them.
//
// Pure and deterministic, like the sequencing layer it builds on: no UI, no
// review scheduling. Those are later phases.
// ---------------------------------------------------------------------------

import type { Lesson } from "./lessons";
import { resolveActiveLanguages, teachingSweep, type ChainLanguage, type LanguageSelection, type SweepStop } from "./sequence";

/**
 * A link from one stop in the sweep back to an EARLIER one, justified by the
 * roots their lessons share. `to` is always a language that came before this
 * step in chain order; `sharedRoots` is why they are linked.
 */
export interface Connection {
  to: ChainLanguage;
  /** The root strings both stops' lessons cite. Sorted, deduped, non-empty. */
  sharedRoots: string[];
}

/**
 * One step of a session: a language, the lesson(s) that teach the concept there,
 * and the connections back to earlier languages in the same sweep. The first
 * stop always has no connections (nothing precedes it); later stops have one
 * connection per earlier language they share a root with.
 */
export interface SessionStep {
  language: ChainLanguage;
  lessons: Lesson[];
  connections: Connection[];
}

/** The union of every root cited by a stop's lessons. */
function rootsOfStop(stop: SweepStop): Set<string> {
  const roots = new Set<string>();
  for (const lesson of stop.lessons) for (const r of lesson.roots) roots.add(r);
  return roots;
}

/**
 * Assemble a session for one concept over the active chain prefix.
 *
 * Runs the teaching sweep, then for each stop finds its connections back to the
 * stops already seen (earlier in chain order) whose lessons share a root. The
 * result is the sweep annotated with grounded cross-language links.
 */
export function buildSession(
  concept: string,
  lessons: Lesson[],
  active: LanguageSelection,
): SessionStep[] {
  const sweep = teachingSweep(concept, lessons, resolveActiveLanguages(active));

  const steps: SessionStep[] = [];
  const seen: Array<{ language: ChainLanguage; roots: Set<string> }> = [];

  for (const stop of sweep) {
    const myRoots = rootsOfStop(stop);
    const connections: Connection[] = [];
    for (const earlier of seen) {
      const shared = [...myRoots].filter((r) => earlier.roots.has(r)).sort();
      if (shared.length > 0) connections.push({ to: earlier.language, sharedRoots: shared });
    }
    steps.push({ language: stop.language, lessons: stop.lessons, connections });
    seen.push({ language: stop.language, roots: myRoots });
  }

  return steps;
}

/** Every distinct connected pair in a session — handy for tests and summaries. */
export function connectionPairs(steps: SessionStep[]): Array<{ from: ChainLanguage; to: ChainLanguage; roots: string[] }> {
  const pairs: Array<{ from: ChainLanguage; to: ChainLanguage; roots: string[] }> = [];
  for (const step of steps) {
    for (const c of step.connections) {
      pairs.push({ from: step.language, to: c.to, roots: c.sharedRoots });
    }
  }
  return pairs;
}
