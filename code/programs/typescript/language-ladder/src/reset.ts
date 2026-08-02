// ---------------------------------------------------------------------------
// reset.ts — starting over.
//
// The app now remembers a lot between visits: the review quiz's SRS state and
// answer log (reviewstore.ts), the teaching cursor (cursorstore.ts), and the
// lesson schedule (progress.ts). That is good — until you want a clean slate
// (you're handing the tab to someone else, or you want to re-walk from the top).
// There was no way to clear it. This is that way.
//
// The rule that keeps this safe: clear ONLY the keys this app owns. We import
// the canonical key constants from the modules that own them rather than
// re-typing the strings, so the list can never drift out of sync with what is
// actually written. Everything here is pure over a tiny storage port, so "did we
// clear every owned key?" is unit-testable without a browser.
// ---------------------------------------------------------------------------

import { STORAGE_KEY as LESSON_SCHEDULE_KEY } from "./progress.ts";
import { REVIEW_STORAGE_KEY } from "./reviewstore.ts";
import { CURSOR_STORAGE_KEY } from "./cursorstore.ts";
import { LANGUAGE_STORAGE_KEY } from "./languagestore.ts";

/**
 * Every localStorage key this app owns. Sourced from the owning modules'
 * exported constants so it stays in lockstep with what is written — add a new
 * persisted key there and it is removed here for free (once added to this list).
 */
export const OWNED_STORAGE_KEYS: readonly string[] = [
  REVIEW_STORAGE_KEY,
  CURSOR_STORAGE_KEY,
  LESSON_SCHEDULE_KEY,
  LANGUAGE_STORAGE_KEY,
];

/** The slice of the Storage API a reset needs — just removal. Easy to fake. */
export interface RemovableStorage {
  removeItem(key: string): void;
}

/**
 * Remove every owned key. Silent per-key on failure (a locked/So-full storage
 * shouldn't turn "start over" into a crash), and a null storage is a no-op.
 */
export function clearProgress(storage: RemovableStorage | null): void {
  if (!storage) return;
  for (const key of OWNED_STORAGE_KEYS) {
    try {
      storage.removeItem(key);
    } catch {
      /* keep going — clearing the rest still helps */
    }
  }
}

/** `localStorage` (which supports removeItem) when available, else null. */
export function removableStorage(): RemovableStorage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}
