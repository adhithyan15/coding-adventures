/**
 * sm2.ts — The SM-2 spaced repetition scheduling algorithm.
 *
 * SM-2 was originally published by Piotr Wozniak in 1987 as part of the
 * SuperMemo software. It models human memory as an exponentially decaying
 * function: we forget things faster after long gaps without review.
 *
 * The key insight is that reviewing a fact *just before* you forget it
 * maximises retention while minimising total review time. SM-2 predicts
 * the optimal moment for each review based on past performance.
 *
 * === How it works ===
 *
 * Two numbers describe your relationship with each card:
 *
 *   interval (days)
 *     How long until you need to review this card again. Starts at 1 day.
 *     Grows multiplicatively on each successful recall. Resets to 1 on
 *     a failed recall ("again"). This is the core of spaced repetition:
 *     successful recalls push the next review further into the future.
 *
 *   easeFactor (multiplier, range [1.3, 4.0])
 *     Controls how fast the interval grows. A card you always find easy
 *     will have a high ease factor (intervals grow quickly — you rarely
 *     need to review it). A card you struggle with will have a low ease
 *     factor (intervals grow slowly — you need frequent practice).
 *     Starts at 2.5 for all cards.
 *
 * === Rating scores ===
 *
 *   again (0) — complete blank or wrong answer
 *   hard  (1) — correct but required significant effort
 *   good  (2) — correct with normal effort  ← the "ideal" rating
 *   easy  (3) — immediate, effortless recall
 *
 * === Update rules ===
 *
 *   again: interval ← 1,  easeFactor ← max(1.3, ef - 0.20)
 *   hard:  interval ← max(1, round(interval × 1.2)),
 *          easeFactor ← max(1.3, ef - 0.15)
 *   good:  interval ← max(1, round(interval × easeFactor))
 *   easy:  interval ← max(1, round(interval × easeFactor × 1.3)),
 *          easeFactor ← min(4.0, ef + 0.15)
 *
 * The next due date is always: now + interval × 24h in milliseconds.
 *
 * === Initial state ===
 *
 * New cards (never reviewed) have no CardProgress record. When first
 * reviewed, a fresh record is created with interval=1, easeFactor=2.5,
 * then the rating is applied on top via updateCardProgress().
 *
 * === This module ===
 *
 * All functions are pure — no side effects, no mutation. Pass in a record,
 * get back a new record. This makes the algorithm trivially testable and
 * easy to reason about.
 */

import type { CardProgress, Rating } from "./types.js";

/**
 * INITIAL_EASE_FACTOR — the ease factor assigned to every new card.
 *
 * 2.5 is Anki's default. It means the interval roughly doubles each time
 * you correctly recall the card (e.g., 1 → 3 → 7 → 17 → 43 days...).
 */
export const INITIAL_EASE_FACTOR = 2.5;

/**
 * MIN_EASE_FACTOR — the floor for the ease factor.
 *
 * A card can never become "harder" than this threshold, no matter how many
 * times you rate it "again". This prevents the interval from stagnating
 * permanently on difficult cards.
 */
export const MIN_EASE_FACTOR = 1.3;

/**
 * MAX_EASE_FACTOR — the ceiling for the ease factor.
 *
 * Prevents extremely easy cards from growing intervals so fast they
 * are never reviewed at all.
 */
export const MAX_EASE_FACTOR = 4.0;

/** Score value for each rating. Drives the SM-2 update logic. */
const RATING_SCORE: Record<Rating, number> = {
  again: 0,
  hard: 1,
  good: 2,
  easy: 3,
};

/**
 * createInitialProgress — build a CardProgress record for a card's first review.
 *
 * Called when a card has no existing progress record and is being reviewed
 * for the first time. Creates a baseline record with SM-2 defaults, then
 * immediately applies the user's rating via updateCardProgress().
 *
 * @param cardId - The ID of the card being reviewed for the first time.
 * @param rating - The user's recall quality rating.
 * @param now    - Current timestamp in ms (defaults to Date.now()).
 */
export function createInitialProgress(
  cardId: string,
  rating: Rating,
  now: number = Date.now(),
): CardProgress {
  const initial: CardProgress = {
    cardId,
    interval: 1,
    easeFactor: INITIAL_EASE_FACTOR,
    nextDueAt: now + 24 * 60 * 60 * 1000,
    timesSeen: 0,
    timesCorrect: 0,
    timesIncorrect: 0,
    lastSeenAt: now,
  };
  return updateCardProgress(initial, rating, now);
}

/**
 * updateCardProgress — apply SM-2 to compute the next review schedule.
 *
 * Takes the current CardProgress record and the user's rating, returns a
 * new record with updated interval, easeFactor, nextDueAt, and counters.
 * Does NOT mutate the input record.
 *
 * @param progress - Current SM-2 state for this card.
 * @param rating   - User's recall quality rating.
 * @param now      - Current timestamp in ms (defaults to Date.now()).
 */
export function updateCardProgress(
  progress: CardProgress,
  rating: Rating,
  now: number = Date.now(),
): CardProgress {
  const score = RATING_SCORE[rating];
  let { interval, easeFactor } = progress;

  if (score === 0) {
    // again — complete failure. Reset interval to 1, decrease ease.
    // The card goes back to the beginning of the learning sequence.
    interval = 1;
    easeFactor = Math.max(MIN_EASE_FACTOR, easeFactor - 0.2);
  } else if (score === 1) {
    // hard — correct but effortful. Slow growth, slightly decrease ease.
    // The ×1.2 factor is less than the minimum ease factor (1.3), so
    // this always grows slower than "good" would.
    interval = Math.max(1, Math.round(interval * 1.2));
    easeFactor = Math.max(MIN_EASE_FACTOR, easeFactor - 0.15);
  } else if (score === 2) {
    // good — normal recall. Standard growth via the ease factor.
    // This is the "ideal" path: consistent "good" ratings produce
    // an exponentially growing interval.
    interval = Math.max(1, Math.round(interval * easeFactor));
  } else {
    // easy — effortless recall. Bonus growth, increase ease factor.
    // The ×1.3 bonus means "easy" intervals grow faster than "good".
    // The ease factor increase means future "good" recalls also grow faster.
    interval = Math.max(1, Math.round(interval * easeFactor * 1.3));
    easeFactor = Math.min(MAX_EASE_FACTOR, easeFactor + 0.15);
  }

  const isCorrect = score > 0;

  return {
    ...progress,
    interval,
    easeFactor,
    nextDueAt: now + interval * 24 * 60 * 60 * 1000,
    lastSeenAt: now,
    timesSeen: progress.timesSeen + 1,
    timesCorrect: isCorrect ? progress.timesCorrect + 1 : progress.timesCorrect,
    timesIncorrect: !isCorrect
      ? progress.timesIncorrect + 1
      : progress.timesIncorrect,
  };
}
