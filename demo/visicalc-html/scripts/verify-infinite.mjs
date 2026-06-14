// verify-infinite.mjs — headless proof that the virtualized infinite-sheet demo
// (infinite.html) does REAL windowed rendering on the shared Rust engine.
//
// It loads the SAME committed bundle the browser loads
// (vendor/spreadsheet-engine-wasm.js — the base64-embedded .wasm + loader),
// seeds the SAME data infinite.html seeds, and replays infinite.html's exact
// visible-window computation at several scroll positions — asserting that:
//   - the virtual grid is huge but each render is bounded to the visible window
//     (only a few hundred cells, never the millions the sheet spans),
//   - a formula 1000 rows down is reachable with the same bounded render,
//   - the gap between data islands is empty (the sheet is sparse),
//   - column letters run past Z (AA, BA, BB), and
//   - an edit's changedSince diff reaches the far cell that depends on it.
//
// This is the web analog of the native demos' headless tests. It needs no
// browser — the windowing math is deterministic; only real-pixel layout (which
// a browser does) is out of scope, and render() is a direct transcription of
// these computations into DOM nodes.
//
// Run:  node demo/visicalc-html/scripts/verify-infinite.mjs

import { readFileSync } from "node:fs";
import vm from "node:vm";

// Load the committed vendored bundle in a sandbox that provides the few globals
// the loader expects (the same ones a browser has).
const bundle = readFileSync(
  new URL("../vendor/spreadsheet-engine-wasm.js", import.meta.url),
  "utf8",
);
const sandbox = { window: {}, WebAssembly, atob, TextEncoder, TextDecoder, console };
vm.createContext(sandbox);
vm.runInContext(bundle, sandbox);

await sandbox.window.__spreadsheetEngineReady;
const wb = sandbox.window.SpreadsheetEngine.createSpreadsheet();

// Same seed + geometry as infinite.html.
const SEED = {
  A1: "15", B1: "3", C1: "12", D1: "8", E1: "=SUM(A1:D1)",
  A2: "8", B2: "14", C2: "7", D2: "22", E2: "=SUM(A2:D2)",
  A3: "12", B3: "9", C3: "18", D3: "6", E3: "=SUM(A3:D3)",
  A4: "4", B4: "11", C4: "3", D4: "17", E4: "=SUM(A4:D4)",
  A5: "=SUM(A1:A4)", B5: "=SUM(B1:B4)", C5: "=SUM(C1:C4)", D5: "=SUM(D1:D4)", E5: "=SUM(E1:E4)",
  Z1000: "=SUM(A1:A4)", BA50: "far cell", BB50: "=Z1000*2",
};
for (const k in SEED) wb.setCell(k, SEED[k]);

const ROW_H = 22, COL_W = 80, OVER = 3;
const u = wb.usedRange();
const TOTAL_ROWS = Math.max(u.maxRow + 200, 1000);
const TOTAL_COLS = Math.max(u.maxCol + 30, 60);

// infinite.html's render() window computation, headless.
function windowAt(st, sl, vh = 400, vw = 900) {
  const firstRow = Math.max(1, Math.floor(st / ROW_H) + 1 - OVER);
  const lastRow = Math.min(TOTAL_ROWS, Math.ceil((st + vh) / ROW_H) + OVER);
  const firstCol = Math.max(1, Math.floor(sl / COL_W) + 1 - OVER);
  const lastCol = Math.min(TOTAL_COLS, Math.ceil((sl + vw) / COL_W) + OVER);
  const win = wb.getWindow(firstRow, firstCol, lastRow, lastCol);
  const cells = (lastRow - firstRow + 1) * (lastCol - firstCol + 1);
  return { firstRow, lastRow, firstCol, lastCol, win, cells };
}
const disp = (v) =>
  v.kind === "number" ? String(v.value)
  : v.kind === "text" ? v.value
  : v.kind === "error" ? v.code : "";
const valAt = (w, r, c) => w.win.values[r - w.firstRow][c - w.firstCol];

let fail = 0;
const ok = (cond, msg) => { console.log((cond ? "ok  " : "FAIL") + "  " + msg); if (!cond) fail++; };

ok(TOTAL_ROWS >= 1000 && TOTAL_COLS >= 60, `virtual grid ${TOTAL_ROWS}x${TOTAL_COLS}`);

const top = windowAt(0, 0);
ok(top.cells < 1000, `top renders only ${top.cells} cells (bounded, not ${TOTAL_ROWS * TOTAL_COLS})`);
ok(disp(valAt(top, 1, 1)) === "15", "A1 = 15 in view");
ok(disp(valAt(top, 1, 5)) === "38", "E1 = 38 (engine SUM) in view");
ok(disp(valAt(top, 5, 5)) === "169", "E5 = 169 (grand total) in view");

const far = windowAt((1000 - 1) * ROW_H - 100, (26 - 1) * COL_W - 100); // row 1000, col Z
ok(far.firstRow > 900 && far.lastRow >= 1000, `scrolled to rows ${far.firstRow}..${far.lastRow}`);
ok(far.cells < 1000, `far view renders only ${far.cells} cells (same bound as the top)`);
ok(disp(valAt(far, 1000, 26)) === "39", "Z1000 = 39 (=SUM(A1:A4), 1000 rows down)");

const gap = windowAt(110 * ROW_H, 0);
let allEmpty = true;
for (let r = gap.firstRow; r <= gap.lastRow; r++)
  for (let c = gap.firstCol; c <= gap.lastCol; c++)
    if (valAt(gap, r, c).kind !== "empty") allEmpty = false;
ok(allEmpty, "gap region (rows ~100-120) is entirely empty (sparse)");

ok(wb.columnLetters(27) === "AA" && wb.columnLetters(53) === "BA" && wb.columnLetters(54) === "BB",
  "column letters AA / BA / BB");
ok(disp(wb.getWindow(50, 54, 50, 54).values[0][0]) === "78", "BB50 = 78 (=Z1000*2)");

const rev = wb.currentRevision();
wb.setCell("A1", "115");
const d = wb.changedSince(rev);
ok(!d.stale && d.changed.includes("A1") && d.changed.includes("E1") && d.changed.includes("Z1000"),
  `edit A1 dirtied ${d.changed.length} cells incl. far Z1000: ${d.changed.join(",")}`);

console.log(fail === 0 ? "\nALL PASS" : `\n${fail} FAILURE(S)`);
process.exit(fail ? 1 : 0);
