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

        // ── Cell display formats ─────────────────────────────────────
        // An Excel-style code per cell (e.g. "#,##0.00", "0%", "yyyy-mm-dd")
        // decides how its value reads. setFormat with an empty code clears it.

        /** Set a cell's display format code (empty string clears it). */
        setFormat: (a1, code) => {
          const [ap, al] = writeStr(String(a1));
          const [cp, cl] = writeStr(String(code));
          ex.set_format(ap, al, cp, cl);
          freeInput(ap, al);
          freeInput(cp, cl);
        },
        /** A cell's format code, or `""` if it uses the default (General). */
        getFormat: (a1) => call1("get_format", String(a1)),
        /** A cell's value rendered through its format — the display string. */
        getDisplay: (a1) => call1("get_display", String(a1)),

        // ── Structural edits: insert / delete rows & columns ─────────
        // 1-based `at`, `count`. The engine relocates cells and rewrites every
        // formula's references; re-read via getWindow / getRaw afterwards.

        /** Insert `count` blank rows before row `at`; rows at/after slide down. */
        insertRows: (at, count) => ex.insert_rows(at >>> 0, count >>> 0),
        /** Delete `count` rows from `at`; refs to deleted rows become #REF!. */
        deleteRows: (at, count) => ex.delete_rows(at >>> 0, count >>> 0),
        /** Insert `count` blank columns before column `at`. */
        insertCols: (at, count) => ex.insert_cols(at >>> 0, count >>> 0),
        /** Delete `count` columns from `at`; refs to deleted cols become #REF!. */
        deleteCols: (at, count) => ex.delete_cols(at >>> 0, count >>> 0),

        /**
         * Drag-fill: replicate the `src` cell across the inclusive A1 rectangle
         * `dstStart`..`dstEnd`. Relative references shift per target (`=A1`
         * filled down → `=A2`), absolute (`$`) refs pin, the source's format
         * carries along, an empty source clears each target. A malformed address
         * is a no-op. Re-read via getWindow / getDisplayWindow / getRaw after.
         */
        fill: (src, dstStart, dstEnd) => {
          const [sp, sl] = writeStr(String(src));
          const [ap, al] = writeStr(String(dstStart));
          const [ep, el] = writeStr(String(dstEnd));
          ex.fill(sp, sl, ap, al, ep, el);
          freeInput(sp, sl);
          freeInput(ap, al);
          freeInput(ep, el);
        },

        /**
         * Copy the inclusive rectangle `start`..`end` into the clipboard — a
         * whole-block copy that pastes as a unit (the sibling of fill). The
         * source is untouched and the buffer survives any number of pastes.
         */
        copy: (start, end) => {
          const [sp, sl] = writeStr(String(start));
          const [ep, el] = writeStr(String(end));
          ex.copy(sp, sl, ep, el);
          freeInput(sp, sl);
          freeInput(ep, el);
        },

        /**
         * Cut the inclusive rectangle `start`..`end`. Like copy but a one-shot
         * move: the paste that places it clears the source it didn't overwrite
         * and consumes the buffer.
         */
        cut: (start, end) => {
          const [sp, sl] = writeStr(String(start));
          const [ep, el] = writeStr(String(end));
          ex.cut(sp, sl, ep, el);
          freeInput(sp, sl);
          freeInput(ep, el);
        },

        /**
         * Paste the clipboard so its top-left lands at `dstStart`. Returns `true`
         * when applied, `false` for a no-op (empty clipboard, malformed address,
         * or off-grid). Re-read via getWindow / getDisplayWindow / getRaw after.
         */
        paste: (dstStart) => {
          const [dp, dl] = writeStr(String(dstStart));
          const ok = ex.paste(dp, dl);
          freeInput(dp, dl);
          return ok === 1;
        },

        // ── Save / load (serialize) ──────────────────────────────────
        // A round-trippable JSON document of the workbook's SOURCE (formula
        // text + typed literals) and formats — not computed values, which
        // recompute on load. Persist the string, then deserialize to restore.

        /** Serialize the workbook to a self-contained JSON document string. */
        serialize: () => call0("serialize"),
        /**
         * Replace the workbook with a document from `serialize`. Returns `true`
         * on success, `false` if the data is malformed or an unsupported version
         * (the existing workbook is left untouched on failure). Re-read via
         * getWindow / getDisplayWindow / getRaw afterwards.
         */
        deserialize: (data) => {
          const [dp, dl] = writeStr(String(data));
          const ok = ex.deserialize(dp, dl);
          freeInput(dp, dl);
          return ok === 1;
        },

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
        /**
         * Dense display **strings** for the inclusive 1-based rectangle, as
         * `{ row0, col0, rows, cols, cells: string[][] }` (row-major; each cell
         * rendered through its format code, blanks as `""`), or `{ error }` on a
         * bad/oversized request. The format-aware sibling of getWindow — the one
         * read a virtualized grid needs per frame.
         */
        getDisplayWindow: (row0, col0, row1, col1) =>
          JSON.parse(callInts("get_display_window", row0, col0, row1, col1)),
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
