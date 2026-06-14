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

Console.WriteLine(failures == 0 ? "\nALL PASS" : $"\n{failures} FAILURE(S)");
return failures == 0 ? 0 : 1;
