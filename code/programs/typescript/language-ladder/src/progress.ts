// ---------------------------------------------------------------------------
// progress.ts — remembering what you have learned, between visits.
//
// THE PROBLEM THIS SOLVES. `scheduler.ts` implements a perfectly good Leitner
// system: every item sits in a box, boxes map to intervals, a miss knocks an
// item back to "due now". But it kept that state in memory only, so every page
// reload wiped it. The app could schedule you, but it could not *remember* you
// — which made the whole spaced-repetition idea decorative.
//
// THE DESIGN. Two rules keep this honest:
//
//   1. `scheduler.ts` stays PURE and index-based. It never learns what an item
//      is or where it is stored. Everything here converts between its
//      `ItemState[]` (positional) and a saved record (keyed by lesson id).
//
//   2. We save by lesson **id**, never by index. Indices shift the moment a
//      lesson is added — and lessons are added constantly. Saving by position
//      would silently reattribute your progress to the wrong lesson, which is
//      worse than losing it. Keyed by id, adding a lesson simply means one more
//      unseen item.
//
// Everything except the tiny `localStorage` adapter at the bottom is pure, so
// the round-trip is unit-testable without a browser.
// ---------------------------------------------------------------------------

import { initStates, MAX_BOX, type ItemState } from "./scheduler.ts";

/** Bump when the saved shape changes incompatibly; older payloads are dropped. */
export const SCHEMA_VERSION = 1;

/** The key we own in localStorage. Namespaced so nothing else collides. */
export const STORAGE_KEY = "hl-study:progress:v1";

/**
 * The per-item history we persist.
 *
 * Note what is NOT here: `letterIndex`. That field is positional — it is the
 * item's slot in the current run's array — so saving it would be saving exactly
 * the thing that goes stale when the curriculum grows. We rebuild it on load.
 */
export interface SavedItem {
  box: number;
  dueAtSession: number;
  introducedAt: number;
  lapses: number;
  reps: number;
}

/** What we persist. Deliberately small, flat and boring. */
export interface SavedProgress {
  version: number;
  /** Session counter — the scheduler's notion of "now". */
  session: number;
  /** lesson id → its history. */
  items: Record<string, SavedItem>;
}

/** A fresh, empty record. */
export function emptyProgress(): SavedProgress {
  return { version: SCHEMA_VERSION, session: 0, items: Object.create(null) };
}

/** Positional scheduler state → a saveable, id-keyed record. */
export function toSaved(
  ids: string[],
  states: ItemState[],
  session: number,
): SavedProgress {
  const items: Record<string, SavedItem> = Object.create(null);
  ids.forEach((id, index) => {
    const state = states[index];
    if (!state) return;
    // Don't persist untouched items: an unseen lesson is the default, so
    // storing it wastes space and makes the payload grow with the curriculum
    // rather than with what you've actually studied.
    //
    // Test this ONLY against review history (`reps`/`lapses`/`box`), never
    // against `dueAtSession`. Fresh items are seeded with the CURRENT session,
    // so on any reload after the first, `dueAtSession` is non-zero for every
    // unseen lesson — a guard that consulted it would quietly fail open from
    // session 1 onward and save all 679 lessons instead of the handful studied.
    if (state.reps === 0 && state.lapses === 0 && state.box === 0) return;
    items[id] = {
      box: state.box,
      dueAtSession: state.dueAtSession,
      introducedAt: state.introducedAt,
      lapses: state.lapses,
      reps: state.reps,
    };
  });
  return { version: SCHEMA_VERSION, session, items };
}

/**
 * A saved record → positional scheduler state for `ids`.
 *
 * Anything we don't recognise becomes a fresh item: lessons added since the
 * save, ids that vanished (renamed lessons), corrupt entries. That is the whole
 * migration story, and it is why saving by id was worth the trouble.
 */
export function fromSaved(ids: string[], saved: SavedProgress): ItemState[] {
  const states = initStates(ids.length, saved.session);
  ids.forEach((id, index) => {
    const entry = saved.items[id];
    const base = states[index];
    if (!entry || !base) return;
    states[index] = {
      // letterIndex is positional: keep the freshly-built one, never the saved
      // one. This is what makes progress survive the curriculum growing.
      letterIndex: base.letterIndex,
      box: clampBox(entry.box),
      dueAtSession: int(entry.dueAtSession),
      introducedAt: int(entry.introducedAt),
      lapses: Math.max(0, int(entry.lapses)),
      reps: Math.max(0, int(entry.reps)),
    };
  });
  return states;
}

function int(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.trunc(value) : 0;
}

function clampBox(box: unknown): number {
  return Math.max(0, Math.min(MAX_BOX, int(box)));
}

/**
 * Parse whatever was in storage, defensively.
 *
 * This input is untrusted: a user can edit localStorage by hand, another tab
 * can leave a half-written value, and an older build can leave an older shape.
 * So we validate every field and fall back to `emptyProgress()` rather than
 * throwing — losing progress is bad, but a study app that won't start because
 * of one bad key is worse.
 *
 * Note `Object.create(null)` for the item map: a saved payload containing a
 * `__proto__` key must not be able to reach Object.prototype.
 */
export function parseProgress(raw: string | null): SavedProgress {
  if (raw === null || raw === "") return emptyProgress();

  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return emptyProgress();
  }
  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    return emptyProgress();
  }

  const record = data as Record<string, unknown>;
  if (record.version !== SCHEMA_VERSION) return emptyProgress();

  const session =
    typeof record.session === "number" && Number.isFinite(record.session)
      ? Math.max(0, Math.trunc(record.session))
      : 0;

  const items: Record<string, SavedItem> = Object.create(null);
  const rawItems = record.items;
  if (typeof rawItems === "object" && rawItems !== null && !Array.isArray(rawItems)) {
    // Own keys only — never walk the prototype chain.
    for (const [id, value] of Object.entries(rawItems)) {
      if (id === "__proto__" || id === "constructor" || id === "prototype") continue;
      if (typeof value !== "object" || value === null || Array.isArray(value)) continue;
      const entry = value as Record<string, unknown>;
      items[id] = {
        box: clampBox(entry.box),
        dueAtSession: int(entry.dueAtSession),
        introducedAt: int(entry.introducedAt),
        lapses: Math.max(0, int(entry.lapses)),
        reps: Math.max(0, int(entry.reps)),
      };
    }
  }

  return { version: SCHEMA_VERSION, session, items };
}

/** How many of `ids` have any saved history at all. */
export function seenCount(ids: string[], saved: SavedProgress): number {
  return ids.reduce((n, id) => (saved.items[id] ? n + 1 : n), 0);
}

// --- the only impure part -------------------------------------------------
//
// A minimal storage port. Everything above is pure; this is the thin edge that
// touches the browser, so tests can pass a fake and never need jsdom for the
// logic. Storage can throw (Safari private mode, quota, disabled cookies), so
// both directions swallow failure: not being able to save is a degraded
// experience, not a crash.

/** The slice of the Storage API we need — easy to fake in a test. */
export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export function loadProgress(storage: StorageLike | null): SavedProgress {
  if (!storage) return emptyProgress();
  try {
    return parseProgress(storage.getItem(STORAGE_KEY));
  } catch {
    return emptyProgress();
  }
}

export function saveProgress(
  storage: StorageLike | null,
  progress: SavedProgress,
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify(progress));
    return true;
  } catch {
    return false;
  }
}

/** `localStorage` when we're in a browser that allows it, else null. */
export function browserStorage(): StorageLike | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}
