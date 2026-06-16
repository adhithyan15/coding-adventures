// smoke.mjs — end-to-end check that the compiled .wasm actually computes.
//
// Loads pkg/spreadsheet_engine.wasm through the JS loader and drives it the
// way the demo will, asserting the same results the Rust and TypeScript engines
// produce. Run after build-wasm.sh:
//
//   node js/smoke.mjs        (exits non-zero on any mismatch)

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createEngine } from "./spreadsheet-engine-wasm.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasm = readFileSync(join(here, "..", "pkg", "spreadsheet_engine.wasm"));

const engine = createEngine(wasm);
const wb = engine.createSpreadsheet();

let failures = 0;
// Canonical stringify: sort object keys so comparison is insensitive to key
// order (serde_json emits keys alphabetically; the expectations below are
// written in reading order).
function canon(x) {
  if (Array.isArray(x)) return x.map(canon);
  if (x && typeof x === "object") {
    return Object.keys(x).sort().reduce((o, k) => ((o[k] = canon(x[k])), o), {});
  }
  return x;
}
function check(label, got, want) {
  const g = JSON.stringify(canon(got));
  const ok = g === JSON.stringify(canon(want));
  if (!ok) failures++;
  console.log(
    `${ok ? "ok  " : "FAIL"}  ${label}: ${JSON.stringify(got)}` +
      (ok ? "" : ` (want ${JSON.stringify(want)})`),
  );
}

// A cross-footing budget — the same shape the demo seeds.
for (const [a, v] of [
  ["B1", "15"], ["B2", "8"], ["B3", "12"], ["B4", "4"], ["B5", "7"],
]) {
  wb.setCell(a, v);
}
wb.setCell("B6", "=SUM(B1:B5)");
wb.setCell("B7", "=AVERAGE(B1:B5)");
wb.setCell("A1", "3");
wb.setCell("A2", "5");
wb.setCell("C1", "=A1+A2*2");

check("B6 SUM", wb.getValue("B6"), { kind: "number", value: 46 });
check("B7 AVERAGE", wb.getValue("B7"), { kind: "number", value: 9.2 });
check("C1 precedence", wb.getValue("C1"), { kind: "number", value: 13 });
check("B6 raw (formula bar)", wb.getRaw("B6"), "=SUM(B1:B5)");

// Incremental recalc through the dependency graph.
wb.setCell("B1", "115");
check("B6 after B1=115", wb.getValue("B6"), { kind: "number", value: 146 });

// Error semantics propagate.
wb.setCell("D1", "=1/0");
wb.setCell("D2", "=D1+1");
check("D1 div-by-zero", wb.getValue("D1"), { kind: "error", code: "#DIV/0!" });
check("D2 error propagation", wb.getValue("D2"), { kind: "error", code: "#DIV/0!" });

// A text label, JSON-escaped safely.
wb.setCell("E1", 'a"b');
check("E1 text", wb.getValue("E1"), { kind: "text", value: 'a"b' });

// Viewport: the format-aware windowed read paints display strings. Format A1
// (=3) with a 2-decimal code and read the A1:C1 window back as strings.
wb.setFormat("A1", "#,##0.00");
check("getDisplayWindow formatted strings", wb.getDisplayWindow(1, 1, 1, 3), {
  row0: 1,
  col0: 1,
  rows: 1,
  cols: 3,
  cells: [["3.00", "115", "13"]], // A1 formatted; B1=115, C1=13 (General)
});
check("getDisplayWindow bad request", wb.getDisplayWindow(0, 0, 5, 5), {
  error: "#REF!",
});

// Fresh workbook is empty.
const wb2 = engine.createSpreadsheet();
check("reset clears values", wb2.getValues(), {});

console.log(failures === 0 ? "\nALL PASS" : `\n${failures} FAILURE(S)`);
process.exit(failures === 0 ? 0 : 1);
