// ---------------------------------------------------------------------------
// scheduler.ts — the spaced-repetition heart of Practice mode (HL02).
//
// Recognition and recall build memory; *spacing* makes it last. This is a
// Leitner / SM-2-lite scheduler measured in SESSIONS (not wall-clock, so there
// is no Date dependency and everything is reproducible). Each answered question
// advances the session clock by one; a letter you get right drifts further into
// the future on an expanding schedule (1 → 3 → 7 → 15 → 30 sessions), while a
// letter you miss drops back to "due now" and resurfaces quickly.
//
// Pure and deterministic — no Date, no Math.random. The UI owns the session
// counter and the per-script state; this module only computes the next state
// and which item is most due. That is exactly what makes it unit-testable, and
// per HL02 it is "the core module … where test coverage matters most."
// ---------------------------------------------------------------------------

/** Scheduling state for one practice item (here, one letter of a script). */
export interface ItemState {
  /** The item id — an index into the script's letters. */
  letterIndex: number;
  /** Leitner box 0..MAX_BOX; higher = longer interval. */
  box: number;
  /** Session index at which this item next becomes due. */
  dueAtSession: number;
  /** Session index when it first appeared. */
  introducedAt: number;
  /** Times failed. */
  lapses: number;
  /** Total reviews (used for tie-breaking and a mastery read-out). */
  reps: number;
}

/**
 * Box → interval in sessions until next due. Matches HL00's open-loop
 * N+1/N+3/N+7/N+15 baseline, now closed-loop. Box 0 (new or just-lapsed) comes
 * back next session; each promotion roughly triples the gap.
 */
export const INTERVALS = [1, 1, 3, 7, 15, 30];
export const MAX_BOX = INTERVALS.length - 1;

/** Interval for a given box, clamped into range. */
export function intervalFor(box: number): number {
  const b = Math.max(0, Math.min(MAX_BOX, box));
  return INTERVALS[b]!;
}

/** Fresh state for `count` items, all due at `session` (default 0). */
export function initStates(count: number, session = 0): ItemState[] {
  return Array.from({ length: Math.max(0, count) }, (_, i) => ({
    letterIndex: i,
    box: 0,
    dueAtSession: session,
    introducedAt: session,
    lapses: 0,
    reps: 0,
  }));
}

/** Is this item due at (or before) the given session? */
export function isDue(item: ItemState, session: number): boolean {
  return item.dueAtSession <= session;
}

/** How many items are due at `session`. */
export function dueCount(items: ItemState[], session: number): number {
  return items.filter((it) => isDue(it, session)).length;
}

/** How many items have reached a "learned" box (default: box >= 3). */
export function masteredCount(items: ItemState[], minBox = 3): number {
  return items.filter((it) => it.box >= minBox).length;
}

/**
 * Choose the next item to practise, deterministically.
 *
 * Prefer items that are **due** (dueAtSession <= session); among those, the most
 * overdue (smallest dueAtSession) wins, tie-broken by fewest reps then lowest
 * letterIndex. If nothing is due yet, fall back to the soonest-due item so
 * practice never stalls. Returns the item's letterIndex, or -1 if empty.
 */
export function pickNext(items: ItemState[], session: number): number {
  if (items.length === 0) return -1;
  const due = items.filter((it) => isDue(it, session));
  const pool = due.length > 0 ? due : items;
  let best = pool[0]!;
  for (const it of pool) {
    if (
      it.dueAtSession < best.dueAtSession ||
      (it.dueAtSession === best.dueAtSession && it.reps < best.reps) ||
      (it.dueAtSession === best.dueAtSession && it.reps === best.reps && it.letterIndex < best.letterIndex)
    ) {
      best = it;
    }
  }
  return best.letterIndex;
}

/**
 * Fold one answer into an item's state (immutably), at the given session.
 *
 * Correct → promote a box (capped) and schedule by the *new* box's interval, so
 * the gap expands. Wrong → drop to box 0 (due again very soon) and count a lapse.
 */
export function review(item: ItemState, wasCorrect: boolean, session: number): ItemState {
  if (wasCorrect) {
    const box = Math.min(item.box + 1, MAX_BOX);
    return {
      ...item,
      box,
      dueAtSession: session + intervalFor(box),
      reps: item.reps + 1,
    };
  }
  return {
    ...item,
    box: 0,
    dueAtSession: session + intervalFor(0),
    lapses: item.lapses + 1,
    reps: item.reps + 1,
  };
}

/** Apply `review` to whichever item matches `letterIndex`, leaving others as-is. */
export function reviewIn(
  items: ItemState[],
  letterIndex: number,
  wasCorrect: boolean,
  session: number,
): ItemState[] {
  return items.map((it) => (it.letterIndex === letterIndex ? review(it, wasCorrect, session) : it));
}
