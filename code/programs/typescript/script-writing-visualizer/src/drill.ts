// ---------------------------------------------------------------------------
// drill.ts — the pure logic of the "recall" practice mode.
//
// Recognition is the first half of reading a script; recall is the second:
// given a *sound*, can you produce the *glyph*? This module builds a
// multiple-choice recall question and scores answers. Like core.ts it is
// deterministic and DOM-free — all randomness is INJECTED by the caller (the UI
// passes a seeded picker / position), so every function here is unit-testable
// without a browser and without flaky Math.random.
// ---------------------------------------------------------------------------

import type { LetterView } from "./core.ts";

/** One selectable answer: a glyph, tagged with the letter it came from. */
export interface DrillOption {
  glyph: string;
  /** Index into the original letters array (lets the UI show its decomposition). */
  letterIndex: number;
}

/** A single recall question: "which glyph makes this sound?" */
export interface DrillQuestion {
  /** The prompt shown to the learner — the target letter's sound. */
  promptSound: string;
  /** The correct glyph (also findable at options[answerIndex]). */
  targetGlyph: string;
  targetIndex: number;
  /** The shuffled choices. */
  options: DrillOption[];
  /** Which option is correct. */
  answerIndex: number;
}

/**
 * Rank the *other* letters by how confusable they are with the target, so a
 * question's wrong answers are meaningfully hard (not random noise). The score
 * favours letters that share the target's role (consonant/vowel) and its
 * false-friend status; ties keep inventory order (stable). Pure and
 * deterministic — no randomness.
 *
 * Returns candidate indices, most-confusable first, excluding the target.
 */
export function confusabilityOrder(letters: LetterView[], targetIndex: number): number[] {
  const target = letters[targetIndex];
  if (!target) return [];
  const candidates = letters
    .map((_, i) => i)
    .filter((i) => i !== targetIndex);
  const score = (i: number): number => {
    const c = letters[i]!;
    return (c.role === target.role ? 2 : 0) + (c.falseFriend === target.falseFriend ? 1 : 0);
  };
  // Stable sort by score descending (Array.sort is stable in modern JS, but we
  // tie-break on original index explicitly to be certain across engines).
  return candidates.sort((a, b) => score(b) - score(a) || a - b);
}

/** Default distractor chooser: the top-N most-confusable, deterministically. */
export function topDistractors(ranked: number[], count: number): number[] {
  return ranked.slice(0, Math.max(0, count));
}

/**
 * Build a recall question for `targetIndex`.
 *
 * @param optionCount  total choices to show (clamped to what's available).
 * @param choose       picks the distractor indices from the confusability-ranked
 *                     candidates. Defaults to the top-N; the UI can pass a
 *                     seeded picker for variety. MUST return distinct indices,
 *                     none equal to targetIndex.
 * @param placeAt      where the correct answer sits among the options (injected
 *                     so tests are deterministic and the answer isn't always
 *                     first). Clamped into range.
 */
export function buildDrillQuestion(
  letters: LetterView[],
  targetIndex: number,
  optionCount = 4,
  choose: (ranked: number[], count: number) => number[] = topDistractors,
  placeAt = 0,
): DrillQuestion {
  const target = letters[targetIndex];
  if (!target) throw new RangeError(`targetIndex ${targetIndex} out of range`);

  const ranked = confusabilityOrder(letters, targetIndex);
  const wantDistractors = Math.max(0, Math.min(optionCount, letters.length) - 1);

  // Take the chooser's picks, but defensively drop the target and duplicates and
  // anything out of range, then top up from the ranked list if the chooser
  // returned too few. This keeps the function total even for a sloppy chooser.
  const seen = new Set<number>();
  const distractors: number[] = [];
  const consider = (i: number) => {
    if (i === targetIndex || seen.has(i) || i < 0 || i >= letters.length) return;
    seen.add(i);
    distractors.push(i);
  };
  choose(ranked, wantDistractors).forEach(consider);
  for (const i of ranked) {
    if (distractors.length >= wantDistractors) break;
    consider(i);
  }

  const options: DrillOption[] = distractors.map((i) => ({ glyph: letters[i]!.glyph, letterIndex: i }));
  const at = clamp(placeAt, 0, options.length);
  options.splice(at, 0, { glyph: target.glyph, letterIndex: targetIndex });

  return {
    promptSound: target.sound,
    targetGlyph: target.glyph,
    targetIndex,
    options,
    answerIndex: at,
  };
}

/** True when the chosen option is the correct one. */
export function checkAnswer(q: DrillQuestion, chosenIndex: number): boolean {
  return chosenIndex === q.answerIndex;
}

// --- scoring ----------------------------------------------------------------

export interface Score {
  correct: number;
  total: number;
}

export const emptyScore = (): Score => ({ correct: 0, total: 0 });

/** Fold one answer into the running score (immutably). */
export function record(score: Score, wasCorrect: boolean): Score {
  return { correct: score.correct + (wasCorrect ? 1 : 0), total: score.total + 1 };
}

/** Percentage 0–100, or null before any answer (avoids 0/0). */
export function accuracy(score: Score): number | null {
  return score.total === 0 ? null : Math.round((score.correct / score.total) * 100);
}

function clamp(n: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, n));
}
