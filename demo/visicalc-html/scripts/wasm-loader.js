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
