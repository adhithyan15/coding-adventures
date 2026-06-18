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

// Same format codes as infinite.html: the cross-foot totals read with thousands
// grouping + 2 decimals, and Z1000 as a percent. Values are unchanged — this
// only affects how getDisplayWindow renders them.
const FORMATS = {
  E1: "#,##0.00", E2: "#,##0.00", E3: "#,##0.00", E4: "#,##0.00", E5: "#,##0.00",
  A5: "#,##0.00", B5: "#,##0.00", C5: "#,##0.00", D5: "#,##0.00",
  Z1000: "0.0%",
};
for (const k in FORMATS) wb.setFormat(k, FORMATS[k]);

const ROW_H = 22, COL_W = 80, OVER = 3;
const u = wb.usedRange();
const TOTAL_ROWS = Math.max(u.maxRow + 200, 1000);
const TOTAL_COLS = Math.max(u.maxCol + 30, 60);

// infinite.html's render() window computation, headless. Like the page, it now
// reads getDisplayWindow — each cell is its display STRING (value rendered
// through its format code; empty cells ""), so the renderer paints text
// directly and never re-derives number formatting.
function windowAt(st, sl, vh = 400, vw = 900) {
  const firstRow = Math.max(1, Math.floor(st / ROW_H) + 1 - OVER);
  const lastRow = Math.min(TOTAL_ROWS, Math.ceil((st + vh) / ROW_H) + OVER);
  const firstCol = Math.max(1, Math.floor(sl / COL_W) + 1 - OVER);
  const lastCol = Math.min(TOTAL_COLS, Math.ceil((sl + vw) / COL_W) + OVER);
  const win = wb.getDisplayWindow(firstRow, firstCol, lastRow, lastCol);
  const cells = (lastRow - firstRow + 1) * (lastCol - firstCol + 1);
  return { firstRow, lastRow, firstCol, lastCol, win, cells };
}
// A cell's display string at an absolute 1-based (r, c) within window w.
const valAt = (w, r, c) => w.win.cells[r - w.firstRow][c - w.firstCol];

let fail = 0;
const ok = (cond, msg) => { console.log((cond ? "ok  " : "FAIL") + "  " + msg); if (!cond) fail++; };

ok(TOTAL_ROWS >= 1000 && TOTAL_COLS >= 60, `virtual grid ${TOTAL_ROWS}x${TOTAL_COLS}`);

const top = windowAt(0, 0);
ok(top.cells < 1000, `top renders only ${top.cells} cells (bounded, not ${TOTAL_ROWS * TOTAL_COLS})`);
ok(valAt(top, 1, 1) === "15", "A1 = 15 in view (unformatted)");
// E1/E5 carry "#,##0.00" → the engine renders the formatted display string.
ok(valAt(top, 1, 5) === "38.00", "E1 = 38.00 (engine SUM, thousands+2dp format) in view");
ok(valAt(top, 5, 5) === "169.00", "E5 = 169.00 (grand total, formatted) in view");

const far = windowAt((1000 - 1) * ROW_H - 100, (26 - 1) * COL_W - 100); // row 1000, col Z
ok(far.firstRow > 900 && far.lastRow >= 1000, `scrolled to rows ${far.firstRow}..${far.lastRow}`);
ok(far.cells < 1000, `far view renders only ${far.cells} cells (same bound as the top)`);
// Z1000 = SUM(A1:A4) = 39, format "0.0%" → 39 × 100 = "3900.0%": the format
// applies identically 1000 rows off-origin.
ok(valAt(far, 1000, 26) === "3900.0%", "Z1000 = 3900.0% (=SUM(A1:A4) as percent, 1000 rows down)");

const gap = windowAt(110 * ROW_H, 0);
let allEmpty = true;
for (let r = gap.firstRow; r <= gap.lastRow; r++)
  for (let c = gap.firstCol; c <= gap.lastCol; c++)
    if (valAt(gap, r, c) !== "") allEmpty = false;
ok(allEmpty, "gap region (rows ~100-120) is entirely empty (sparse)");

ok(wb.columnLetters(27) === "AA" && wb.columnLetters(53) === "BA" && wb.columnLetters(54) === "BB",
  "column letters AA / BA / BB");
ok(wb.getDisplayWindow(50, 54, 50, 54).cells[0][0] === "78", "BB50 = 78 (=Z1000*2, unformatted)");

const rev = wb.currentRevision();
wb.setCell("A1", "115");
const d = wb.changedSince(rev);
ok(!d.stale && d.changed.includes("A1") && d.changed.includes("E1") && d.changed.includes("Z1000"),
  `edit A1 dirtied ${d.changed.length} cells incl. far Z1000: ${d.changed.join(",")}`);

// Drag-fill (the "Fill ↓ 10" button): G1 = F1*2, fill into G2:G3 — each copy's
// relative reference tracks its row (the same engine.fill the button calls).
wb.setCell("F1", "10");
wb.setCell("F2", "20");
wb.setCell("F3", "30");
wb.setCell("G1", "=F1*2"); // 20
wb.fill("G1", "G2", "G3");
const fillWin = wb.getDisplayWindow(2, 7, 3, 7); // G2:G3 (col 7 = G)
ok(fillWin.cells[0][0] === "40" && fillWin.cells[1][0] === "60",
  `fill G1 down → G2=${fillWin.cells[0][0]} (F2*2), G3=${fillWin.cells[1][0]} (F3*2)`);
ok(wb.getRaw("G3") === "=(F3*2)", `filled G3 source tracked the row: ${wb.getRaw("G3")}`);

// Clipboard (the Copy/Cut/Paste buttons): copy the block F1:G1 (F1=10,
// G1 = F1*2) and paste at F4 — the block shifts as a unit, so the paste writes
// F4=10 (the copied literal) and G4 = F4*2 = 20, the echo tracking the row.
wb.copy("F1", "G1");
const pasted = wb.paste("F4");
const pasteWin = wb.getDisplayWindow(4, 7, 4, 7); // G4 (col 7 = G)
ok(pasted === true, `paste applied (returned ${pasted})`);
ok(pasteWin.cells[0][0] === "20", `copy F1:G1 → paste at F4: G4=${pasteWin.cells[0][0]} (F4*2)`);
ok(wb.getRaw("G4") === "=(F4*2)", `pasted G4 source shifted as a unit: ${wb.getRaw("G4")}`);
// Cut moves: cut A1, paste at H1, source clears; a second paste is a no-op.
wb.setCell("A1", "99");
wb.cut("A1", "A1");
ok(wb.paste("H1") === true, "cut paste applied");
ok(wb.getDisplayWindow(1, 8, 1, 8).cells[0][0] === "99", "cut moved value to H1");
ok(wb.getDisplayWindow(1, 1, 1, 1).cells[0][0] === "", "cut cleared the source A1");
ok(wb.paste("J1") === false, "cut buffer consumed (second paste is a no-op)");

console.log(fail === 0 ? "\nALL PASS" : `\n${fail} FAILURE(S)`);
process.exit(fail ? 1 : 0);
