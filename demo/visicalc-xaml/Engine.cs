// Engine.cs — the XAML/.NET host glue for the VisiCalc demo, computing on the
// shared Rust `spreadsheet-core` engine through its C ABI (spreadsheet-capi),
// reached via P/Invoke — the path WinUI / XAML use on Windows.
//
// This is the .NET sibling of the SwiftUI demo's Engine.swift, the Qt demo's
// SpreadsheetModel, the Flutter demo's lib/engine.dart, and the Compose demo's
// Engine.kt: it owns NO spreadsheet logic. The Rust engine (cells, dependency
// graph, recalc, formulas) lives behind the C ABI; this file marshals .NET
// strings across it and maps the engine's JSON value shape into display text.
//
// Deliberately free of any WinUI / Microsoft.UI dependency: it's plain
// System.Runtime.InteropServices + System.Text.Json, so the same file compiles
// into the WinUI app on Windows AND into the cross-platform console test
// (test/EngineSmoke) that proves the engine path on macOS/Linux.

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace Mosaic.Generated;

/// P/Invoke surface over the C ABI. The library name "spreadsheet_capi" is
/// resolved at load time to the vendored dynamic library (native/…), or to an
/// explicit CAPI_LIB override — so the same bindings work whether the engine is
/// a Windows .dll, a macOS .dylib, or a Linux .so.
internal static class ScNative
{
    static ScNative()
    {
        NativeLibrary.SetDllImportResolver(typeof(ScNative).Assembly, (name, asm, searchPath) =>
            name == "spreadsheet_capi" ? NativeLibrary.Load(ResolveLibraryPath()) : IntPtr.Zero);
    }

    [DllImport("spreadsheet_capi")] internal static extern IntPtr sc_session_new();
    [DllImport("spreadsheet_capi")] internal static extern void sc_session_free(IntPtr s);

    [DllImport("spreadsheet_capi")]
    internal static extern IntPtr sc_set_cell(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string a1,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string raw);

    [DllImport("spreadsheet_capi")]
    internal static extern IntPtr sc_get_value(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string a1);

    [DllImport("spreadsheet_capi")]
    internal static extern IntPtr sc_get_raw(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string a1);

    // Viewport primitive: integer coords (uint32_t) and a u64 revision; JSON
    // char* results, except sc_current_revision returns the u64 directly.
    [DllImport("spreadsheet_capi")]
    internal static extern IntPtr sc_get_window(IntPtr s, uint row0, uint col0, uint row1, uint col1);
    // Format-aware sibling of sc_get_window: each cell is its display string.
    [DllImport("spreadsheet_capi")]
    internal static extern IntPtr sc_get_display_window(IntPtr s, uint row0, uint col0, uint row1, uint col1);
    // sc_set_format(session, a1, code) → void (an empty code clears the format).
    [DllImport("spreadsheet_capi")]
    internal static extern void sc_set_format(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string a1,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string code);
    // sc_fill(session, src, dst_start, dst_end) → void (drag-fill; three A1 strings).
    [DllImport("spreadsheet_capi")]
    internal static extern void sc_fill(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string src,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string dstStart,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string dstEnd);
    // sc_sort_range(session, start, end, key_col, ascending) → int (1 applied /
    // already sorted, 0 no-op). Two A1 strings + a 1-based key column + a flag.
    [DllImport("spreadsheet_capi")]
    internal static extern int sc_sort_range(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string start,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string end,
        uint keyCol, int ascending);
    // sc_copy / sc_cut(session, start, end) → void (clipboard capture; two A1 strings).
    [DllImport("spreadsheet_capi")]
    internal static extern void sc_copy(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string start,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string end);
    [DllImport("spreadsheet_capi")]
    internal static extern void sc_cut(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string start,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string end);
    // sc_paste(session, dst_start) → int (1 applied, 0 no-op).
    [DllImport("spreadsheet_capi")]
    internal static extern int sc_paste(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string dstStart);
    // sc_insert_rows / sc_delete_rows / sc_insert_cols / sc_delete_cols(session,
    // at, count) → void. Structural edits at a 1-based position; the engine
    // shifts every formula reference across the band.
    [DllImport("spreadsheet_capi")] internal static extern void sc_insert_rows(IntPtr s, uint at, uint count);
    [DllImport("spreadsheet_capi")] internal static extern void sc_delete_rows(IntPtr s, uint at, uint count);
    [DllImport("spreadsheet_capi")] internal static extern void sc_insert_cols(IntPtr s, uint at, uint count);
    [DllImport("spreadsheet_capi")] internal static extern void sc_delete_cols(IntPtr s, uint at, uint count);
    // sc_serialize(session) → char* (workbook source + formats as JSON);
    // sc_deserialize(session, data) → int (1 loaded, 0 malformed/unsupported).
    [DllImport("spreadsheet_capi")] internal static extern IntPtr sc_serialize(IntPtr s);
    [DllImport("spreadsheet_capi")]
    internal static extern int sc_deserialize(IntPtr s,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string data);
    // sc_undo / sc_redo / sc_can_undo / sc_can_redo(session) → int (1/0).
    [DllImport("spreadsheet_capi")] internal static extern int sc_undo(IntPtr s);
    [DllImport("spreadsheet_capi")] internal static extern int sc_redo(IntPtr s);
    [DllImport("spreadsheet_capi")] internal static extern int sc_can_undo(IntPtr s);
    [DllImport("spreadsheet_capi")] internal static extern int sc_can_redo(IntPtr s);
    [DllImport("spreadsheet_capi")] internal static extern IntPtr sc_used_range(IntPtr s);
    [DllImport("spreadsheet_capi")] internal static extern IntPtr sc_column_letters(IntPtr s, uint index);
    [DllImport("spreadsheet_capi")] internal static extern ulong sc_current_revision(IntPtr s);
    [DllImport("spreadsheet_capi")] internal static extern IntPtr sc_changed_since(IntPtr s, ulong since);

    [DllImport("spreadsheet_capi")] internal static extern void sc_string_free(IntPtr p);

    /// Resolve the vendored engine library: an explicit CAPI_LIB env var, or
    /// `native/<lib>` found by walking up from the app base directory and the
    /// current directory. The filename is a hardcoded per-OS constant.
    internal static string ResolveLibraryPath()
    {
        string name = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? "spreadsheet_capi.dll"
            : RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
                ? "libspreadsheet_capi.dylib"
                : "libspreadsheet_capi.so";

        string? env = Environment.GetEnvironmentVariable("CAPI_LIB");
        if (!string.IsNullOrEmpty(env) && File.Exists(env)) return env;

        foreach (var start in new[] { AppContext.BaseDirectory, Directory.GetCurrentDirectory() })
        {
            var dir = new DirectoryInfo(start);
            for (int i = 0; i < 6 && dir is not null; i++, dir = dir.Parent)
            {
                var candidate = Path.Combine(dir.FullName, "native", name);
                if (File.Exists(candidate)) return candidate;
            }
        }
        return Path.Combine("native", name); // let NativeLibrary throw a clear error
    }
}

/// A single spreadsheet session, owning the opaque C handle.
public sealed class SpreadsheetSession : IDisposable
{
    private IntPtr _handle = ScNative.sc_session_new();

    /// Read an engine-returned char* into a .NET string and free it with the
    /// engine's allocator (sc_string_free — NOT the CLR's). NULL becomes "".
    private static string Take(IntPtr p)
    {
        if (p == IntPtr.Zero) return string.Empty;
        string s = Marshal.PtrToStringUTF8(p) ?? string.Empty;
        ScNative.sc_string_free(p);
        return s;
    }

    public string SetCell(string a1, string raw) => Take(ScNative.sc_set_cell(_handle, a1, raw));
    public string GetValueJson(string a1) => Take(ScNative.sc_get_value(_handle, a1));
    public string GetRaw(string a1) => Take(ScNative.sc_get_raw(_handle, a1));

    /// Set a cell's display format code (an Excel-style code like "#,##0.00" or
    /// "0%"); an empty code clears it. Drives the engine's display path that
    /// <see cref="Window"/> reads through sc_get_display_window.
    public void SetFormat(string a1, string code) => ScNative.sc_set_format(_handle, a1, code);

    /// Drag-fill: replicate the `src` cell across the inclusive A1 rectangle
    /// `dstStart`..`dstEnd`. Relative references shift per target (`=A1` filled
    /// one row down becomes `=A2`), absolute (`$`) refs pin, off-grid refs become
    /// `#REF!`; the source's display format rides along. The engine recomputes
    /// every dependent. Reaches sc_fill — the same path every other backend drives.
    public void Fill(string src, string dstStart, string dstEnd) =>
        ScNative.sc_fill(_handle, src, dstStart, dstEnd);

    /// Range sort: reorder the rows of the rectangle `start`..`end` by the
    /// computed values in `keyCol` (1-based, inside the rectangle), ascending or
    /// descending. Each row moves as a record; the engine shifts moved formulas'
    /// references with their row and carries formats. Returns true when a sort was
    /// applied (or the range was already sorted), false for a no-op. `keyCol` is
    /// clamped to ≥ 0 before the uint cast; the engine validates it lies inside
    /// the rectangle. Reaches sc_sort_range — the same path every backend drives.
    public bool SortRange(string start, string end, int keyCol, bool ascending) =>
        ScNative.sc_sort_range(_handle, start, end, (uint)Math.Max(0, keyCol), ascending ? 1 : 0) != 0;

    /// Structural edits: insert / delete `count` rows or columns at the 1-based
    /// position `at`. The engine shifts every formula reference at or after the
    /// band (a reference whose whole band is deleted becomes `#REF!`), then
    /// recomputes. `at`/`count` are clamped to ≥ 0 before the uint cast so a
    /// negative can't wrap to a huge unsigned band.
    public void InsertRows(int at, int count) =>
        ScNative.sc_insert_rows(_handle, (uint)Math.Max(0, at), (uint)Math.Max(0, count));
    public void DeleteRows(int at, int count) =>
        ScNative.sc_delete_rows(_handle, (uint)Math.Max(0, at), (uint)Math.Max(0, count));
    public void InsertCols(int at, int count) =>
        ScNative.sc_insert_cols(_handle, (uint)Math.Max(0, at), (uint)Math.Max(0, count));
    public void DeleteCols(int at, int count) =>
        ScNative.sc_delete_cols(_handle, (uint)Math.Max(0, at), (uint)Math.Max(0, count));

    /// Copy the inclusive rectangle `start`..`end` into the clipboard — a
    /// whole-block copy that pastes as a unit. The source is untouched; the
    /// buffer survives any number of pastes.
    public void Copy(string start, string end) => ScNative.sc_copy(_handle, start, end);

    /// Cut the inclusive rectangle `start`..`end`. Like <see cref="Copy"/> but a
    /// one-shot move: the paste that places it clears the source it didn't overwrite.
    public void Cut(string start, string end) => ScNative.sc_cut(_handle, start, end);

    /// Paste the clipboard so its top-left lands at `dstStart`. Returns `true`
    /// when applied, `false` (a no-op) for an empty clipboard, malformed address,
    /// or off-grid destination. The block's references shift by the destination's
    /// offset; content and format ride along.
    public bool Paste(string dstStart) => ScNative.sc_paste(_handle, dstStart) != 0;

    /// Serialize the whole workbook to a self-contained JSON document — the
    /// SOURCE (formula text + typed literals) + per-cell formats, not the
    /// computed values (those recompute on load, so the document is small and
    /// can't disagree with itself). <see cref="Take"/> frees the engine's char*;
    /// the host persists the returned string wherever it likes.
    public string Serialize() => Take(ScNative.sc_serialize(_handle));

    /// Replace the workbook from a document produced by <see cref="Serialize"/>.
    /// Returns `true` on success, `false` for malformed / unsupported input (the
    /// workbook is left untouched — the engine validates before it mutates).
    /// Formulas reload live.
    public bool Deserialize(string data) => ScNative.sc_deserialize(_handle, data) != 0;

    /// Undo / redo: walk the engine's snapshot history. Each returns `true` if it
    /// changed the document (the host then re-reads the viewport), `false` if
    /// there was nothing to do. CanUndo/CanRedo gate a host's Undo/Redo controls.
    public bool Undo() => ScNative.sc_undo(_handle) != 0;
    public bool Redo() => ScNative.sc_redo(_handle) != 0;
    public bool CanUndo() => ScNative.sc_can_undo(_handle) != 0;
    public bool CanRedo() => ScNative.sc_can_redo(_handle) != 0;

    /// The display string for a cell — what a spreadsheet should show. Parses
    /// the engine's JSON (the fixed shape every backend's engine emits).
    public string Display(string a1)
    {
        string json = GetValueJson(a1);
        if (string.IsNullOrEmpty(json)) return string.Empty;
        // The JSON is engine-emitted and well-formed, but parse defensively so a
        // malformed/unexpected response degrades to "#ERR" rather than throwing
        // up to the UI thread and crashing the window.
        try
        {
            using var doc = JsonDocument.Parse(json);
            return DisplayValue(doc.RootElement);
        }
        catch (Exception ex) when (ex is JsonException or KeyNotFoundException or InvalidOperationException)
        {
            return "#ERR";
        }
    }

    // ── Viewport primitive (virtualized infinite sheet) ──────────────
    // These mirror the engine's get_window / used_range / changed_since reads
    // (1-based inclusive coords) so a windowed XAML grid (a virtualizing
    // ItemsRepeater / ListView) can render only the visible rectangle of an
    // unbounded sheet — the .NET sibling of the web/SwiftUI/Qt/Flutter/Compose
    // infinite views.

    /// Map one decoded value object (`{"kind":...}`) to its display string.
    /// Shared by `Display` (one cell) and `Window` (a whole rectangle).
    private static string DisplayValue(JsonElement obj)
    {
        if (!obj.TryGetProperty("kind", out var kindEl)) return string.Empty;
        switch (kindEl.GetString())
        {
            case "empty":
                return string.Empty;
            case "number":
            {
                double d = obj.GetProperty("value").GetDouble();
                return (d == Math.Floor(d) && Math.Abs(d) < 1e15)
                    ? ((long)d).ToString(System.Globalization.CultureInfo.InvariantCulture)
                    : d.ToString(System.Globalization.CultureInfo.InvariantCulture);
            }
            case "text":
                return obj.GetProperty("value").GetString() ?? string.Empty;
            case "boolean":
                return obj.GetProperty("value").GetBoolean() ? "TRUE" : "FALSE";
            case "error":
                return obj.GetProperty("code").GetString() ?? "#ERR";
            default:
                return string.Empty;
        }
    }

    /// Dense display strings for the inclusive 1-based rectangle, row-major
    /// (empty cells become ""). Empty list on a bad/oversized request.
    ///
    /// Reads sc_get_display_window: each cell arrives already rendered through its
    /// format code as a display string, so the host paints it directly and never
    /// re-derives number formatting. The format-aware sibling of sc_get_window;
    /// the JSON is {...,"cells":[["1,234.50",…],…]}.
    public IReadOnlyList<IReadOnlyList<string>> Window(uint row0, uint col0, uint row1, uint col1)
    {
        string json = Take(ScNative.sc_get_display_window(_handle, row0, col0, row1, col1));
        var rows = new List<IReadOnlyList<string>>();
        try
        {
            using var doc = JsonDocument.Parse(json);
            if (!doc.RootElement.TryGetProperty("cells", out var cells)) return rows;
            foreach (var rowEl in cells.EnumerateArray())
            {
                var row = new List<string>();
                foreach (var cell in rowEl.EnumerateArray()) row.Add(cell.GetString() ?? string.Empty);
                rows.Add(row);
            }
        }
        catch (Exception ex) when (ex is JsonException or KeyNotFoundException or InvalidOperationException) { /* bad/oversized request → empty */ }
        return rows;
    }

    /// The data extent (1-based inclusive), or null if the sheet is empty.
    public (uint minRow, uint minCol, uint maxRow, uint maxCol)? UsedRange()
    {
        string json = Take(ScNative.sc_used_range(_handle));
        try
        {
            using var doc = JsonDocument.Parse(json);
            var r = doc.RootElement;
            if (r.ValueKind != JsonValueKind.Object) return null; // "null" → empty sheet
            return (r.GetProperty("minRow").GetUInt32(), r.GetProperty("minCol").GetUInt32(),
                    r.GetProperty("maxRow").GetUInt32(), r.GetProperty("maxCol").GetUInt32());
        }
        catch (Exception ex) when (ex is JsonException or KeyNotFoundException or InvalidOperationException)
        {
            return null;
        }
    }

    /// Column letters for a 1-based index (`1` → `"A"`, `27` → `"AA"`).
    public string ColumnLetters(uint index) => Take(ScNative.sc_column_letters(_handle, index));

    /// The per-edit revision clock. Snapshot it, then pass to ChangedSince.
    public ulong CurrentRevision() => ScNative.sc_current_revision(_handle);

    /// Cells changed since `since`; `stale` means re-read the whole window.
    public (IReadOnlyList<string> changed, bool stale) ChangedSince(ulong since)
    {
        string json = Take(ScNative.sc_changed_since(_handle, since));
        try
        {
            using var doc = JsonDocument.Parse(json);
            var r = doc.RootElement;
            if (r.TryGetProperty("stale", out var st) && st.GetBoolean())
                return (Array.Empty<string>(), true);
            var changed = new List<string>();
            if (r.TryGetProperty("changed", out var arr))
                foreach (var c in arr.EnumerateArray()) changed.Add(c.GetString() ?? string.Empty);
            return (changed, false);
        }
        catch (Exception ex) when (ex is JsonException or KeyNotFoundException or InvalidOperationException)
        {
            return (Array.Empty<string>(), false);
        }
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            ScNative.sc_session_free(_handle);
            _handle = IntPtr.Zero;
        }
    }
}

/// An engine-backed 5×5 spreadsheet the XAML host drives. Mirrors the SwiftUI /
/// Qt / Flutter / Compose models: seeds the cross-footing budget, exposes the
/// computed display matrix, and writes through to the engine on edit.
public sealed class SpreadsheetModel : IDisposable
{
    public const int Rows = 5;
    public const int Cols = 5; // A..E

    private readonly SpreadsheetSession _session = new();

    public SpreadsheetModel() => Seed();

    /// A1 address for grid display row `r` (0-based) and column `c` (1..5).
    public static string Address(int r, int c) => $"{(char)('A' + c - 1)}{r + 1}";

    /// The classic cross-footing budget — identical seed to every other demo.
    private void Seed()
    {
        (string a1, string raw)[] cells =
        {
            ("A1", "15"), ("B1", "3"),  ("C1", "12"), ("D1", "8"),  ("E1", "=SUM(A1:D1)"),
            ("A2", "8"),  ("B2", "14"), ("C2", "7"),  ("D2", "22"), ("E2", "=SUM(A2:D2)"),
            ("A3", "12"), ("B3", "9"),  ("C3", "18"), ("D3", "6"),  ("E3", "=SUM(A3:D3)"),
            ("A4", "4"),  ("B4", "11"), ("C4", "3"),  ("D4", "17"), ("E4", "=SUM(A4:D4)"),
            ("A5", "=SUM(A1:A4)"), ("B5", "=SUM(B1:B4)"), ("C5", "=SUM(C1:C4)"),
            ("D5", "=SUM(D1:D4)"), ("E5", "=SUM(E1:E4)"),
        };
        foreach (var (a1, raw) in cells) _session.SetCell(a1, raw);
    }

    /// Display matrix fed to the Grid: each row is [rowLabel, A, B, C, D, E].
    public IReadOnlyList<IReadOnlyList<string>> ViewportRows()
    {
        var rows = new List<IReadOnlyList<string>>(Rows);
        for (int r = 0; r < Rows; r++)
        {
            var row = new List<string> { (r + 1).ToString() };
            for (int c = 1; c <= Cols; c++) row.Add(_session.Display(Address(r, c)));
            rows.Add(row);
        }
        return rows;
    }

    /// The raw source of the cell at display row/col (col 1..5; 0 = gutter).
    public string RawAt(int r, int c) => c < 1 ? string.Empty : _session.GetRaw(Address(r, c));

    /// The value JSON of a cell — used by the verify harness to assert directly.
    public string ValueJson(string a1) => _session.GetValueJson(a1);

    /// Write `raw` into the cell at display row/col. The caller rebuilds the
    /// display matrix afterwards via ViewportRows().
    public void SetCell(int r, int c, string raw)
    {
        if (c >= 1) _session.SetCell(Address(r, c), raw);
    }

    public void Dispose() => _session.Dispose();
}

/// Engine-backed model for the VIRTUALIZED infinite sheet — the .NET sibling of
/// the SwiftUI `WindowedSheetModel`, the Qt `SpreadsheetModel` infinite-view
/// state, the Flutter/Compose `InfiniteSheetModel`. It seeds a deliberately
/// far-flung, sparse dataset and exposes one-row windowed reads plus the data
/// extent, so a virtualizing XAML `ListView` (which realizes only on-screen
/// items) can render only the visible rectangle of an effectively-unbounded
/// (u32 × u32) sheet.
///
/// Plain .NET (no WinUI types), so the same cross-platform console test that
/// proves `SpreadsheetModel` also proves this. All coordinates are 1-based
/// (row/col ≥ 1, col 1 = "A"), matching the engine.
public sealed class InfiniteSheetModel : IDisposable
{
    private readonly SpreadsheetSession _session = new();

    /// The virtual grid size, derived from the data extent plus a margin so you
    /// can scroll past the data into blank space.
    public int TotalRows { get; private set; } = 1000;
    public int TotalCols { get; private set; } = 60;

    /// The selected cell (1-based) and the formula-bar text (its raw source).
    public int SelRow { get; private set; } = 1;
    public int SelCol { get; private set; } = 1;
    public string Formula { get; private set; } = string.Empty;

    public InfiniteSheetModel()
    {
        Seed();
        ComputeExtent();
        SelectInf(1, 1); // prime the selection + formula bar at A1
    }

    /// The classic cross-footing budget PLUS far-flung cells (a formula at
    /// `Z1000`, a couple near `BA50`/`BB50`) to prove the sheet is sparse and
    /// unbounded — identical seed to the SwiftUI/Qt/Flutter/Compose infinite views.
    private void Seed()
    {
        (string a1, string raw)[] cells =
        {
            ("A1", "15"), ("B1", "3"),  ("C1", "12"), ("D1", "8"),  ("E1", "=SUM(A1:D1)"),
            ("A2", "8"),  ("B2", "14"), ("C2", "7"),  ("D2", "22"), ("E2", "=SUM(A2:D2)"),
            ("A3", "12"), ("B3", "9"),  ("C3", "18"), ("D3", "6"),  ("E3", "=SUM(A3:D3)"),
            ("A4", "4"),  ("B4", "11"), ("C4", "3"),  ("D4", "17"), ("E4", "=SUM(A4:D4)"),
            ("A5", "=SUM(A1:A4)"), ("B5", "=SUM(B1:B4)"), ("C5", "=SUM(C1:C4)"),
            ("D5", "=SUM(D1:D4)"), ("E5", "=SUM(E1:E4)"),
            ("Z1000", "=SUM(A1:A4)"),                 // 1000 rows down: 39
            ("BA50", "far cell"), ("BB50", "=Z1000*2"), // col 53/54, row 50: 78
        };
        foreach (var (a1, raw) in cells) _session.SetCell(a1, raw);

        // Attach Excel-style format codes so the engine's display path is visible
        // in the windowed view (which renders via sc_get_display_window): the
        // cross-foot totals read with thousands grouping + two decimals, and the
        // far-flung Z1000 total as a percent. Values are unchanged — only how the
        // display strings render. Identical to the web/Qt/Flutter/Compose demos.
        (string a1, string code)[] formats =
        {
            ("E1", "#,##0.00"), ("E2", "#,##0.00"), ("E3", "#,##0.00"),
            ("E4", "#,##0.00"), ("E5", "#,##0.00"),
            ("A5", "#,##0.00"), ("B5", "#,##0.00"), ("C5", "#,##0.00"), ("D5", "#,##0.00"),
            ("Z1000", "0.0%"), // 39 → "3900.0%": proves the format applies far off-origin
        };
        foreach (var (a1, code) in formats) _session.SetFormat(a1, code);
    }

    /// Re-derive the virtual grid size from the engine's data extent plus a
    /// comfortable margin. The `+ margin` is done in `long` and saturated back
    /// into `int`: the engine is u32-backed, so a `maxRow`/`maxCol` near
    /// `int.MaxValue` would otherwise overflow the add to a negative size. Not
    /// reachable in this demo (the only far cell is the fixed Z1000), but the
    /// saturation keeps the model safe against any sheet.
    public void ComputeExtent()
    {
        var u = _session.UsedRange();
        long maxRow = u?.maxRow ?? 1u;
        long maxCol = u?.maxCol ?? 1u;
        TotalRows = Saturate(maxRow + 200, floor: 1000);
        TotalCols = Saturate(maxCol + 30, floor: 60);
    }

    private static int Saturate(long value, int floor) =>
        (int)Math.Clamp(value, floor, int.MaxValue);

    /// Column letters for a 1-based index (`1` → `"A"`, `27` → `"AA"`).
    public string ColumnLetters(int index) => _session.ColumnLetters((uint)index);

    /// The A1 address of the selected cell (e.g. `"Z1000"`).
    public string InfAddress => $"{_session.ColumnLetters((uint)SelCol)}{SelRow}";

    /// The engine's per-edit revision clock — bumps on every mutation. The
    /// status footer shows it so the live recompute is visible while scrolling.
    public ulong Revision => _session.CurrentRevision();

    /// One row's display strings (columns 1..TotalCols) — what a virtualized
    /// `ListView` item renders. A single engine `get_window` over a 1×N strip;
    /// returns an empty list if the request was rejected/oversized.
    public IReadOnlyList<string> RowCells(int row)
    {
        if (row < 1) return Array.Empty<string>();
        var w = _session.Window((uint)row, 1, (uint)row, (uint)TotalCols);
        return w.Count == 0 ? Array.Empty<string>() : w[0];
    }

    /// Move the selection (clamped to the virtual grid; row/col ≥ 1) and pull the
    /// selected cell's raw source into the formula bar.
    public void SelectInf(int row, int col)
    {
        SelRow = Math.Clamp(row, 1, TotalRows);
        SelCol = Math.Clamp(col, 1, TotalCols);
        Formula = _session.GetRaw(InfAddress);
    }

    /// Commit the formula bar into the selected cell: write through to the engine
    /// (which recomputes every dependent), grow the extent if the edit reached new
    /// ground, and re-read the canonicalised source back into the bar.
    public void CommitInf(string raw)
    {
        _session.SetCell(InfAddress, raw);
        ComputeExtent();
        Formula = _session.GetRaw(InfAddress);
    }

    /// Drag-fill: replicate the selected cell into the `rows` rows below it. The
    /// engine shifts each copy's relative references (`=A1`→`=A2`, …), pins
    /// absolute (`$`) refs, carries the format, and recomputes every dependent.
    /// Regrows the extent if the fill reached new ground. The .NET sibling of the
    /// Flutter/Compose `FillDown` and the Qt "Fill ↓ 10" button.
    public void FillDown(int rows)
    {
        string col = _session.ColumnLetters((uint)SelCol);
        // Widen the row arithmetic to `long` and saturate back into a valid row
        // before building the A1 strings. SelRow is clamped to [1, TotalRows] but
        // TotalRows can be up to int.MaxValue, so `SelRow + rows` would otherwise
        // overflow to a negative row (the same u32-overflow-defeats-cap hazard the
        // Saturate helper guards) and emit a malformed address. Saturate clamps to
        // [1, int.MaxValue]; the engine treats any off-grid target as #REF! and
        // ComputeExtent() below regrows the virtual grid to cover the fill.
        int first = Saturate((long)SelRow + 1, floor: 1);
        int last = Saturate((long)SelRow + rows, floor: 1);
        _session.Fill(InfAddress, $"{col}{first}", $"{col}{last}");
        ComputeExtent();
    }

    /// Structural edits: insert / delete the selected cell's row or column. The
    /// engine shifts every formula reference at or after the band (a reference
    /// whose whole band is deleted becomes `#REF!`) and recomputes; regrow the
    /// extent so the view re-reads. Operate on a single row/column at the cursor.
    public void InsertRow() { _session.InsertRows(SelRow, 1); ComputeExtent(); }
    public void DeleteRow() { _session.DeleteRows(SelRow, 1); ComputeExtent(); }
    public void InsertCol() { _session.InsertCols(SelCol, 1); ComputeExtent(); }
    public void DeleteCol() { _session.DeleteCols(SelCol, 1); ComputeExtent(); }

    /// Number formatting: attach an Excel-style format code to the selected cell
    /// ("#,##0.00", "0.0%", "$#,##0.00", or "" to clear). Display-only — the
    /// stored value is unchanged; the engine renders it through the code, so a
    /// fresh RowCells read shows the formatted string.
    public void ApplyFormat(string code) => _session.SetFormat(InfAddress, code);

    /// Range sort: reorder the rows of the seeded budget block A1:E4 by the
    /// SELECTED column (clamped into the block's columns A..E = 1..5), ascending
    /// or descending. Each row moves as a record; the E-column SUM formulas travel
    /// with their row (the engine shifts their refs), so every total stays correct.
    /// Returns false for a no-op (already sorted / bad args). Regrows the extent.
    public bool SortBlock(bool ascending)
    {
        int keyCol = Math.Clamp(SelCol, 1, 5);
        bool ok = _session.SortRange("A1", "E4", keyCol, ascending);
        ComputeExtent();
        return ok;
    }

    /// Clipboard: copy/cut the selected cell, then paste it at the selection. The
    /// engine shifts the pasted formula's relative references by the destination's
    /// offset, pins absolute (`$`) refs, carries the format; a cut clears the
    /// source on paste. <see cref="PasteCell"/> returns false (a no-op) when the
    /// clipboard is empty, and regrows the extent on success.
    public void CopyCell() => _session.Copy(InfAddress, InfAddress);
    public void CutCell() => _session.Cut(InfAddress, InfAddress);
    public bool PasteCell()
    {
        bool ok = _session.Paste(InfAddress);
        if (ok) ComputeExtent();
        return ok;
    }

    /// Save / load: serialize the whole workbook to a JSON document, and restore
    /// it. The document stores only the source + formats — computed values
    /// recompute on load, so a loaded formula stays live. <see cref="LoadBook"/>
    /// returns false (workbook untouched) for malformed input; on success it
    /// regrows the extent and refreshes the formula bar so the view re-reads.
    public string SaveBook() => _session.Serialize();
    public bool LoadBook(string data)
    {
        bool ok = _session.Deserialize(data);
        if (ok)
        {
            ComputeExtent();
            Formula = _session.GetRaw(InfAddress);
        }
        return ok;
    }

    /// Undo / redo: walk the engine's snapshot history. On success the extent
    /// regrows and the formula bar refreshes (any cell could have changed); a
    /// restored formula stays live. CanUndo/CanRedo gate the buttons.
    public bool CanUndo => _session.CanUndo();
    public bool CanRedo => _session.CanRedo();
    public bool UndoEdit()
    {
        bool ok = _session.Undo();
        if (ok)
        {
            ComputeExtent();
            Formula = _session.GetRaw(InfAddress);
        }
        return ok;
    }
    public bool RedoEdit()
    {
        bool ok = _session.Redo();
        if (ok)
        {
            ComputeExtent();
            Formula = _session.GetRaw(InfAddress);
        }
        return ok;
    }

    public void Dispose() => _session.Dispose();
}
