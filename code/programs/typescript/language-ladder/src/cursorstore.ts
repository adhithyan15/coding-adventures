// ---------------------------------------------------------------------------
// cursorstore.ts — remembering WHERE you are in the walk, between visits.
//
// The Learn session walks the concept spine, and the review quiz already
// remembers your SRS state (reviewstore.ts) and the lesson schedule remembers
// its own (progress.ts). But the teaching cursor — which concept you had walked
// to — reset to 0 on every reload. Walk to concept 40, close the tab, come back,
// and you were dumped at "thanks" again. This gives the cursor a memory too, so
// the app resumes exactly where you left off.
//
// Deliberately tiny: the persisted state is a single integer index. Everything
// except the reused localStorage adapter is pure, and — like its siblings — the
// stored blob is UNTRUSTED, so a corrupt, wrong-version, or out-of-range value
// falls back to 0 (the start) rather than throwing or pointing off the end of
// the spine.
// ---------------------------------------------------------------------------

import { type StorageLike, browserStorage } from "./progress.ts";

export { browserStorage };
export type { StorageLike };

/** Bump when the saved shape changes incompatibly; older payloads are dropped. */
export const CURSOR_SCHEMA_VERSION = 1;

/** The key we own in localStorage. Namespaced so nothing else collides. */
export const CURSOR_STORAGE_KEY = "language-ladder:cursor:v1";

/** What we persist — just the concept index, wrapped with a version. */
export interface SavedCursor {
  version: number;
  index: number;
}

/** Clamp an index into the valid range for a spine of `length` concepts. */
export function clampCursor(index: number, length: number): number {
  if (!Number.isFinite(index) || length <= 0) return 0;
  return Math.max(0, Math.min(Math.trunc(index), length - 1));
}

/**
 * Parse whatever was in storage into a raw (unclamped) non-negative index.
 *
 * Untrusted input: non-JSON, wrong version, wrong shape, or a negative /
 * non-finite index all yield 0. The caller clamps the result to the current
 * spine length (which can change as the curriculum grows), so parsing and
 * bounding stay separate concerns.
 */
export function parseCursor(raw: string | null): number {
  if (raw === null || raw === "") return 0;

  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return 0;
  }
  if (typeof data !== "object" || data === null || Array.isArray(data)) return 0;

  const record = data as Record<string, unknown>;
  if (record.version !== CURSOR_SCHEMA_VERSION) return 0;

  const index = record.index;
  if (typeof index !== "number" || !Number.isFinite(index) || index < 0) return 0;
  return Math.trunc(index);
}

// --- the only impure part (delegated to progress.ts's port) ----------------

/** Load the saved concept cursor, clamped to the current spine `length`. */
export function loadCursor(storage: StorageLike | null, length: number): number {
  if (!storage) return 0;
  let raw: string | null;
  try {
    raw = storage.getItem(CURSOR_STORAGE_KEY);
  } catch {
    return 0;
  }
  return clampCursor(parseCursor(raw), length);
}

/** Persist the concept cursor. Silent on failure (Safari private mode, quota). */
export function saveCursor(storage: StorageLike | null, index: number): boolean {
  if (!storage) return false;
  try {
    const payload: SavedCursor = {
      version: CURSOR_SCHEMA_VERSION,
      index: Math.max(0, Math.trunc(Number.isFinite(index) ? index : 0)),
    };
    storage.setItem(CURSOR_STORAGE_KEY, JSON.stringify(payload));
    return true;
  } catch {
    return false;
  }
}
