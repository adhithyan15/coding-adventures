/**
 * persistence.ts — pluggable, host-owned persistence for the task-app web host.
 *
 * The engine (`task-core`, reached over `task-wasm`) is **pure**: it never
 * touches storage, a clock, or the network. Persistence is therefore the
 * *host's* job — and this module is that seam.
 *
 * It reuses the repo's canonical **pluggable storage interface**
 * (`@coding-adventures/storage`, implemented for the browser by
 * `@coding-adventures/indexeddb`'s `IndexedDBStorage`). Because everything goes
 * through that one `KVStorage` contract, swapping IndexedDB for SQLite, a sync
 * server, or cloud storage later is a one-line change *here* — never a change
 * to the engine or the UI. That is the whole point of the "flexible storage
 * layer" in `code/specs/task-app-super-app.md`.
 *
 * === What we persist ===
 *
 * One **whole-workspace snapshot per record**. The engine already serializes
 * its entire `ProjectState` to JSON via `snapshot()` / restores it via
 * `load()`; we store that blob next to the small bit of *host* state the engine
 * does not own:
 *
 *   - `order`   — the row display order (task ids in creation order), and
 *   - `counter` — the id high-water mark, so ids minted after a reload never
 *                 collide with ids already in the loaded project.
 *
 * so that a reload restores the exact session. A per-entity decomposition
 * (one record per task/resource/…) is a later optimization that buys nothing
 * until writes get large — see task-app-super-app.md §9.
 *
 *   engine.snapshot()  ──►  { id, snapshot, order, counter } ──►  storage.put
 *   storage.get        ──►  { … , snapshot }                 ──►  engine.load
 */
import { IndexedDBStorage, MemoryStorage } from "@coding-adventures/indexeddb";
import type { KVStorage } from "@coding-adventures/indexeddb";

/** The single object store; one record in it holds the entire workspace. */
export const WORKSPACE_STORE = "workspace";

/** The key of the (currently only) workspace record. */
export const WORKSPACE_KEY = "web";

/** The persisted shape: the engine snapshot plus the host-owned session state. */
export interface WorkspaceRecord {
  /** Primary key (the store's keyPath) — always {@link WORKSPACE_KEY}. */
  id: string;
  /** `engine.snapshot()` — the whole `ProjectState` as JSON. */
  snapshot: string;
  /** Row display order (task ids, creation order) — host state, not in the engine. */
  order: string[];
  /** Id sequence high-water mark, so new ids never collide with loaded ones. */
  counter: number;
  /** When this record was written (ms epoch) — for debugging / future conflict resolution. */
  savedAt: number;
}

/** The store schema, shared by both backends so their key handling matches. */
const SCHEMA = [{ name: WORKSPACE_STORE, keyPath: "id" }];

/**
 * Open the browser storage backend, falling back to an in-memory store when
 * IndexedDB is unavailable (private browsing, SSR, Node/test runners). Both
 * implement the same `KVStorage` contract, so callers never branch on which one
 * they got — the app simply loses persistence-across-reload on the fallback.
 *
 * (The checklist-app boots exactly this way; we copy the proven pattern.)
 */
export async function openWorkspaceStorage(): Promise<KVStorage> {
  try {
    const idb = new IndexedDBStorage({ dbName: "task-app", version: 1, stores: SCHEMA });
    await idb.open();
    return idb;
  } catch {
    const mem = new MemoryStorage(SCHEMA);
    await mem.open();
    return mem;
  }
}

/** Read the persisted workspace, or `undefined` on a first visit. */
export function loadWorkspace(storage: KVStorage): Promise<WorkspaceRecord | undefined> {
  return storage.get<WorkspaceRecord>(WORKSPACE_STORE, WORKSPACE_KEY);
}

/**
 * Build a record from the current engine snapshot + host state.
 *
 * `now` is passed in (rather than read from `Date.now()` inside) so this stays
 * a pure function — trivially testable, and honouring the engine's own
 * "inject the clock, don't read it" discipline at the host layer too. The
 * `order` array is copied so the record can't alias the controller's live list.
 */
export function makeWorkspaceRecord(
  snapshot: string,
  order: readonly string[],
  counter: number,
  now: number,
): WorkspaceRecord {
  return { id: WORKSPACE_KEY, snapshot, order: [...order], counter, savedAt: now };
}

/**
 * Persist a record, **fire-and-forget** (like the checklist-app middleware):
 * the UI stays responsive because we don't await the write. A dropped write
 * costs at most the latest edit on a crash — an acceptable trade for a task
 * app. (A banking app would await and surface errors.)
 */
export function saveWorkspace(storage: KVStorage, record: WorkspaceRecord): void {
  void storage.put(WORKSPACE_STORE, record);
}
