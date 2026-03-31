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
// We define it as a standalone interface (not a type alias for Storage)
// because KVStorage intentionally omits query() and transaction().
// IndexedDBStorage implements KVStorage today; it will implement the full
// Storage interface when query/transaction support is added.

export interface KVStorage {
  open(): Promise<void>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  get<T = any>(storeName: string, key: string): Promise<T | undefined>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  getAll<T = any>(storeName: string): Promise<T[]>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  put(storeName: string, record: any): Promise<void>;
  delete(storeName: string, key: string): Promise<void>;
  close(): void;
}
