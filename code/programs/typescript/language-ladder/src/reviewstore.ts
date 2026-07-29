// ---------------------------------------------------------------------------
// reviewstore.ts — remembering the Learn-mode review quiz, between visits.
//
// THE PROBLEM. The review quiz (sessionplan.ts `applyAnswer`) keeps a `Progress`
// — per-cell Leitner state plus the answer log — in memory only, so a reload
// wiped every promotion, demotion, and logged confusion. The SRS could adapt
// within a session but forgot you the moment you left. This module gives it a
// memory, the same way `progress.ts` did for the lesson schedule.
//
// THE DESIGN, borrowed wholesale from progress.ts:
//   1. The engine (quiz.ts / sessionplan.ts / mistakes.ts) stays pure. Nothing
//      there knows about storage; the (de)serialization lives entirely here.
//   2. Everything except the reused `localStorage` adapter is pure, so the
//      round-trip is unit-testable without a browser.
//   3. The stored blob is UNTRUSTED (hand-editable, half-written by another tab,
//      left by an older build). Every field is validated and a bad or
//      wrong-version payload falls back to EMPTY rather than throwing — a study
//      app that won't start because of one bad key is worse than lost progress.
//
// WHY A NEW KEY AND MODULE rather than extending progress.ts: that file persists
// the *lesson schedule* (id-keyed, positional scheduler state). The review quiz
// persists a *different* thing — cellKey-keyed `QuizState` plus an answer log —
// so it gets its own key and its own shape. Sharing the code would mean one
// blob with two unrelated schemas.
// ---------------------------------------------------------------------------

import { MAX_BOX } from "./scheduler.ts";
import type { QuizState } from "./quiz.ts";
import type { AnswerRecord } from "./mistakes.ts";
import type { Progress } from "./sessionplan.ts";
// Reuse the exact storage port progress.ts already proved out — no second copy.
import { type StorageLike, browserStorage } from "./progress.ts";

export { browserStorage };
export type { StorageLike };

/** Bump when the saved shape changes incompatibly; older payloads are dropped. */
export const REVIEW_SCHEMA_VERSION = 1;

/** The key we own in localStorage. Namespaced so nothing else collides. */
export const REVIEW_STORAGE_KEY = "language-ladder:review:v1";

/**
 * What we persist. Deliberately flat and boring.
 *
 * `states` is an ARRAY of [cellKey, QuizState] pairs, not an object: a cellKey is
 * a JSON string (`["CONCEPT","language","lesson-id"]`) and could contain any
 * character, so an entries array sidesteps both key-escaping and the `__proto__`
 * hazard that an object map would invite.
 */
export interface SavedReview {
  version: number;
  /** The review SRS clock — how many questions have been answered. */
  session: number;
  states: Array<[string, QuizState]>;
  log: AnswerRecord[];
}

/** A fresh, empty saved record. */
export function emptyReview(): SavedReview {
  return { version: REVIEW_SCHEMA_VERSION, session: 0, states: [], log: [] };
}

/** A fresh, empty in-memory Progress + session (what `fromSavedReview` returns on nothing). */
export function emptyRestored(): { progress: Progress; session: number } {
  return { progress: { states: new Map(), log: [] }, session: 0 };
}

function int(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? Math.trunc(value) : 0;
}

function clampBox(box: unknown): number {
  return Math.max(0, Math.min(MAX_BOX, int(box)));
}

/** Normalize one persisted cell state, clamping every field to a sane range. */
function cleanState(value: unknown): QuizState | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const s = value as Record<string, unknown>;
  return {
    box: clampBox(s.box),
    dueAtSession: int(s.dueAtSession),
    lapses: Math.max(0, int(s.lapses)),
    reps: Math.max(0, int(s.reps)),
  };
}

/** In-memory Progress + session → a saveable record. */
export function toSavedReview(progress: Progress, session: number): SavedReview {
  return {
    version: REVIEW_SCHEMA_VERSION,
    session: Math.max(0, int(session)),
    states: [...progress.states.entries()],
    log: progress.log,
  };
}

/**
 * A saved record → in-memory Progress + session.
 *
 * Every entry is re-validated (the caller may hand us the raw parse result), so
 * a malformed state or log row is dropped rather than trusted.
 */
export function fromSavedReview(saved: SavedReview): { progress: Progress; session: number } {
  const states = new Map<string, QuizState>();
  // Guard the shape too — this is a public entry point that may be handed a raw,
  // untrusted object directly (not only well-formed `parseReview` output).
  const savedStates = Array.isArray(saved.states) ? saved.states : [];
  for (const pair of savedStates) {
    if (!Array.isArray(pair) || pair.length !== 2) continue;
    const [key, rawState] = pair;
    if (typeof key !== "string") continue;
    const state = cleanState(rawState);
    if (state) states.set(key, state);
  }

  const log: AnswerRecord[] = [];
  const savedLog = Array.isArray(saved.log) ? saved.log : [];
  for (const row of savedLog) {
    if (typeof row !== "object" || row === null || Array.isArray(row)) continue;
    const r = row as Record<string, unknown>;
    if (typeof r.cellKey !== "string" || typeof r.correct !== "boolean") continue;
    const record: AnswerRecord = { cellKey: r.cellKey, correct: r.correct };
    // chosenKey is only meaningful on a miss; keep it only when it's a string.
    if (!r.correct && typeof r.chosenKey === "string") record.chosenKey = r.chosenKey;
    log.push(record);
  }

  return { progress: { states, log }, session: Math.max(0, int(saved.session)) };
}

/**
 * Parse whatever was in storage, defensively. Untrusted input in, valid
 * `SavedReview` out — a wrong version, non-JSON, or wrong-shaped blob yields
 * `emptyReview()` rather than throwing.
 */
export function parseReview(raw: string | null): SavedReview {
  if (raw === null || raw === "") return emptyReview();

  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return emptyReview();
  }
  if (typeof data !== "object" || data === null || Array.isArray(data)) return emptyReview();

  const record = data as Record<string, unknown>;
  // Version gate: an older/newer incompatible shape is dropped, not migrated.
  if (record.version !== REVIEW_SCHEMA_VERSION) return emptyReview();

  const session = Math.max(0, int(record.session));

  const states: Array<[string, QuizState]> = [];
  if (Array.isArray(record.states)) {
    for (const pair of record.states) {
      if (!Array.isArray(pair) || pair.length !== 2) continue;
      const [key, rawState] = pair;
      if (typeof key !== "string") continue;
      const state = cleanState(rawState);
      if (state) states.push([key, state]);
    }
  }

  const log: AnswerRecord[] = [];
  if (Array.isArray(record.log)) {
    for (const row of record.log) {
      if (typeof row !== "object" || row === null || Array.isArray(row)) continue;
      const r = row as Record<string, unknown>;
      if (typeof r.cellKey !== "string" || typeof r.correct !== "boolean") continue;
      const clean: AnswerRecord = { cellKey: r.cellKey, correct: r.correct };
      if (!r.correct && typeof r.chosenKey === "string") clean.chosenKey = r.chosenKey;
      log.push(clean);
    }
  }

  return { version: REVIEW_SCHEMA_VERSION, session, states, log };
}

// --- the only impure part (delegated to progress.ts's port) ----------------

/** Load the saved review, restored into an in-memory Progress + session. */
export function loadReview(storage: StorageLike | null): { progress: Progress; session: number } {
  if (!storage) return emptyRestored();
  let raw: string | null;
  try {
    raw = storage.getItem(REVIEW_STORAGE_KEY);
  } catch {
    return emptyRestored();
  }
  return fromSavedReview(parseReview(raw));
}

/** Persist the review Progress + session. Silent on failure (Safari private mode, quota). */
export function saveReview(
  storage: StorageLike | null,
  progress: Progress,
  session: number,
): boolean {
  if (!storage) return false;
  try {
    storage.setItem(REVIEW_STORAGE_KEY, JSON.stringify(toSavedReview(progress, session)));
    return true;
  } catch {
    return false;
  }
}
