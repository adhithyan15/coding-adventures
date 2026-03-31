/**
 * memory-storage.ts — Re-export from @coding-adventures/storage.
 *
 * The canonical MemoryStorage implementation now lives in the storage
 * package. This file re-exports it so existing imports from
 * @coding-adventures/indexeddb continue to work.
 *
 * The storage package's MemoryStorage implements the full Storage
 * interface (CRUD + query + transactions). It is a superset of the
 * old KVStorage-only MemoryStorage that used to live here.
 */

export { MemoryStorage } from "@coding-adventures/storage";
