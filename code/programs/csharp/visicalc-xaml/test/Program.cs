// Program.cs — headless proof that the XAML VisiCalc demo does REAL formula work
// on the shared Rust engine, with no WinUI in the loop. It drives the same
// engine-backed SpreadsheetModel the WinUI code-behind binds to (../Engine.cs)
// and asserts the values are engine-computed and recompute on edit.
//
// Run (after scripts/build.sh has vendored native/libspreadsheet_capi.*):
//   cd code/programs/csharp/visicalc-xaml && bash scripts/verify.sh
// or directly:
//   CAPI_LIB=$PWD/native/libspreadsheet_capi.dylib dotnet run --project test

using Mosaic.Generated;

int failures = 0;

void Check(string label, string got, string want)
{
    bool ok = got == want;
    if (!ok) failures++;
    Console.WriteLine($"{(ok ? "ok  " : "FAIL")}  {label}: got=\"{got}\" want=\"{want}\"");
}

void Contains(string label, string got, string needle)
{
    bool ok = got.Contains(needle);
    if (!ok) failures++;
    Console.WriteLine($"{(ok ? "ok  " : "FAIL")}  {label}: \"{got}\" contains \"{needle}\"");
}

using var model = new SpreadsheetModel();

// Seeded cross-footing budget — computed by the engine, not hard-coded.
var rows = model.ViewportRows();
Check("E1 row total", rows[0][5], "38");   // 15+3+12+8
Check("E2 row total", rows[1][5], "51");   // 8+14+7+22
Check("A5 col total", rows[4][1], "39");   // 15+8+12+4
Check("E5 grand total", rows[4][5], "169");
Check("row-label gutter", rows[0][0], "1");

// Editing A1 (display row 0, col 1) 15 -> 115 recomputes every dependent.
model.SetCell(0, 1, "115");
rows = model.ViewportRows();
Check("A1 after edit", rows[0][1], "115");
Check("E1 after edit", rows[0][5], "138"); // 115+3+12+8
Check("A5 after edit", rows[4][1], "139"); // 115+8+12+4
Check("E5 after edit", rows[4][5], "269"); // 138+51+45+35

// A formula that divides by zero, and an error propagating through a binary op.
model.SetCell(0, 1, "=1/0");               // A1
Contains("A1 div-by-0", model.ValueJson("A1"), "#DIV/0!");
model.SetCell(0, 2, "=A1+1");              // B1
Contains("B1 propagated", model.ValueJson("B1"), "#DIV/0!");
Check("B1 display", model.ViewportRows()[0][2], "#DIV/0!");
model.Dispose();

// ── Viewport primitive (virtualized infinite sheet) ──────────────
// A fresh session seeded with the cross-foot budget + a far-flung formula at
// Z1000 (row 1000, col 26), to exercise the windowed reads.
using (var s = new SpreadsheetSession())
{
    foreach (var (a, v) in new[]
    {
        ("A1", "15"), ("B1", "3"), ("C1", "12"), ("D1", "8"), ("E1", "=SUM(A1:D1)"),
        ("A2", "8"), ("B2", "14"), ("C2", "7"), ("D2", "22"), ("E2", "=SUM(A2:D2)"),
        ("A3", "12"), ("B3", "9"), ("C3", "18"), ("D3", "6"), ("E3", "=SUM(A3:D3)"),
        ("A4", "4"), ("B4", "11"), ("C4", "3"), ("D4", "17"), ("E4", "=SUM(A4:D4)"),
        ("A5", "=SUM(A1:A4)"), ("E5", "=SUM(E1:E4)"), ("Z1000", "=SUM(A1:A4)"),
    }) s.SetCell(a, v);

    var w = s.Window(1, 1, 5, 5);
    Check("window A1", w[0][0], "15");
    Check("window E1", w[0][4], "38");
    Check("window E5", w[4][4], "169");
    Check("window Z1000", s.Window(998, 24, 1002, 28)[2][2], "39");
    Check("window gap empty", s.Window(100, 1, 110, 10)[5][5], "");

    var u = s.UsedRange()!.Value;
    Check("usedRange maxRow", u.maxRow.ToString(), "1000");
    Check("usedRange maxCol", u.maxCol.ToString(), "26");
    Check("columnLetters 27", s.ColumnLetters(27), "AA");
    Check("columnLetters 53", s.ColumnLetters(53), "BA");

    ulong rev = s.CurrentRevision();
    s.SetCell("A1", "115");
    var (changed, stale) = s.ChangedSince(rev);
    Check("changedSince not stale", stale.ToString(), "False");
    Contains("changedSince has A1", string.Join(",", changed), "A1");
    Contains("changedSince reaches Z1000", string.Join(",", changed), "Z1000");
    Check("window Z1000 after edit", s.Window(1000, 26, 1000, 26)[0][0], "139");
}

// ── Infinite-view binding layer (InfiniteSheetModel) ─────────────
// The model the WinUI InfiniteSheet view drives: one engine read per visible
// row via RowCells, tap-to-select via SelectInf (loading the cell's source into
// the formula bar), and write-through via CommitInf (recompute + regrow extent).
using (var inf = new InfiniteSheetModel())
{
    // The constructor seeds the budget PLUS far-flung cells (Z1000, BA50/BB50)
    // and computes the extent: at least 1000×60, grown to reach the far cells.
    Check("inf totalRows >= 1000", (inf.TotalRows >= 1000).ToString(), "True");
    Check("inf totalCols >= 60", (inf.TotalCols >= 60).ToString(), "True");

    // RowCells returns one row's display strings (columns 1..TotalCols).
    var row1 = inf.RowCells(1);
    Check("inf rowCells width", (row1.Count == inf.TotalCols).ToString(), "True");
    Check("inf rowCells A1", row1[0], "15"); // unformatted
    Check("inf rowCells E1", row1[4], "38.00");  // SUM(A1:D1), "#,##0.00" formatted
    Check("inf rowCells J1 empty", row1[9], ""); // sparse
    bool gapBlank = true;
    foreach (var c in inf.RowCells(200)) if (c.Length != 0) gapBlank = false;
    Check("inf gap row blank", gapBlank.ToString(), "True");

    // SelectInf loads the cell SOURCE (A5 is a formula) and clamps to the grid.
    inf.SelectInf(5, 1);
    Check("inf select A5 addr", inf.InfAddress, "A5");
    Check("inf select A5 formula", inf.Formula, "=SUM(A1:A4)");
    inf.SelectInf(-3, 0); // clamps to (1,1)
    Check("inf clamp row", inf.SelRow.ToString(), "1");
    Check("inf clamp col", inf.SelCol.ToString(), "1");

    // CommitInf writes through and recomputes every dependent.
    inf.SelectInf(2, 1);          // A2
    inf.CommitInf("108");         // 8 -> 108
    Check("inf commit A2", inf.RowCells(2)[0], "108"); // unformatted
    Check("inf commit E2", inf.RowCells(2)[4], "151.00"); // 108+14+7+22, formatted
    Check("inf commit A5", inf.RowCells(5)[0], "139.00"); // 15+108+12+4, formatted
    Check("inf commit E5", inf.RowCells(5)[4], "269.00"); // grand total, formatted

    // FillDown replicates the selected cell, shifting relative refs per target.
    // Seed a fresh column via select+commit: H1=2, H2=3, H3=4 (col 8 = H);
    // I1 = H1*10 (col 9 = I). Select I1 and fill down 10 — each filled formula
    // tracks its row (I2 = H2*10 = 30, I3 = H3*10 = 40), and I1 stays untouched.
    inf.SelectInf(1, 8); inf.CommitInf("2");        // H1
    inf.SelectInf(2, 8); inf.CommitInf("3");        // H2
    inf.SelectInf(3, 8); inf.CommitInf("4");        // H3
    inf.SelectInf(1, 9); inf.CommitInf("=H1*10");   // I1 = 20
    inf.SelectInf(1, 9);
    inf.FillDown(10);
    Check("inf fillDown I2", inf.RowCells(2)[8], "30"); // I2 = H2*10
    Check("inf fillDown I3", inf.RowCells(3)[8], "40"); // I3 = H3*10
    Check("inf fillDown I1 source", inf.RowCells(1)[8], "20"); // I1 untouched

    // Clipboard: copy I1 (= H1*10) and paste at I4 — the relative ref shifts by
    // the destination's offset, so I4 = H4*10. (Seed H4 first.)
    inf.SelectInf(4, 8); inf.CommitInf("6");        // H4 = 6
    inf.SelectInf(1, 9); inf.CopyCell();            // copy I1
    inf.SelectInf(4, 9); Check("inf pasteCell applied", inf.PasteCell().ToString(), "True");
    Check("inf paste I4 = H4*10", inf.RowCells(4)[8], "60"); // I4 = H4*10 = 60
    // Cut A1 and move it to C1: source clears, a second paste is a no-op.
    inf.SelectInf(1, 1); inf.CommitInf("99");       // A1
    inf.SelectInf(1, 1); inf.CutCell();
    inf.SelectInf(1, 3); Check("inf cut paste applied", inf.PasteCell().ToString(), "True");
    Check("inf cut moved C1", inf.RowCells(1)[2], "99"); // C1 (col 3, index 2)
    Check("inf cut cleared A1", inf.RowCells(1)[0], ""); // A1 cleared
    inf.SelectInf(1, 5); Check("inf cut buffer consumed", inf.PasteCell().ToString(), "False");
}

// Save / load (the Save / Load buttons drive SaveBook/LoadBook): serialize a
// fresh seeded workbook, mutate it, restore the snapshot, and confirm the
// loaded formula stays LIVE (the document stores source + formats, not values).
using (var sl = new InfiniteSheetModel())
{
    string snapshot = sl.SaveBook();
    Check("serialize non-empty", (snapshot.Length > 0).ToString(), "True");
    sl.SelectInf(1, 1); sl.CommitInf("500");                 // E1 → 500+3+12+8 = 523
    Check("mutated E1", sl.RowCells(1)[4], "523.00");
    Check("loadBook ok", sl.LoadBook(snapshot).ToString(), "True");
    Check("loaded A1", sl.RowCells(1)[0], "15");             // restored
    Check("loaded E1 formatted", sl.RowCells(1)[4], "38.00"); // recomputed through format
    sl.SelectInf(1, 1); sl.CommitInf("5");                   // live: 5+3+12+8 = 28
    Check("loaded formula live", sl.RowCells(1)[4], "28.00");
    Check("loadBook rejects garbage", sl.LoadBook("not a workbook").ToString(), "False");
    Check("workbook intact after reject", sl.RowCells(1)[4], "28.00");
}

// Undo / redo (the Undo / Redo buttons drive UndoEdit/RedoEdit): a fresh,
// unseeded session so the initial history is empty. Two edits, walk back and
// forward, and confirm a restored formula recomputes live.
using (var ur = new SpreadsheetSession())
{
    Check("fresh canUndo false", ur.CanUndo().ToString(), "False");
    ur.SetCell("A1", "1");
    ur.SetCell("B1", "=A1*10"); // 10
    Check("after edits canUndo true", ur.CanUndo().ToString(), "True");
    Check("undo formula", ur.Undo().ToString(), "True");
    Check("B1 cleared by undo", ur.Window(1, 2, 1, 2)[0][0], "");
    Check("undo literal", ur.Undo().ToString(), "True");
    Check("A1 cleared by undo", ur.Window(1, 1, 1, 1)[0][0], "");
    Check("canUndo false at bottom", ur.CanUndo().ToString(), "False");
    Check("undo at bottom is noop", ur.Undo().ToString(), "False");
    Check("redo literal", ur.Redo().ToString(), "True");
    Check("redo formula", ur.Redo().ToString(), "True");
    Check("B1 live after redo", ur.Window(1, 2, 1, 2)[0][0], "10");
    Check("canRedo false at top", ur.CanRedo().ToString(), "False");
    // A fresh edit forks history (drops the redo branch).
    ur.Undo(); // back: B1 gone
    Check("canRedo true before fork", ur.CanRedo().ToString(), "True");
    ur.SetCell("C1", "9");
    Check("fresh edit clears redo", ur.CanRedo().ToString(), "False");
}

// ── Structural edits (the + Row / − Row / + Col / − Col buttons drive
// InsertRows/DeleteRows/InsertCols/DeleteCols): inserting and deleting
// rows/columns shifts every formula reference across the band, and deleting a
// referenced band turns that reference into #REF!. The engine parenthesizes
// binary ops on re-emit ("=(A1+A3)"), so compare with parens stripped.
static string Bare(string s) => s.Replace("(", "").Replace(")", "");
using (var st = new SpreadsheetSession())
{
    st.SetCell("A1", "10"); st.SetCell("A2", "20"); st.SetCell("A3", "=A1+A2"); // 30
    Check("struct A3 before", st.Window(3, 1, 3, 1)[0][0], "30");
    st.InsertRows(2, 1);
    Check("struct inserted row blank", st.Window(2, 1, 2, 1)[0][0], "");
    Check("struct formula at A4", st.Window(4, 1, 4, 1)[0][0], "30");
    Check("struct insert shifted refs", Bare(st.GetRaw("A4")), "=A1+A3");
    st.DeleteRows(2, 1);
    Check("struct delete shifted back", Bare(st.GetRaw("A3")), "=A1+A2");
    st.DeleteRows(1, 1); // delete the referenced row 1 → A1 ref destroyed
    Check("struct deleted ref is #REF!", st.Window(2, 1, 2, 1)[0][0], "#REF!");
}
using (var sc = new SpreadsheetSession())
{
    sc.SetCell("K1", "5"); sc.SetCell("L1", "=K1*3");
    sc.InsertCols(11, 1); // col 11 = K → formula moves to M1, refs shift
    Check("struct insertCol value", sc.Window(1, 13, 1, 13)[0][0], "15"); // M1
    Check("struct insertCol shifted refs", Bare(sc.GetRaw("M1")), "=L1*3");
}

// ── Number formatting (the .00 / % / $ / Gen buttons drive SetFormat): applying
// a format code changes only how the cell DISPLAYS; the stored value is
// unchanged. An empty code clears the format back to General.
using (var fmt = new SpreadsheetSession())
{
    fmt.SetCell("A1", "1234");
    string Disp() => fmt.Window(1, 1, 1, 1)[0][0];
    Check("fmt unformatted", Disp(), "1234");
    fmt.SetFormat("A1", "#,##0.00");
    Check("fmt #,##0.00", Disp(), "1,234.00");
    fmt.SetFormat("A1", "0.0%");
    Check("fmt 0.0%", Disp(), "123400.0%");
    fmt.SetFormat("A1", "$#,##0.00");
    Check("fmt $", Disp(), "$1,234.00");
    fmt.SetFormat("A1", "");
    Check("fmt cleared", Disp(), "1234");
    Check("fmt raw untouched", fmt.GetRaw("A1"), "1234"); // display-only
}

// ── Range sort (the ▲/▼ Sort buttons drive SortRange): reorder the budget block
// A1:E4 by a key column. Each row moves as a record — the E-column SUM formulas
// travel with their row (the engine shifts the refs), so every total stays
// correct after the reorder.
using (var so = new SpreadsheetSession())
{
    foreach (var (a, v) in new[]
    {
        ("A1", "15"), ("B1", "3"), ("C1", "12"), ("D1", "8"), ("E1", "=SUM(A1:D1)"),
        ("A2", "8"), ("B2", "14"), ("C2", "7"), ("D2", "22"), ("E2", "=SUM(A2:D2)"),
        ("A3", "12"), ("B3", "9"), ("C3", "18"), ("D3", "6"), ("E3", "=SUM(A3:D3)"),
        ("A4", "4"), ("B4", "11"), ("C4", "3"), ("D4", "17"), ("E4", "=SUM(A4:D4)"),
    }) so.SetCell(a, v);
    Check("sort pre A1", so.Window(1, 1, 1, 1)[0][0], "15");
    Check("sort applied asc", so.SortRange("A1", "E4", 1, true).ToString(), "True");
    Check("sort A1 asc", so.Window(1, 1, 1, 1)[0][0], "4");    // col A → 4,8,12,15
    Check("sort A4 asc", so.Window(4, 1, 4, 1)[0][0], "15");
    Check("sort E1 asc", so.Window(1, 5, 1, 5)[0][0], "35");   // E tracks row: 4+11+3+17
    Check("sort E4 asc", so.Window(4, 5, 4, 5)[0][0], "38");   // 15+3+12+8
    Check("sort applied desc", so.SortRange("A1", "E4", 1, false).ToString(), "True");
    Check("sort A1 desc", so.Window(1, 1, 1, 1)[0][0], "15");
    Check("sort single-row no-op", so.SortRange("A1", "A1", 1, true).ToString(), "False");
    Check("sort bad key no-op", so.SortRange("A1", "E4", 9, true).ToString(), "False");
}

// ── Find / replace (the find/replace boxes + Find/Replace buttons drive
// FindAll/ReplaceAll): FindAll returns the A1 addresses whose SOURCE contains
// the query (case-insensitive); ReplaceAll rewrites the query in every cell's
// source and recomputes, returning the count. A rewritten formula stays live;
// a rewritten literal stays typed.
using (var fr = new InfiniteSheetModel())
{
    // The seed has the literal "15" only at A1, and "=SUM(" in every total formula.
    Check("find literal 15", string.Join(",", fr.FindAll("15")), "A1");
    Contains("find formula SUM has E1", string.Join(",", fr.FindAll("sum")), "E1");
    Check("find empty query", fr.FindAll("").Count.ToString(), "0");
    Check("find no match", fr.FindAll("zzz").Count.ToString(), "0");
    // SelectA1 moves the cursor onto a hit (parsing column letters past Z).
    fr.SelectA1("Z1000");
    Check("selectA1 Z1000 addr", fr.InfAddress, "Z1000");
    // Replace a literal: A1 "15" → "99"; E1 = 99+3+12+8 = 122 (#,##0.00 format).
    Check("replace 15->99 count", fr.ReplaceAll("15", "99").ToString(), "1");
    fr.SelectA1("A1");
    Check("replaced A1 value", fr.RowCells(1)[0], "99");
    Check("replaced E1 recomputed", fr.RowCells(1)[4], "122.00");
    // Replace inside a formula reference keeps it LIVE: H1=10, H2=20, H3 = =H1+5
    // (15). Rewrite "H1" → "H2" → H3 becomes =H2+5 = 25, recomputed by the engine.
    fr.SelectInf(1, 8); fr.CommitInf("10");     // H1
    fr.SelectInf(2, 8); fr.CommitInf("20");     // H2
    fr.SelectInf(3, 8); fr.CommitInf("=H1+5");  // H3 = 15
    Check("pre-replace H3", fr.RowCells(3)[7], "15");
    Check("replace H1->H2 count", fr.ReplaceAll("H1", "H2").ToString(), "1");
    Check("H3 recomputed live", fr.RowCells(3)[7], "25"); // =H2+5
}

// ── Multi-sheet workbook + cross-sheet references (the sheet tab bar drives
// SheetNames/ActiveSheet/SelectSheet/AddSheet/RenameSheet/DeleteSheet): the
// workbook holds several sheets; bare-A1 ops address the ACTIVE sheet, while a
// formula reaches ACROSS with a qualifier (=Summary!B3). This proves the .NET ↔
// P/Invoke ↔ C ABI path drives sheet management + cross-sheet recompute.
using (var ms = new SpreadsheetSession())
{
    Check("ms initial sheets", string.Join(",", ms.SheetNames().names), "Sheet1");
    Check("ms add Summary", ms.AddSheet("Summary").ToString(), "True");
    Check("ms sheets after add", string.Join(",", ms.SheetNames().names), "Sheet1,Summary");
    // Edit the Summary sheet (index 1): B3 = A1+A2 = 300.
    Check("ms activate Summary", ms.SetActiveSheet(1).ToString(), "True");
    Check("ms active index", ms.ActiveSheet().ToString(), "1");
    ms.SetCell("A1", "100"); ms.SetCell("A2", "200"); ms.SetCell("B3", "=A1+A2");
    Check("ms Summary B3", ms.Window(3, 2, 3, 2)[0][0], "300");
    // Back on Sheet1, reach ACROSS with a qualifier.
    Check("ms back to Sheet1", ms.SetActiveSheet(0).ToString(), "True");
    ms.SetCell("G1", "=Summary!B3");
    Check("ms cross-sheet G1", ms.Window(1, 7, 1, 7)[0][0], "300");
    // Editing a Summary input recomputes the cross-sheet dependent live.
    ms.SetActiveSheet(1); ms.SetCell("A1", "150"); // Summary!A1: 100 → 150, B3 = 350
    ms.SetActiveSheet(0);
    Check("ms cross-sheet live", ms.Window(1, 7, 1, 7)[0][0], "350");
    // Rename Summary → Totals; the qualifier in G1 is rewritten by the engine.
    Check("ms rename", ms.RenameSheet(1, "Totals").ToString(), "True");
    Check("ms sheets after rename", string.Join(",", ms.SheetNames().names), "Sheet1,Totals");
    Contains("ms G1 names Totals", ms.GetRaw("G1"), "Totals");
    Check("ms cross-sheet after rename", ms.Window(1, 7, 1, 7)[0][0], "350");
    // Delete the referenced sheet → the dangling cross-sheet ref becomes #REF!.
    Check("ms delete", ms.DeleteSheet(1).ToString(), "True");
    Check("ms sheets after delete", string.Join(",", ms.SheetNames().names), "Sheet1");
    Contains("ms G1 #REF!", ms.Window(1, 7, 1, 7)[0][0], "#REF!");
    // The engine keeps at least one sheet: deleting the last one is a no-op.
    Check("ms cannot delete last", ms.DeleteSheet(0).ToString(), "False");
}

// The InfiniteSheetModel seed exposes Sheet1/Summary with a live cross-ref.
using (var msm = new InfiniteSheetModel())
{
    Check("msm sheets", string.Join(",", msm.SheetNames()), "Sheet1,Summary");
    Check("msm active", msm.ActiveSheet().ToString(), "0");
    msm.SelectA1("G1");
    Check("msm Sheet1 G1 cross-ref", msm.RowCells(1)[6], "300"); // col 7, index 6
    msm.SelectSheet(1);
    Check("msm switched to Summary", msm.ActiveSheet().ToString(), "1");
    Check("msm Summary B3", msm.RowCells(3)[1], "300"); // B3 = A1+A2 = 100+200
}

// ── File open / save (the File → Save/Open buttons drive ExportBytes/
// ImportBytes over sc_save_*/sc_load_*): open and save a REAL spreadsheet file
// over the engine's byte codecs. File bytes are binary (a .xlsx is a ZIP, an
// .xls an OLE2 file) and cross P/Invoke as a (byte[], len) pair going in and a
// copied-out heap buffer coming back — never a NUL-terminated string a 0x00
// inside the file would truncate.
using (var src = new SpreadsheetSession())
{
    src.SetCell("A1", "15"); src.SetCell("B1", "3"); src.SetCell("C1", "=A1+B1"); // 18

    // .xlsx is a real ZIP (magic "PK\x03\x04") and keeps its live formula.
    byte[] xlsx = src.SaveXlsx();
    Check("xlsx ZIP magic",
        (xlsx.Length > 4 && xlsx[0] == 0x50 && xlsx[1] == 0x4B && xlsx[2] == 0x03 && xlsx[3] == 0x04).ToString(),
        "True");
    using (var dst = new SpreadsheetSession())
    {
        Check("xlsx reopens", dst.LoadXlsx(xlsx).ToString(), "True");
        Check("xlsx keeps formula", dst.GetRaw("C1"), "=A1+B1");
        Check("xlsx computes", dst.Display("C1"), "18");
    }

    // .xls is a real OLE2 file (magic D0 CF 11 E0) — the 0xD0 high bit is exactly
    // what a lossy string round-trip would have mangled.
    byte[] xls = src.SaveXls();
    Check("xls OLE2 magic",
        (xls.Length > 8 && xls[0] == 0xD0 && xls[1] == 0xCF && xls[2] == 0x11 && xls[3] == 0xE0).ToString(),
        "True");
    using (var dst = new SpreadsheetSession())
    {
        Check("xls reopens", dst.LoadXls(xls).ToString(), "True");
        Check("xls value", dst.Display("C1"), "18");
    }

    // A bad or empty payload is rejected (no exception), workbook left untouched.
    Check("xlsx rejects garbage",
        src.LoadXlsx(System.Text.Encoding.UTF8.GetBytes("not a spreadsheet")).ToString(), "False");
    Check("xlsx rejects empty", src.LoadXlsx(Array.Empty<byte>()).ToString(), "False");
    Check("workbook intact after reject", src.Display("C1"), "18");
}

// CSV / TSV / JSON are values-only tabular codecs. JSON's canonical shape is an
// array of objects, so row 1 is the HEADER (the keys) and row 2 the first data
// record; CSV/TSV are positional grids. A header row + one data row round-trips
// consistently through all three.
foreach (var format in new[] { "csv", "tsv", "json" })
{
    byte[] bytes;
    using (var t = new SpreadsheetSession())
    {
        t.SetCell("A1", "qty"); t.SetCell("B1", "unit"); t.SetCell("C1", "total");
        t.SetCell("A2", "15"); t.SetCell("B2", "3"); t.SetCell("C2", "=A2*B2"); // 45
        bytes = format switch { "csv" => t.SaveCsv(), "tsv" => t.SaveTsv(), _ => t.SaveJson() };
    }
    Check($"{format} save non-empty", (bytes.Length > 0).ToString(), "True");
    using (var t2 = new SpreadsheetSession())
    {
        bool ok = format switch { "csv" => t2.LoadCsv(bytes), "tsv" => t2.LoadTsv(bytes), _ => t2.LoadJson(bytes) };
        Check($"{format} reopens", ok.ToString(), "True");
        Check($"{format} header round-trip", t2.Display("A1"), "qty");
        Check($"{format} value round-trip", t2.Display("C2"), "45");
    }
}

// The InfiniteSheetModel exposes format-parameterised ExportBytes/ImportBytes.
using (var fm = new InfiniteSheetModel())
{
    foreach (var format in InfiniteSheetModel.FileFormats)
    {
        byte[] bytes = fm.ExportBytes(format);
        Check($"model {format} export", (bytes.Length > 0).ToString(), "True");
        Check($"model {format} import", fm.ImportBytes(format, bytes).ToString(), "True");
    }
    Check("model unknown export empty", (fm.ExportBytes("numbers").Length == 0).ToString(), "True");
    Check("model unknown import false", fm.ImportBytes("numbers", new byte[] { 1, 2, 3 }).ToString(), "False");
}

Console.WriteLine(failures == 0 ? "\nALL PASS" : $"\n{failures} FAILURE(S)");
return failures == 0 ? 0 : 1;
