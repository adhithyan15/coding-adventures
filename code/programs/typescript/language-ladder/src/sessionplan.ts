// ---------------------------------------------------------------------------
// sessionplan.ts — the session view-model (HL03 phase 6, slice 6a)
// ---------------------------------------------------------------------------
//
// The engine is complete but scattered across four pure modules — the sweep
// (session.ts), the connections it carries, the covered grid and the weighted
// draw (quiz.ts), and the mistakes store (mistakes.ts). This file is the thin
// seam that assembles them into the shape a single study session actually has,
// with no DOM in sight: the UI layer (slice 6b) renders what this returns and
// hands answers back to it.
//
// A session is two passes:
//   • the TEACHING pass — the current concept walked forward across the active
//     chain, each stop annotated with its connections back to earlier languages;
//   • the REVIEW pass — a weighted draw over everything covered so far.
//
// And it threads the state that makes the review adaptive: each answer updates
// the per-cell Leitner state (promote on a hit, demote on a miss) and appends to
// the mistakes log, so the next `pickNext` leans on what was just missed. All of
// that is done here as pure functions over immutable inputs.
// ---------------------------------------------------------------------------

import type { Lesson } from "./lessons";
import { buildSession, type SessionStep } from "./session";
import { coveredGrid, cellKey, type GridCell, type QuizState } from "./quiz";
import { recordAnswer, demote, type AnswerRecord } from "./mistakes";
import { intervalFor, MAX_BOX } from "./scheduler";
import type { LanguageSelection } from "./sequence";

/** Everything the UI needs to run one session, assembled from the engine. */
export interface SessionPlan {
  concept: string;
  /** The teaching sweep: the concept across the active chain, with connections. */
  teaching: SessionStep[];
  /** The pool the review quiz draws from — everything covered so far. */
  reviewGrid: GridCell[];
}

/**
 * Assemble a session for the current concept.
 *
 * `covered` is every concept studied so far (it should include `current`); the
 * teaching pass is `current` alone, the review pass is the whole covered grid.
 */
export function planSession(
  current: string,
  covered: Iterable<string>,
  lessons: Lesson[],
  active: LanguageSelection,
): SessionPlan {
  return {
    concept: current,
    teaching: buildSession(current, lessons, active),
    reviewGrid: coveredGrid(covered, lessons, active),
  };
}

/** The mutable progress a session accumulates: per-cell SRS state + the log. */
export interface Progress {
  states: Map<string, QuizState>;
  log: AnswerRecord[];
}

/** A fresh, empty progress. */
export function initProgress(): Progress {
  return { states: new Map(), log: [] };
}

/** Promote a cell after a HIT: up a box (capped), due after the new interval. */
function promote(state: QuizState, session: number): QuizState {
  const box = Math.min(MAX_BOX, state.box + 1);
  return { box, dueAtSession: session + intervalFor(box), lapses: state.lapses, reps: state.reps + 1 };
}

/** The state a cell starts from the first time it is answered. */
function seed(session: number): QuizState {
  return { box: 0, dueAtSession: session, lapses: 0, reps: 0 };
}

/**
 * Apply one quiz answer, returning fresh progress (inputs untouched).
 *
 * A correct answer promotes the cell (it comes back later); a wrong answer
 * demotes it (box 0, due now, lapse) so it resurfaces soon, and records the
 * confusion (`chosenKey`, what was picked instead). Either way the answer is
 * logged.
 */
export function applyAnswer(
  progress: Progress,
  cell: GridCell,
  correct: boolean,
  session: number,
  chosenKey?: string,
): Progress {
  const key = cellKey(cell);
  const prev = progress.states.get(key) ?? seed(session);
  const next = correct ? promote(prev, session) : demote(prev, session);

  const states = new Map(progress.states);
  states.set(key, next);
  return {
    states,
    log: recordAnswer(progress.log, key, correct, chosenKey),
  };
}
