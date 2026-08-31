/**
 * types.ts — Type re-exports and backward compatibility aliases.
 *
 * All schema types and the full Storage interface now live in
 * @coding-adventures/storage. This file re-exports them so existing
 * consumers of @coding-adventures/indexeddb don't break.
 *
 * The old KVStorage interface (CRUD only, no query/transaction) is kept
 * as a backward-compatible alias. New code should use Storage instead.
 */

// ── Re-exports from @coding-adventures/storage ──────────────────────────────
//
// These are the canonical definitions. Everything below is re-exported
// unchanged so that `import { ... } from "@coding-adventures/indexeddb"`
// continues to work.

export type {
  Storage,
  StorageRecord,
  StorageConfig,
  StoreSchema,
  IndexSchema,
  QueryResult,
  SqlValue,
} from "@coding-adventures/storage";

// ── Backward-compatible KVStorage alias ─────────────────────────────────────
//
// KVStorage was the original interface in this package — CRUD + open/close,
// no query() or transaction(). Apps that import KVStorage keep working.
// Once all consumers migrate to Storage, this alias can be removed.
//
// The browser-safe contract intentionally omits query() and transaction().
// IndexedDBStorage implements KVStorage today; it will implement the full
// Storage interface when query/transaction support is added.

export type { KVStorage } from "./browser-types.js";
