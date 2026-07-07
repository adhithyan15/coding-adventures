// smoke-files.mjs — end-to-end check of file open/save through the compiled
// .wasm and the JS loader (the browser open/save buttons drive the exact same
// methods). Proves a real save → open round-trip over every format the engine
// exposes, including that .xlsx keeps a formula LIVE. Run after build-wasm.sh:
//
//   node js/smoke-files.mjs        (exits non-zero on any mismatch)

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createEngine } from "./spreadsheet-engine-wasm.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasm = readFileSync(join(here, "..", "pkg", "spreadsheet_engine.wasm"));
const engine = createEngine(wasm);

let failures = 0;
function check(label, cond, detail = "") {
  if (!cond) failures++;
  console.log(`${cond ? "ok  " : "FAIL"}  ${label}${cond ? "" : "  " + detail}`);
}

// Author a small workbook with a live formula, in a fresh session.
function authored() {
  const wb = engine.createSpreadsheet();
  wb.setCell("A1", "10");
  wb.setCell("A2", "20");
  wb.setCell("A3", "=SUM(A1:A2)"); // 30, live
  wb.setCell("B1", "hello");
  return wb;
}

// --- .xlsx: binary, keeps live formulas -----------------------------------
{
  const bytes = authored().saveXlsx();
  check("xlsx save → ZIP magic (PK)", bytes[0] === 0x50 && bytes[1] === 0x4b,
    `got ${bytes[0]},${bytes[1]}`);
  const wb = engine.createSpreadsheet();
  check("xlsx open returns true", wb.openXlsx(bytes) === true);
  check("xlsx A3 computed = 30",
    wb.getValue("A3").value === 30, JSON.stringify(wb.getValue("A3")));
  check("xlsx A3 stays a live formula", wb.getRaw("A3") === "=SUM(A1:A2)", wb.getRaw("A3"));
  // Live: edit a precedent, the total recomputes.
  wb.setCell("A1", "110");
  check("xlsx formula recomputes to 130", wb.getValue("A3").value === 130);
}

// --- .xls: binary (OLE2), values only -------------------------------------
{
  const bytes = authored().saveXls();
  check("xls save → OLE2 magic (D0 CF)", bytes[0] === 0xd0 && bytes[1] === 0xcf,
    `got ${bytes[0]},${bytes[1]}`);
  const wb = engine.createSpreadsheet();
  check("xls open returns true", wb.openXls(bytes) === true);
  check("xls A3 value = 30 (flattened)", wb.getValue("A3").value === 30);
  check("xls B1 text preserved", wb.getValue("B1").value === "hello");
}

// --- .csv / .tsv: positional grid -----------------------------------------
{
  const csv = authored().saveCsv();
  const text = new TextDecoder().decode(csv);
  // Row-major: A1,B1 / A2,B2 / A3,B3 — A1..A3 is a column, B1 holds "hello".
  check("csv save shape", text === "10,hello\n20,\n30,", JSON.stringify(text));
  const wb = engine.createSpreadsheet();
  check("csv open returns true", wb.openCsv(csv) === true);
  check("csv A3 = 30 numeric", wb.getValue("A3").value === 30);

  const tsv = wb.saveTsv();
  check("tsv uses tabs", new TextDecoder().decode(tsv).includes("\t"));
}

// --- .json: array-of-objects records --------------------------------------
{
  const wb = engine.createSpreadsheet();
  const doc = new TextEncoder().encode(
    '[{"region":"East","sales":200},{"region":"West","sales":340}]',
  );
  check("json open returns true", wb.openJson(doc) === true);
  check("json header A1 = region", wb.getValue("A1").value === "region");
  check("json B3 = 340", wb.getValue("B3").value === 340);
  const out = new TextDecoder().decode(wb.saveJson());
  check("json round-trips",
    out === '[{"region":"East","sales":200},{"region":"West","sales":340}]', out);
}

// --- a bad file leaves the open document untouched ------------------------
{
  const wb = engine.createSpreadsheet();
  wb.setCell("A1", "keepme");
  check("bad xlsx open returns false",
    wb.openXlsx(new Uint8Array([1, 2, 3, 4])) === false);
  check("bad json open returns false",
    wb.openJson(new TextEncoder().encode("{not json")) === false);
  check("document survived the failed opens", wb.getValue("A1").value === "keepme");
}

console.log(failures === 0 ? "\nALL FILE ROUND-TRIPS OK" : `\n${failures} FAILURE(S)`);
process.exit(failures === 0 ? 0 : 1);
