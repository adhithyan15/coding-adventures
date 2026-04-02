/**
 * types.ts — All TypeScript interfaces for the Journal app.
 *
 * The data model is intentionally simple: a flat list of journal entries.
 * Each entry has a title, a body written in GitHub Flavored Markdown, and
 * two timestamps.
 *
 * === Date vs timestamp ===
 *
 * createdAt is an ISO 8601 date STRING ("YYYY-MM-DD"), not a Unix timestamp.
 * A journal entry belongs to a calendar date — "I wrote this on March 15th"
 * means the same thing in every timezone. Date strings also enable simple
 * comparison and grouping: "2026-03-15" < "2026-04-02".
 *
 * updatedAt is a Unix timestamp (milliseconds) because edit times care about
 * precision — knowing whether two edits happened seconds apart matters for
 * future conflict resolution in sync backends.
 */

// ── Entry ──────────────────────────────────────────────────────────────────

/**
 * Entry — one journal entry.
 *
 * Multiple entries can exist on the same calendar date. The content field
 * holds raw GitHub Flavored Markdown that is parsed and rendered on demand
 * via the @coding-adventures/gfm pipeline.
 */
export interface Entry {
  id: string;         // UUID, generated on creation
  title: string;      // Short heading for the entry
  content: string;    // Raw GFM markdown body
  createdAt: string;  // ISO 8601 date: "YYYY-MM-DD"
  updatedAt: number;  // Unix timestamp (ms), set on every save
}

// ── Application State ──────────────────────────────────────────────────────

/**
 * AppState — the complete application state managed by the Flux store.
 *
 * The entries array is loaded from storage on startup and written back via
 * persistence middleware on every mutation.
 */
export interface AppState {
  entries: Entry[];
}
