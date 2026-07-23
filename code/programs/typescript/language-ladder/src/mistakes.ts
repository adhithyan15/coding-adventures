// ---------------------------------------------------------------------------
// mistakes.ts — the mistakes store (HL03 phase 5)
// ---------------------------------------------------------------------------
//
// A quiz that only says "wrong" and moves on wastes the most useful signal a
// learner produces: WHAT they picked instead. Choosing the French cognate's
// meaning for the Spanish word is not noise — it is the exact cross-language
// confusion the interleaving is meant to expose, and it is worth surfacing back
// ("the ones you keep mixing up") rather than discarding.
//
// This module records each answer, feeds a miss back into the SRS so the item
// returns sooner, and rolls the wrong answers up into ranked confusions. Every
// number it reports is GROUNDED in something actually recorded — a confusion is
// a real (chosen, correct) pair the learner produced, never an inferred one.
//
// Pure and deterministic: recording appends to a fresh log, demote returns fresh
// state, confusions is a pure fold. No I/O, no clock — the caller passes the
// session index in.
// ---------------------------------------------------------------------------

import type { QuizState } from "./quiz";

/**
 * One answer the learner gave. `cellKey` is the item asked; `correct` is whether
 * they got it right; `chosenKey` is what they picked WHEN WRONG (the confusion) —
 * omitted for a correct answer or a typed/free answer with no distractor.
 */
export interface AnswerRecord {
  cellKey: string;
  correct: boolean;
  chosenKey?: string;
}

/** Append an answer to the log, returning a new log (the old one is untouched). */
export function recordAnswer(
  log: AnswerRecord[],
  cellKey: string,
  correct: boolean,
  chosenKey?: string,
): AnswerRecord[] {
  const record: AnswerRecord = { cellKey, correct };
  if (!correct && chosenKey !== undefined) record.chosenKey = chosenKey;
  return [...log, record];
}

/**
 * Demote a cell after a MISS: reset it to Leitner box 0, make it due right now
 * (so `pickNext` weights it heavily and it resurfaces soon), and count the
 * lapse. Returns fresh state; the caller applies this only on a wrong answer.
 */
export function demote(state: QuizState, session: number): QuizState {
  return {
    box: 0,
    dueAtSession: session,
    lapses: state.lapses + 1,
    reps: state.reps + 1,
  };
}

/** A pair the learner mixed up, and how often. */
export interface Confusion {
  /** The item that was asked. */
  correct: string;
  /** The item they picked instead. */
  chosen: string;
  count: number;
}

/**
 * Roll the wrong answers up into ranked confusions — for each (chosen instead of
 * correct) pair, how many times it happened, most frequent first. Only wrong
 * answers that recorded a `chosenKey` contribute; correct answers and misses
 * without a chosen distractor are ignored, so a confusion never appears unless
 * the learner actually made it.
 */
export function confusions(log: AnswerRecord[]): Confusion[] {
  const counts = new Map<string, number>();
  for (const r of log) {
    if (r.correct || r.chosenKey === undefined) continue;
    const key = JSON.stringify([r.cellKey, r.chosenKey]);
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  const out: Confusion[] = [];
  for (const [key, count] of counts) {
    const [correct, chosen] = JSON.parse(key) as [string, string];
    out.push({ correct, chosen, count });
  }
  // Most-confused first; ties broken by the pair encoding for determinism.
  out.sort((a, b) => b.count - a.count || (a.correct < b.correct ? -1 : a.correct > b.correct ? 1 : a.chosen < b.chosen ? -1 : 1));
  return out;
}
