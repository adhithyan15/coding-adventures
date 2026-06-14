// spreadsheet-engine-wasm.mjs
//
// The JavaScript loader for the spreadsheet engine compiled to WebAssembly.
// It instantiates the `.wasm` module (built by build-wasm.sh from the
// `spreadsheet-wasm` crate) and presents the SAME API surface as the
// TypeScript engine, so a host can swap one for the other with no other
// changes:
//
//     const engine = createEngine(wasmBytes);
//     const wb = engine.createSpreadsheet();
//     wb.setCell("B6", "=SUM(B1:B5)");
//     wb.getValue("B6");   // { kind: "number", value: 46 }
//     wb.getRaw("B6");     // "=SUM(B1:B5)"
//     wb.getValues();      // { B1: {...}, ..., B6: {...} }
//
// The loader owns the linear-memory string protocol the Rust ABI defines
// (src/lib.rs): inputs are written into module memory via `alloc(len)`;
// outputs come back as `[len: u32 LE][utf8 bytes]` buffers that we read and
// then `dealloc`. It is dependency-free and runs in both Node and the browser
// — the only thing the caller provides is the raw `.wasm` bytes (a Uint8Array
// / ArrayBuffer), so it works from `file://` (pass embedded bytes) or a server
// (pass a fetched buffer).

/**
 * Instantiate the engine from raw `.wasm` bytes.
 * @param {BufferSource} wasmBytes - the module bytes (Uint8Array/ArrayBuffer).
 * @returns {{ createSpreadsheet: () => object }}
 */
export function createEngine(wasmBytes) {
  const module = new WebAssembly.Module(wasmBytes);
  const instance = new WebAssembly.Instance(module, {});
  const ex = instance.exports;

  const enc = new TextEncoder();
  const dec = new TextDecoder();

  // Always re-derive the memory view: a call may grow linear memory, which
  // detaches any previously-created Uint8Array. Byte *offsets* (pointers)
  // stay valid across growth, so we keep the integer ptr and re-view here.
  const mem = () => new Uint8Array(ex.memory.buffer);

  // Copy a JS string into module memory; returns [ptr, len]. Empty strings
  // need no allocation — the ABI treats (0, 0) as the empty input.
  function writeStr(s) {
    const bytes = enc.encode(s);
    if (bytes.length === 0) return [0, 0];
    const ptr = ex.alloc(bytes.length);
    mem().set(bytes, ptr);
    return [ptr, bytes.length];
  }

  // Read a `[len][bytes]` result buffer the Rust side allocated, then free it.
  function readResult(ptr) {
    const m = mem();
    // length prefix is little-endian u32; `>>> 0` keeps it unsigned.
    const len =
      (m[ptr] | (m[ptr + 1] << 8) | (m[ptr + 2] << 16) | (m[ptr + 3] << 24)) >>> 0;
    const str = dec.decode(m.subarray(ptr + 4, ptr + 4 + len));
    ex.dealloc(ptr, 4 + len);
    return str;
  }

  function freeInput(ptr, len) {
    if (len) ex.dealloc(ptr, len);
  }

  function call0(fn) {
    return readResult(ex[fn]());
  }
  function call1(fn, a) {
    const [p, l] = writeStr(a);
    const r = ex[fn](p, l);
    freeInput(p, l);
    return readResult(r);
  }
  function call2(fn, a, b) {
    const [ap, al] = writeStr(a);
    const [bp, bl] = writeStr(b);
    const r = ex[fn](ap, al, bp, bl);
    freeInput(ap, al);
    freeInput(bp, bl);
    return readResult(r);
  }

  // The viewport exports take integer coordinates directly (no string inputs)
  // and return a packed `[len][bytes]` buffer — so we just forward the ints and
  // read the result. (`>>> 0` coerces each to an unsigned 32-bit int.)
  function callInts(fn, ...ints) {
    return readResult(ex[fn](...ints.map((n) => n >>> 0)));
  }

  return {
    /**
     * Start a fresh, empty workbook and return a handle whose methods mirror
     * the TypeScript engine. (The WASM module holds one global session, so
     * this resets it — one live workbook at a time, all the demos need.)
     */
    createSpreadsheet() {
      ex.reset();
      return {
        /** Set a cell from a raw string; returns `{ ok: true }` or an error. */
        setCell: (a1, raw) => JSON.parse(call2("set_cell", String(a1), String(raw))),
        /** Computed value as `{ kind, value?/code? }`. */
        getValue: (a1) => JSON.parse(call1("get_value", String(a1))),
        /** The typed source (formula or literal) for the formula bar. */
        getRaw: (a1) => call1("get_raw", String(a1)),
        /** Every set cell's computed value, keyed by A1. */
        getValues: () => JSON.parse(call0("get_values")),

        // ── Viewport primitive (virtualized infinite sheet) ──────────
        // A scrolling host renders only the visible window of an unbounded
        // sheet: getWindow for the visible rectangle, usedRange for scrollbar
        // sizing, columnLetters for the frozen header, and currentRevision +
        // changedSince to refetch only what an edit dirtied.

        /**
         * Dense computed values for the inclusive 1-based rectangle, as
         * `{ row0, col0, rows, cols, values: CellValue[][] }` (row-major,
         * blanks included as `{ kind: "empty" }`), or `{ error }` on a
         * bad/oversized request.
         */
        getWindow: (row0, col0, row1, col1) =>
          JSON.parse(callInts("get_window", row0, col0, row1, col1)),
        /** Data extent `{ minRow, minCol, maxRow, maxCol }`, or `null`. */
        usedRange: () => JSON.parse(call0("used_range")),
        /** Column letters for a 1-based index: `1 → "A"`, `27 → "AA"`. */
        columnLetters: (index) => callInts("column_letters", index),
        /** The per-edit revision clock (a Number; the ABI returns u64). */
        currentRevision: () => Number(ex.current_revision()),
        /**
         * Cells changed since a revision: `{ revision, changed: string[] }`,
         * or `{ revision, stale: true }` (re-read the whole window). `since`
         * is widened to the ABI's u64 at the boundary.
         */
        changedSince: (since) =>
          JSON.parse(readResult(ex.changed_since(BigInt(since)))),
      };
    },
  };
}
