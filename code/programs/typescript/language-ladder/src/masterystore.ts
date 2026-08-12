// ---------------------------------------------------------------------------
// masterystore.ts — persisting the per-atom mastery book between visits.
//
// Same shape as `reviewstore.ts`, and for the same reasons, so read that file's
// header first if this is unfamiliar. In short:
//
//   1. The engine (`atommastery.ts`) stays pure. It never reads the clock and
//      never touches storage; all of that lives here.
//   2. Everything except the reused `localStorage` adapter is pure, so the
//      round-trip is unit-testable without a browser.
//   3. The stored blob is UNTRUSTED — hand-editable, possibly half-written by
//      another tab, possibly left by an older build. Every field is validated,
//      and a bad or wrong-version payload falls back to EMPTY rather than
//      throwing. A study app that will not start because of one bad key is
//      worse than one that lost some progress.
//
// WHY A THIRD KEY. `progress.ts` persists the lesson schedule, `reviewstore.ts`
// the review quiz's Leitner cells. This persists a third, genuinely different
// thing: what the learner holds, atom by atom, independent of which lesson or
// which quiz cell happened to teach it. Sharing a blob would mean one payload
// with three unrelated schemas and three reasons to bump one version number.
// ---------------------------------------------------------------------------

import type { AtomMastery, MasteryBook } from "./atommastery.ts";
import { type StorageLike, browserStorage } from "./progress.ts";

export { browserStorage };
export type { StorageLike };

/** Bump when the saved shape changes incompatibly; older payloads are dropped. */
export const MASTERY_SCHEMA_VERSION = 1;

/** The key we own in localStorage. Namespaced so nothing else collides. */
export const MASTERY_STORAGE_KEY = "language-ladder:mastery:v1";

/**
 * What we persist.
 *
 * `atoms` is an ARRAY rather than an object map. Atom ids are well-behaved
 * (`ES-LEX-GRACIAS`), but an object map would still invite the `__proto__`
 * hazard for a hand-edited or hostile payload, and an array costs nothing.
 */
export interface SavedMastery {
  version: number;
  atoms: AtomMastery[];
}

export function emptyMastery(): SavedMastery {
  return { version: MASTERY_SCHEMA_VERSION, atoms: [] };
}

function num(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

/** Normalize one persisted atom, clamping every field to a sane range. */
function cleanAtom(value: unknown): AtomMastery | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const a = value as Record<string, unknown>;
  if (typeof a.atom !== "string" || a.atom === "") return null;
  return {
    atom: a.atom,
    introducedAt: Math.max(0, Math.trunc(num(a.introducedAt))),
    strength: Math.max(0, Math.min(1, num(a.strength))),
    lastSeen: Math.max(0, Math.trunc(num(a.lastSeen))),
    dueAt: Math.max(0, Math.trunc(num(a.dueAt))),
    lapses: Math.max(0, Math.trunc(num(a.lapses))),
  };
}

/** In-memory book → a saveable record. */
export function toSavedMastery(book: MasteryBook): SavedMastery {
  return {
    version: MASTERY_SCHEMA_VERSION,
    // Sorted so the stored blob is stable between saves: a diffable payload is
    // worth more than the microseconds the sort costs.
    atoms: [...book.values()].sort((a, b) => a.atom.localeCompare(b.atom)),
  };
}

/**
 * A saved record → an in-memory book.
 *
 * Re-validates every row, because this is a public entry point that may be
 * handed a raw parse result rather than well-formed `parseMastery` output.
 * A malformed row is dropped, not trusted and not fatal.
 */
export function fromSavedMastery(saved: SavedMastery): MasteryBook {
  const book: MasteryBook = new Map();
  const rows = Array.isArray(saved?.atoms) ? saved.atoms : [];
  for (const row of rows) {
    const atom = cleanAtom(row);
    if (atom) book.set(atom.atom, atom);
  }
  return book;
}

/** Parse an untrusted JSON string. Anything wrong yields an empty record. */
export function parseMastery(raw: string | null): SavedMastery {
  if (!raw) return emptyMastery();
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      return emptyMastery();
    }
    const payload = parsed as Record<string, unknown>;
    // A payload from a future or older schema is dropped rather than guessed at.
    if (payload.version !== MASTERY_SCHEMA_VERSION) return emptyMastery();
    return { version: MASTERY_SCHEMA_VERSION, atoms: (payload.atoms as AtomMastery[]) ?? [] };
  } catch {
    return emptyMastery();
  }
}

/** Read the book from storage. Never throws; a broken key reads as empty. */
export function loadMastery(storage: StorageLike | null): MasteryBook {
  if (!storage) return new Map();
  try {
    return fromSavedMastery(parseMastery(storage.getItem(MASTERY_STORAGE_KEY)));
  } catch {
    return new Map();
  }
}

/** Write the book to storage. Never throws; a full or blocked quota is ignored. */
export function saveMastery(storage: StorageLike | null, book: MasteryBook): void {
  if (!storage) return;
  try {
    storage.setItem(MASTERY_STORAGE_KEY, JSON.stringify(toSavedMastery(book)));
  } catch {
    // Private-mode Safari and a full quota both throw here. Losing this
    // session's mastery updates is bad; refusing to run is worse.
  }
}
