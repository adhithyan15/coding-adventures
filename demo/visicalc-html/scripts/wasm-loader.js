// wasm-loader.js — browser loader for the Rust spreadsheet engine compiled to
// WebAssembly. This is the SOURCE; scripts/bundle-wasm-engine.sh prepends the
// `.wasm` bytes (as `var __SPREADSHEET_WASM_B64__ = "..."`) to produce the
// committed vendor/spreadsheet-engine-wasm.js the page loads.
//
// It presents the SAME global API as the TypeScript engine bundle
// (`window.SpreadsheetEngine` with `createSpreadsheet()` + `columnToLetters`),
// so index.html's glue is unchanged except that it awaits
// `window.__spreadsheetEngineReady` — WASM compilation of a >4 KB module must
// be asynchronous on the browser main thread.
//
// Dependency-free, and the bytes are embedded (not fetched), so the page still
// opens directly from disk via file://.
(function () {
  function b64ToBytes(b64) {
    const bin = atob(b64);
    const n = bin.length;
    const bytes = new Uint8Array(n);
    for (let i = 0; i < n; i++) bytes[i] = bin.charCodeAt(i);
    return bytes;
  }

  // 0-based bijective base-26: 0 -> "A", 25 -> "Z", 26 -> "AA". Matches the
  // TypeScript engine's `columnToLetters`, which the demo calls as
  // `columnToLetters(displayCol - 1)`.
  function columnToLetters(index) {
    let n = index + 1;
    let s = "";
    while (n > 0) {
      const r = (n - 1) % 26;
      s = String.fromCharCode(65 + r) + s;
      n = Math.floor((n - 1) / 26);
    }
    return s;
  }

  // Wrap a WASM instance in the engine API. Owns the linear-memory string
  // protocol defined by the `spreadsheet-wasm` crate: inputs written via
  // alloc(len); outputs returned as [len: u32 LE][utf8] buffers we read then
  // dealloc. `mem()` is re-derived each use because a call may grow memory and
  // detach the previous view.
  function makeEngine(instance) {
    const ex = instance.exports;
    const enc = new TextEncoder();
    const dec = new TextDecoder();
    const mem = () => new Uint8Array(ex.memory.buffer);

    function writeStr(s) {
      const bytes = enc.encode(s);
      if (bytes.length === 0) return [0, 0];
      const ptr = ex.alloc(bytes.length);
      mem().set(bytes, ptr);
      return [ptr, bytes.length];
    }
    function readResult(ptr) {
      const m = mem();
      const len =
        (m[ptr] | (m[ptr + 1] << 8) | (m[ptr + 2] << 16) | (m[ptr + 3] << 24)) >>> 0;
      const s = dec.decode(m.subarray(ptr + 4, ptr + 4 + len));
      ex.dealloc(ptr, 4 + len);
      return s;
    }
    function freeInput(ptr, len) {
      if (len) ex.dealloc(ptr, len);
    }
    const call0 = (fn) => readResult(ex[fn]());
    const call1 = (fn, a) => {
      const [p, l] = writeStr(a);
      const r = ex[fn](p, l);
      freeInput(p, l);
      return readResult(r);
    };
    const call2 = (fn, a, b) => {
      const [ap, al] = writeStr(a);
      const [bp, bl] = writeStr(b);
      const r = ex[fn](ap, al, bp, bl);
      freeInput(ap, al);
      freeInput(bp, bl);
      return readResult(r);
    };
    // The viewport exports take integer coordinates directly (no strings) and
    // return a packed result; `>>> 0` coerces each arg to an unsigned 32-bit int.
    const callInts = (fn, ...ints) => readResult(ex[fn](...ints.map((n) => n >>> 0)));

    return {
      createSpreadsheet() {
        ex.reset();
        return {
          setCell: (a1, raw) => JSON.parse(call2("set_cell", String(a1), String(raw))),
          setCells: (obj) => {
            for (const k in obj) call2("set_cell", k, String(obj[k]));
          },
          getValue: (a1) => JSON.parse(call1("get_value", String(a1))),
          getRaw: (a1) => call1("get_raw", String(a1)),
          getValues: () => JSON.parse(call0("get_values")),

          // Cell display formats — an Excel-style code per cell (e.g.
          // "#,##0.00", "0%", "yyyy-mm-dd") decides how its value reads.
          // setFormat with an empty code clears it.
          setFormat: (a1, code) => {
            const [ap, al] = writeStr(String(a1));
            const [cp, cl] = writeStr(String(code));
            ex.set_format(ap, al, cp, cl);
            freeInput(ap, al);
            freeInput(cp, cl);
          },
          getFormat: (a1) => call1("get_format", String(a1)),
          getDisplay: (a1) => call1("get_display", String(a1)),

          // Drag-fill: replicate the `src` cell across the inclusive A1
          // rectangle `dstStart`..`dstEnd`. Relative refs shift per target,
          // absolute ($) refs pin, the source's format carries along, an empty
          // source clears each target. Re-read via getDisplayWindow afterwards.
          fill: (src, dstStart, dstEnd) => {
            const [sp, sl] = writeStr(String(src));
            const [ap, al] = writeStr(String(dstStart));
            const [ep, el] = writeStr(String(dstEnd));
            ex.fill(sp, sl, ap, al, ep, el);
            freeInput(sp, sl);
            freeInput(ap, al);
            freeInput(ep, el);
          },

          // Clipboard: copy/cut capture the inclusive rectangle start..end (a
          // whole-block copy that pastes as a unit); paste places the block so
          // its top-left lands at dstStart, shifting the block's refs by the
          // destination's offset. paste returns true when applied, false for a
          // no-op (empty clipboard / malformed address / off-grid). A cut is a
          // one-shot move whose paste clears the source it didn't overwrite.
          copy: (start, end) => {
            const [sp, sl] = writeStr(String(start));
            const [ep, el] = writeStr(String(end));
            ex.copy(sp, sl, ep, el);
            freeInput(sp, sl);
            freeInput(ep, el);
          },
          cut: (start, end) => {
            const [sp, sl] = writeStr(String(start));
            const [ep, el] = writeStr(String(end));
            ex.cut(sp, sl, ep, el);
            freeInput(sp, sl);
            freeInput(ep, el);
          },
          paste: (dstStart) => {
            const [dp, dl] = writeStr(String(dstStart));
            const ok = ex.paste(dp, dl);
            freeInput(dp, dl);
            return ok === 1;
          },

          // Structural edits: insert / delete `count` rows or columns at a
          // 1-based position. The engine shifts every formula reference at or
          // after the band (a ref inside a deleted band becomes #REF!), then
          // recomputes. Re-read the window afterwards.
          insertRows: (at, count) => ex.insert_rows(at >>> 0, count >>> 0),
          deleteRows: (at, count) => ex.delete_rows(at >>> 0, count >>> 0),
          insertCols: (at, count) => ex.insert_cols(at >>> 0, count >>> 0),
          deleteCols: (at, count) => ex.delete_cols(at >>> 0, count >>> 0),

          // Sort the rows of the inclusive rectangle start..end by the computed
          // values in keyCol (a 1-based absolute column index inside the
          // rectangle). ascending defaults to true. Each row moves as a record;
          // moved formulas shift their relative refs with the row, formats ride
          // along. Returns true when applied (or already sorted), false for a
          // malformed address / out-of-range keyCol / empty-single-row /
          // oversized range. Re-read via getDisplayWindow afterwards.
          sortRange: (start, end, keyCol, ascending = true) => {
            const [sp, sl] = writeStr(String(start));
            const [ep, el] = writeStr(String(end));
            const ok = ex.sort_range(sp, sl, ep, el, keyCol >>> 0, ascending ? 1 : 0);
            freeInput(sp, sl);
            freeInput(ep, el);
            return ok === 1;
          },

          // Find / replace. findAll returns the A1 addresses whose text contains
          // query (inFormulas=true searches each cell's source, else its computed
          // display value; matchCase=false folds ASCII case) as a JS array, parsed
          // from the engine's {"matches":[...]} JSON. replaceAll replaces query
          // with replacement in matching cells' source (engine rewrites +
          // recomputes) and returns the count changed. Empty query → [] / 0.
          findAll: (query, inFormulas = false, matchCase = true) => {
            const [qp, ql] = writeStr(String(query));
            const json = readResult(ex.find_all(qp, ql, inFormulas ? 1 : 0, matchCase ? 1 : 0));
            freeInput(qp, ql);
            return JSON.parse(json).matches;
          },
          replaceAll: (query, replacement, matchCase = true) => {
            const [qp, ql] = writeStr(String(query));
            const [rp, rl] = writeStr(String(replacement));
            const n = ex.replace_all(qp, ql, rp, rl, matchCase ? 1 : 0);
            freeInput(qp, ql);
            freeInput(rp, rl);
            return n >>> 0;
          },

          // Multi-sheet workbook: bare-A1 cell ops address the ACTIVE sheet; a
          // formula may reference another (=Summary!A1). sheetNames returns
          // {sheets:[...], active:idx}; the mutators return true/false. Re-read
          // the grid (and selection) after a sheet op.
          sheetNames: () => JSON.parse(call0("sheet_names")),
          activeSheet: () => ex.active_sheet() >>> 0,
          setActiveSheet: (index) => ex.set_active_sheet(index >>> 0) === 1,
          addSheet: (name) => {
            const [np, nl] = writeStr(String(name));
            const ok = ex.add_sheet(np, nl);
            freeInput(np, nl);
            return ok === 1;
          },
          renameSheet: (index, newName) => {
            const [np, nl] = writeStr(String(newName));
            const ok = ex.rename_sheet(index >>> 0, np, nl);
            freeInput(np, nl);
            return ok === 1;
          },
          deleteSheet: (index) => ex.delete_sheet(index >>> 0) === 1,
          moveSheet: (index, toIndex) => ex.move_sheet(index >>> 0, toIndex >>> 0) === 1,

          // Save / load: serialize the workbook's SOURCE (formula text + typed
          // literals) + formats to a JSON string, and restore from one. Computed
          // values recompute on load, so a loaded formula stays live.
          serialize: () => call0("serialize"),
          deserialize: (data) => {
            const [dp, dl] = writeStr(String(data));
            const ok = ex.deserialize(dp, dl);
            freeInput(dp, dl);
            return ok === 1;
          },

          // Undo / redo: snapshot-based history. Each undo/redo returns true if it
          // changed the document; canUndo/canRedo gate the buttons. Re-read via
          // getWindow / getDisplayWindow / getRaw after a true undo/redo.
          undo: () => ex.undo() === 1,
          redo: () => ex.redo() === 1,
          canUndo: () => ex.can_undo() === 1,
          canRedo: () => ex.can_redo() === 1,

          // Column widths & row heights on the ACTIVE sheet (presentation chrome
          // the engine stores but never computes with). A 1-based column / row
          // index; a number size in host units. columnWidth/rowHeight return 0 when
          // unset (use the host default); the setters return true if applied, false
          // if rejected (non-finite / <= 0 / index 0). columnWidths/rowHeights return
          // the customized sizes in a range, [{col,w}] / [{row,h}], for a one-call
          // viewport fetch. Sizes persist through serialize/load and shift on insert.
          columnWidth: (col) => ex.column_width(col >>> 0),
          rowHeight: (row) => ex.row_height(row >>> 0),
          setColumnWidth: (col, width) => ex.set_column_width(col >>> 0, +width) === 1,
          setRowHeight: (row, height) => ex.set_row_height(row >>> 0, +height) === 1,
          clearColumnWidth: (col) => ex.clear_column_width(col >>> 0) === 1,
          clearRowHeight: (row) => ex.clear_row_height(row >>> 0) === 1,
          columnWidths: (col0, col1) => JSON.parse(callInts("column_widths", col0, col1)),
          rowHeights: (row0, row1) => JSON.parse(callInts("row_heights", row0, row1)),

          // Viewport primitive — render only the visible window of an unbounded
          // sheet. 1-based inclusive coords.
          getWindow: (row0, col0, row1, col1) =>
            JSON.parse(callInts("get_window", row0, col0, row1, col1)),
          // Like getWindow, but each cell is its display string (value rendered
          // through its format code; empty cells ""). The one read a virtualized
          // grid needs per frame — the host paints the strings directly.
          getDisplayWindow: (row0, col0, row1, col1) =>
            JSON.parse(callInts("get_display_window", row0, col0, row1, col1)),
          usedRange: () => JSON.parse(call0("used_range")),
          columnLetters: (index) => callInts("column_letters", index),
          currentRevision: () => Number(ex.current_revision()),
          changedSince: (since) =>
            JSON.parse(readResult(ex.changed_since(BigInt(since)))),
        };
      },
    };
  }

  // eslint-disable-next-line no-undef -- injected by the bundler.
  window.__spreadsheetEngineReady = WebAssembly.instantiate(
    b64ToBytes(__SPREADSHEET_WASM_B64__),
    {},
  ).then(({ instance }) => {
    const engine = makeEngine(instance);
    window.SpreadsheetEngine = {
      createSpreadsheet: () => engine.createSpreadsheet(),
      columnToLetters,
    };
    return window.SpreadsheetEngine;
  });
})();
