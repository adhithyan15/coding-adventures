// Program.cs — headless proof that the XAML VisiCalc demo does REAL formula work
// on the shared Rust engine, with no WinUI in the loop. It drives the same
// engine-backed SpreadsheetModel the WinUI code-behind binds to (../Engine.cs)
// and asserts the values are engine-computed and recompute on edit.
//
// Run (after scripts/build.sh has vendored native/libspreadsheet_capi.*):
//   cd demo/visicalc-xaml && bash scripts/verify.sh
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

Console.WriteLine(failures == 0 ? "\nALL PASS" : $"\n{failures} FAILURE(S)");
return failures == 0 ? 0 : 1;
