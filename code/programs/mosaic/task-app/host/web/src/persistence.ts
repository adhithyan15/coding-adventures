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
import {
  IndexedDBStorage,
  MemoryStorage,
  type KVStorage,
} from "@coding-adventures/indexeddb/src/browser.js";

/** The single object store; one record in it holds the entire workspace. */
export const WORKSPACE_STORE = "workspace";

/** The key of the (currently only) workspace record. */
export const WORKSPACE_KEY = "web";

/** Fixed recovery key used to preserve the most recently rejected web record. */
export const RECOVERY_WORKSPACE_KEY = "web-corrupt";

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
  /**
   * The project the user was last looking at. Host state by design: the engine keeps
   * this cursor out of its snapshot so two hosts on the same data can sit on different
   * projects, which makes remembering it the host's job. Optional — records written
   * before projects existed simply don't have it, and load falls back to the default.
   */
  activeProject?: string;
  /** When this record was written (ms epoch) — for debugging / future conflict resolution. */
  savedAt: number;
}

/** What the UI needs to say truthfully about the selected backend. */
export interface WorkspaceStorageSession {
  storage: KVStorage;
  durable: boolean;
  status: string;
  location: string;
  warning: string;
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
export async function openWorkspaceStorage(): Promise<WorkspaceStorageSession> {
  try {
    const idb = new IndexedDBStorage({ dbName: "task-app", version: 1, stores: SCHEMA });
    await idb.open();
    return {
      storage: idb,
      durable: true,
      status: "Saved locally on this device",
      location: "This browser profile · IndexedDB database task-app, workspace record web",
      warning: "",
    };
  } catch {
    const mem = new MemoryStorage(SCHEMA);
    await mem.open();
    return {
      storage: mem,
      durable: false,
      status: "Temporary session only",
      location: "Memory in this tab · closing or reloading removes these changes",
      warning:
        "Durable local storage is unavailable. Changes will be lost when this tab closes or reloads.",
    };
  }
}

/** Read the persisted workspace, or `undefined` on a first visit. */
export function loadWorkspace(storage: KVStorage): Promise<WorkspaceRecord | undefined> {
  return storage.get<WorkspaceRecord>(WORKSPACE_STORE, WORKSPACE_KEY);
}

/** Preserve a rejected record before normal saves can replace the live key. */
export function preserveRejectedWorkspace(
  storage: KVStorage,
  record: WorkspaceRecord,
): Promise<void> {
  return storage.put(WORKSPACE_STORE, {
    ...record,
    id: RECOVERY_WORKSPACE_KEY,
  });
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
  activeProject?: string,
): WorkspaceRecord {
  return {
    id: WORKSPACE_KEY,
    snapshot,
    order: [...order],
    counter,
    savedAt: now,
    ...(activeProject === undefined ? {} : { activeProject }),
  };
}

/**
 * Persist a record, **fire-and-forget** (like the checklist-app middleware):
 * the UI stays responsive because we don't await the write. A dropped write
 * costs at most the latest edit on a crash — an acceptable trade for a task
 * app. (A banking app would await and surface errors.)
 */
export function saveWorkspace(
  storage: KVStorage,
  record: WorkspaceRecord,
  onError?: (message: string) => void,
): void {
  void storage.put(WORKSPACE_STORE, record).catch((error: unknown) => {
    const detail = error instanceof Error ? error.message : String(error);
    onError?.(`Could not save changes to local storage: ${detail}`);
  });
}
