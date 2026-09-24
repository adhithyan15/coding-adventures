// Dependency-free JavaScript accessor for the journal-wasm ABI.
//
// One method per export of the pure `journal-core` engine. There is no dispatch
// bus and no props/events facade: a host keeps this engine object in its own state
// and calls methods directly, re-rendering on change. Every method returns the
// parsed JSON envelope — `{ ok: true }`, `{ ok: true, data }`, or
// `{ ok: false, error, code }` — except `snapshot()`, which returns a raw string.
//
//   const engine = createJournalEngine(wasmBytes);
//   engine.init({ journalId, name: "Personal", nowMs: Date.now() });
//   engine.apply({ type: "createEntry", id, journal: journalId,
//                  date: "2026-09-23", title: "Hi", body: "…" });
//   engine.timeline();                                   // → { ok: true, data: [...] }
//   engine.search("lighthouse");                         // → ranked hits
//   const json = engine.snapshot();  engine.load(json);  // persistence
//
// Everything that comes back is plain text. Escape titles, bodies, snippets, and
// tags before putting them into HTML.

export function createJournalEngine(wasmBytes, options = {}) {
  const module =
    wasmBytes instanceof WebAssembly.Module ? wasmBytes : new WebAssembly.Module(wasmBytes);
  const instance = new WebAssembly.Instance(module, options.importObject ?? { env: {} });
  const ex = instance.exports;
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  const mem = () => new Uint8Array(ex.memory.buffer);
  const now = options.now ?? (() => Date.now());

  function writeStr(value) {
    const bytes = enc.encode(String(value));
    if (bytes.length === 0) return [0, 0];
    const ptr = ex.alloc(bytes.length);
    if (ptr === 0) throw new Error("journal-wasm alloc returned null");
    mem().set(bytes, ptr);
    return [ptr, bytes.length];
  }

  function readResult(ptr) {
    if (ptr === 0) throw new Error("journal-wasm returned null");
    const m = mem();
    const len = (m[ptr] | (m[ptr + 1] << 8) | (m[ptr + 2] << 16) | (m[ptr + 3] << 24)) >>> 0;
    const value = dec.decode(m.subarray(ptr + 4, ptr + 4 + len));
    ex.dealloc(ptr, 4 + len);
    return value;
  }

  // Call an export taking a `(ptr,len)` string; returns the raw result string.
  function callRaw(name, str) {
    const [ptr, len] = writeStr(str);
    const out = ex[name](ptr, len);
    if (len) ex.dealloc(ptr, len);
    return readResult(out);
  }
  const call = (name, payload) => JSON.parse(callRaw(name, JSON.stringify(payload ?? {})));

  return {
    // ── lifecycle ──
    /** Start fresh with one journal: `{ journalId, name, nowMs? }`. */
    init: (args) => call("init", { nowMs: now(), ...args }),
    /** Replace the state with a snapshot string — refused whole unless it validates. */
    load: (json) => JSON.parse(callRaw("load", json)),
    /** The whole state as a JSON string (`"null"` before init/load). */
    snapshot: () => readResult(ex.snapshot()),
    reset: () => ex.reset(),

    // ── commands ──
    /** Apply one journal-core command, e.g. `{ type: "setStarred", id, starred: true }`. */
    apply: (command, nowMs = now()) => call("apply", { command, nowMs }),
    /** Import the TypeScript Journal's stored `Entry[]` into `journal`. */
    importLegacy: (journal, entries) => call("import_legacy", { journal, entries }),

    // ── queries ──
    journals: () => JSON.parse(readResult(ex.journals())),
    entry: (id) => call("entry", { id }),
    timeline: (filter = {}) => call("timeline", filter),
    onThisDay: (today, filter = {}) => call("on_this_day", { today, filter }),
    search: (query, filter = {}) => call("search", { query, filter }),
    tagCounts: (filter = {}) => call("tag_counts", filter),
    monthActivity: (year, month, filter = {}) => call("month_activity", { year, month, filter }),
  };
}
