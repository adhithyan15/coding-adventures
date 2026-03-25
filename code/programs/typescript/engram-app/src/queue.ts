/**
 * queue.ts — Session queue assembly for spaced repetition study sessions.
 *
 * Each study session works through a fixed queue of cards assembled at
 * session start. The queue is a blend of two sources:
 *
 *   Due cards   — cards the user has seen before and whose nextDueAt
 *                 timestamp has been reached. These are "on the schedule".
 *                 Sorted most-overdue-first so urgent reviews come first.
 *
 *   New cards   — cards the user has never seen (no CardProgress record).
 *                 A fresh batch is introduced each session to keep the
 *                 user making forward progress through the deck.
 *
 * === Why cap new cards per session? ===
 *
 * Introducing too many new cards at once is counterproductive. The user
 * hasn't had time to form initial memories for the new cards before they
 * need to be reviewed. A cap of ~7 new cards per session balances progress
 * with retention — enough to feel like you're moving forward, few enough
 * that the new cards can actually be remembered.
 *
 * === Why sort due cards most-overdue-first? ===
 *
 * An overdue card is one whose scheduled review date has already passed.
 * The longer a card has been overdue, the more likely the user has already
 * forgotten it. Reviewing the most overdue cards first minimises total
 * forgetting across the session.
 *
 * === All functions are pure ===
 *
 * No side effects, no mutation. Pass in the full state arrays, get back
 * a new array. This makes the queue logic trivially testable.
 */

import type { Card, CardProgress } from "./types.js";

/**
 * SESSION_SIZE — maximum number of cards per session.
 *
 * Chosen to be long enough to feel productive (~20 minutes at 1 min/card)
 * but short enough to complete in a single sitting without fatigue.
 */
export const SESSION_SIZE = 20;

/**
 * MAX_NEW_PER_SESSION — maximum new cards introduced per session.
 *
 * Caps the introduction of unseen cards to avoid overwhelming the user
 * with too many new items before they've had time to form initial memories.
 */
export const MAX_NEW_PER_SESSION = 7;

/**
 * buildSessionQueue — assemble the card queue for a new study session.
 *
 * Algorithm:
 *   1. Partition deck cards into "due" and "new" pools.
 *   2. Take up to MAX_NEW_PER_SESSION new cards.
 *   3. Fill remaining slots (up to SESSION_SIZE - newCards.length) with
 *      the most overdue due cards.
 *   4. Return due cards first, then new cards.
 *      (New cards at the end means they arrive after the user has warmed
 *       up with familiar material.)
 *
 * Returns an empty array if the deck is fully caught up (no due or new cards).
 *
 * @param allCards    - All cards in the app (will be filtered to the deck).
 * @param allProgress - All CardProgress records (used to classify cards).
 * @param deckId      - The deck being studied.
 * @param now         - Current timestamp in ms (defaults to Date.now()).
 */
export function buildSessionQueue(
  allCards: Card[],
  allProgress: CardProgress[],
  deckId: string,
  now: number = Date.now(),
): Card[] {
  // Filter to only cards belonging to this deck
  const deckCards = allCards.filter((c) => c.deckId === deckId);

  // Build a map for O(1) lookups: cardId → CardProgress
  const progressMap = new Map<string, CardProgress>(
    allProgress.map((p) => [p.cardId, p]),
  );

  // ── Due cards ─────────────────────────────────────────────────────────────
  //
  // A card is "due" when it has a progress record AND nextDueAt <= now.
  // Sort ascending by nextDueAt so the most overdue card appears first.

  const dueCards = deckCards
    .filter((c) => {
      const p = progressMap.get(c.id);
      return p !== undefined && p.nextDueAt <= now;
    })
    .sort((a, b) => {
      const pa = progressMap.get(a.id)!;
      const pb = progressMap.get(b.id)!;
      return pa.nextDueAt - pb.nextDueAt; // Most overdue first
    });

  // ── New cards ─────────────────────────────────────────────────────────────
  //
  // A card is "new" when it has NO progress record at all.
  // Preserve the deck order (by createdAt — which matches the array order
  // from the seed) so the user learns cards in a logical sequence.

  const newCards = deckCards
    .filter((c) => !progressMap.has(c.id))
    .slice(0, MAX_NEW_PER_SESSION);

  // ── Assemble ──────────────────────────────────────────────────────────────
  //
  // New cards take up to MAX_NEW_PER_SESSION slots.
  // Due cards fill the remaining slots up to SESSION_SIZE.

  const reviewSlots = Math.min(
    dueCards.length,
    SESSION_SIZE - newCards.length,
  );

  return [...dueCards.slice(0, reviewSlots), ...newCards];
}

/**
 * isDeckCaughtUp — returns true when there are no due or new cards.
 *
 * Used to disable the "Study" button when the user is fully caught up.
 *
 * @param allCards    - All cards in the app.
 * @param allProgress - All CardProgress records.
 * @param deckId      - The deck to check.
 * @param now         - Current timestamp in ms (defaults to Date.now()).
 */
export function isDeckCaughtUp(
  allCards: Card[],
  allProgress: CardProgress[],
  deckId: string,
  now: number = Date.now(),
): boolean {
  return buildSessionQueue(allCards, allProgress, deckId, now).length === 0;
}

/**
 * getDeckStats — compute per-deck card counts for the stats screen.
 *
 * Categorises cards into three buckets:
 *   new      — never reviewed (no progress record)
 *   learning — reviewed, interval ≤ 21 days (still being actively memorised)
 *   mastered — interval > 21 days (well-retained, infrequent reviews)
 *
 * The 21-day threshold is a convention (Anki uses similar heuristics for
 * its "mature" cards). Cards reviewed for 3+ weeks are considered stable
 * in long-term memory.
 */
export interface DeckStats {
  total: number;
  newCount: number;
  learningCount: number;
  masteredCount: number;
  dueCount: number;
  averageEaseFactor: number;
}

export function getDeckStats(
  allCards: Card[],
  allProgress: CardProgress[],
  deckId: string,
  now: number = Date.now(),
): DeckStats {
  const deckCards = allCards.filter((c) => c.deckId === deckId);
  const progressMap = new Map<string, CardProgress>(
    allProgress.map((p) => [p.cardId, p]),
  );

  let newCount = 0;
  let learningCount = 0;
  let masteredCount = 0;
  let dueCount = 0;
  let easeSum = 0;
  let easeCount = 0;

  for (const card of deckCards) {
    const p = progressMap.get(card.id);
    if (!p) {
      newCount++;
    } else {
      if (p.interval > 21) {
        masteredCount++;
      } else {
        learningCount++;
      }
      if (p.nextDueAt <= now) {
        dueCount++;
      }
      easeSum += p.easeFactor;
      easeCount++;
    }
  }

  return {
    total: deckCards.length,
    newCount,
    learningCount,
    masteredCount,
    dueCount,
    averageEaseFactor: easeCount > 0 ? easeSum / easeCount : 0,
  };
}
