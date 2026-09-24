// persistence.ts — keeping the journal between visits (J5c-2).
//
// The runtime owns the state; the host only files away what `snapshot()`
// returns and hands it back to `restore()` on the next visit. A snapshot's
// `bytes` are the runtime's own JSON, so they are stored as TEXT (a JSON array
// of byte values would be about four times the size, against localStorage's
// few-megabyte budget).
//
// Nothing here ever deletes a journal. A stored value the runtime refuses is
// moved aside under a key of its own (UNREADABLE_KEY + "." + a time), and
// the journal starts empty; a second refusal never overwrites the first.
import type { MosaicSnapshot } from "../../../../../../packages/rust/mosaic-app-wasm/js/mosaic-host.mjs";

export const STATE_KEY = "journal-mosaic/state";
export const UNREADABLE_KEY = "journal-mosaic/state.unreadable";

interface StoredSnapshot {
  schema: string;
  version: number;
  text: string;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

/** The storage to use, or null where there is none (some private modes throw). */
export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** The stored snapshot, or null when there is none or it is not ours. */
export function readStored(storage: Storage | null): { snapshot: MosaicSnapshot | null; raw: string | null } {
  const raw = storage?.getItem(STATE_KEY) ?? null;
  if (raw === null) return { snapshot: null, raw: null };
  try {
    const stored = JSON.parse(raw) as StoredSnapshot;
    if (typeof stored.schema !== "string" || typeof stored.version !== "number" || typeof stored.text !== "string") {
      return { snapshot: null, raw };
    }
    return { snapshot: { schema: stored.schema, version: stored.version, bytes: Array.from(encoder.encode(stored.text)) }, raw };
  } catch {
    return { snapshot: null, raw };
  }
}

/** Every value kept aside so far, oldest first. */
export function keptAside(storage: Storage | null): string[] {
  if (!storage) return [];
  const keys: string[] = [];
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    if (key?.startsWith(`${UNREADABLE_KEY}.`)) keys.push(key);
  }
  return keys.sort().map(key => storage.getItem(key) ?? "");
}

/**
 * Keep a value the runtime could not read, instead of losing it. Each one gets
 * its own key, so an earlier one is never overwritten; the same value is not
 * kept twice. The live key is cleared only after the copy is written, so a
 * full storage (setItem throws) loses nothing.
 */
export function setAside(storage: Storage | null, raw: string, now: number = Date.now()): void {
  if (!storage) return;
  if (!keptAside(storage).includes(raw)) {
    let key = `${UNREADABLE_KEY}.${String(now).padStart(16, "0")}`;
    for (let n = 1; storage.getItem(key) !== null; n++) key = `${UNREADABLE_KEY}.${String(now).padStart(16, "0")}-${n}`;
    storage.setItem(key, raw);
  }
  storage.removeItem(STATE_KEY);
}

/** Store a snapshot. Throws when the browser refuses (quota, private mode). */
export function writeSnapshot(storage: Storage, snapshot: MosaicSnapshot): void {
  const text = decoder.decode(Uint8Array.from(snapshot.bytes));
  const stored: StoredSnapshot = { schema: snapshot.schema, version: snapshot.version, text };
  storage.setItem(STATE_KEY, JSON.stringify(stored));
}
