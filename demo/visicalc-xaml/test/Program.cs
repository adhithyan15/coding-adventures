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

Console.WriteLine(failures == 0 ? "\nALL PASS" : $"\n{failures} FAILURE(S)");
return failures == 0 ? 0 : 1;
