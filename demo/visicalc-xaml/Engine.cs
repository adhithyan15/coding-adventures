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
    public IReadOnlyList<IReadOnlyList<string>> Window(uint row0, uint col0, uint row1, uint col1)
    {
        string json = Take(ScNative.sc_get_window(_handle, row0, col0, row1, col1));
        var rows = new List<IReadOnlyList<string>>();
        try
        {
            using var doc = JsonDocument.Parse(json);
            if (!doc.RootElement.TryGetProperty("values", out var values)) return rows;
            foreach (var rowEl in values.EnumerateArray())
            {
                var row = new List<string>();
                foreach (var cell in rowEl.EnumerateArray()) row.Add(DisplayValue(cell));
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
