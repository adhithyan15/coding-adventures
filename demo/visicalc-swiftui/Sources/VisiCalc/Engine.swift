// Engine.swift — a thin Swift wrapper over the spreadsheet engine's C ABI
// (spreadsheet-capi), and the SwiftUI-facing model that drives the views.
//
// This is the SwiftUI demo's "host glue": it owns no spreadsheet logic. The
// Rust `spreadsheet-core` engine (cells, dependency graph, recalc, formulas)
// lives behind the C ABI; this file marshals Swift Strings across it and maps
// the JSON value shape into display text — the same engine, and the same JSON
// contract, the web demos drive as WASM.

import Foundation
import CSpreadsheetEngine

/// A single spreadsheet session, owning the opaque C handle.
final class SpreadsheetSession {
    private let handle: OpaquePointer?

    init() { handle = sc_session_new() }
    deinit { sc_session_free(handle) }

    /// Consume a `char *` the C ABI returned, as a Swift String, freeing it.
    private func take(_ p: UnsafeMutablePointer<CChar>?) -> String {
        guard let p = p else { return "" }
        defer { sc_string_free(p) }
        return String(cString: p)
    }

    @discardableResult
    func setCell(_ a1: String, _ raw: String) -> String {
        // Swift bridges String → const char* for the duration of the call.
        take(sc_set_cell(handle, a1, raw))
    }

    func getValueJSON(_ a1: String) -> String { take(sc_get_value(handle, a1)) }
    func getRaw(_ a1: String) -> String { take(sc_get_raw(handle, a1)) }

    /// Set a cell's display format code (an Excel-style code like `"#,##0.00"` or
    /// `"0%"`); an empty code clears it. Drives the engine's display path that
    /// `window` reads through `sc_get_display_window`.
    func setFormat(_ a1: String, _ code: String) { sc_set_format(handle, a1, code) }

    /// Drag-fill: replicate the `src` cell across the inclusive A1 rectangle
    /// `dstStart`..`dstEnd`. Relative references shift per target (`=A1` filled
    /// one row down becomes `=A2`), absolute (`$`) refs pin, off-grid refs become
    /// `#REF!`, and the source's display format rides along; the engine recomputes
    /// every dependent. Reaches `sc_fill` — the same path the web/Qt/Flutter/
    /// Compose/XAML demos drive.
    func fill(_ src: String, _ dstStart: String, _ dstEnd: String) {
        sc_fill(handle, src, dstStart, dstEnd)
    }

    /// Range sort: reorder the rows of the rectangle `start`..`end` by the
    /// computed values in `keyCol` (1-based, inside the rectangle), ascending or
    /// descending. Each row moves as a record; the engine shifts moved formulas'
    /// references with their row and carries formats. Returns true when a sort was
    /// applied (or the range was already sorted), false for a no-op (malformed
    /// address / out-of-range key / oversized range). Reaches `sc_sort_range` —
    /// the same path every backend drives.
    func sortRange(_ start: String, _ end: String, _ keyCol: UInt32, _ ascending: Bool) -> Bool {
        sc_sort_range(handle, start, end, keyCol, ascending ? 1 : 0) != 0
    }

    /// Structural edits: insert / delete `count` rows or columns at the 1-based
    /// position `at`. The engine shifts every formula reference at or after the
    /// band (a reference whose whole band is deleted becomes `#REF!`), then
    /// recomputes. `at`/`count` are clamped to ≥ 0 before the UInt32 conversion
    /// so a negative can't trap / wrap into a huge unsigned band.
    func insertRows(_ at: Int, _ count: Int) { sc_insert_rows(handle, UInt32(max(0, at)), UInt32(max(0, count))) }
    func deleteRows(_ at: Int, _ count: Int) { sc_delete_rows(handle, UInt32(max(0, at)), UInt32(max(0, count))) }
    func insertCols(_ at: Int, _ count: Int) { sc_insert_cols(handle, UInt32(max(0, at)), UInt32(max(0, count))) }
    func deleteCols(_ at: Int, _ count: Int) { sc_delete_cols(handle, UInt32(max(0, at)), UInt32(max(0, count))) }

    /// Copy the inclusive rectangle `start`..`end` into the clipboard — a
    /// whole-block copy that pastes as a unit. The source is untouched; the
    /// buffer survives any number of pastes. Reaches `sc_copy`.
    func copy(_ start: String, _ end: String) {
        sc_copy(handle, start, end)
    }

    /// Cut the inclusive rectangle `start`..`end`. Like `copy` but a one-shot
    /// move: the `paste` that places it clears the source it didn't overwrite.
    func cut(_ start: String, _ end: String) {
        sc_cut(handle, start, end)
    }

    /// Paste the clipboard so its top-left lands at `dstStart`. Returns `true`
    /// when applied, `false` (a no-op) for an empty clipboard, malformed address,
    /// or off-grid destination. The block's references shift by the destination's
    /// offset; content and format ride along.
    func paste(_ dstStart: String) -> Bool {
        sc_paste(handle, dstStart) != 0
    }

    /// Serialize the whole workbook to a self-contained JSON document — the
    /// SOURCE (formula text + typed literals) + per-cell formats, not the
    /// computed values (those recompute on load, so the document is small and
    /// can't disagree with itself). `take` frees the engine's `char*`; the host
    /// persists the returned string wherever it likes. Reaches `sc_serialize`.
    func serialize() -> String {
        take(sc_serialize(handle))
    }

    /// Replace the workbook from a document produced by `serialize`. Returns
    /// `true` on success, `false` for malformed / unsupported input (the workbook
    /// is left untouched — the engine validates before it mutates). Formulas
    /// reload live. Reaches `sc_deserialize`.
    func deserialize(_ data: String) -> Bool {
        sc_deserialize(handle, data) != 0
    }

    /// Undo / redo: walk the engine's snapshot history. Each returns `true` if it
    /// changed the document (the host then re-reads the viewport), `false` if
    /// there was nothing to do. canUndo/canRedo gate a host's Undo/Redo controls.
    /// Reach `sc_undo`/`sc_redo`/`sc_can_undo`/`sc_can_redo`.
    func undo() -> Bool { sc_undo(handle) != 0 }
    func redo() -> Bool { sc_redo(handle) != 0 }
    func canUndo() -> Bool { sc_can_undo(handle) != 0 }
    func canRedo() -> Bool { sc_can_redo(handle) != 0 }

    /// The computed value of a cell as the string a spreadsheet should show.
    /// Parses the engine's JSON (`{"kind":...}`) — the same shape the TS and
    /// WASM engines emit.
    func display(_ a1: String) -> String {
        let json = getValueJSON(a1)
        guard
            let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return "" }
        return Self.displayValue(obj)
    }

    /// Map one decoded value object (`{"kind":...}`) to the string a spreadsheet
    /// cell should show. Shared by `display` (one cell) and `window` (a whole
    /// rectangle) so both render values identically.
    static func displayValue(_ obj: [String: Any]) -> String {
        switch obj["kind"] as? String {
        case "empty": return ""
        case "number":
            guard let n = obj["value"] as? Double else { return "" }
            // Show integers without a trailing ".0".
            return n == n.rounded() && abs(n) < 1e15 ? String(Int(n)) : String(n)
        case "text": return obj["value"] as? String ?? ""
        case "boolean": return (obj["value"] as? Bool ?? false) ? "TRUE" : "FALSE"
        case "error": return obj["code"] as? String ?? "#ERR"
        default: return ""
        }
    }

    // MARK: Viewport primitive (virtualized infinite sheet)
    //
    // These mirror the engine's `get_window` / `used_range` / `changed_since`
    // reads (1-based inclusive coords) so a SwiftUI host can render only the
    // visible window of an unbounded sheet, sized from the data extent and
    // refreshed by the per-edit change diff — the same primitive the web demo's
    // infinite.html uses through WASM.

    /// Dense display strings for the inclusive 1-based rectangle, row-major
    /// (empty cells become ""). Empty array on a bad/oversized request.
    ///
    /// Reads `sc_get_display_window`: each cell arrives already rendered through
    /// its format code as a display string, so the host paints it directly and
    /// never re-derives number formatting. The format-aware sibling of
    /// `sc_get_window`; the JSON is `{...,"cells":[["1,234.50",…],…]}`.
    func window(_ row0: UInt32, _ col0: UInt32, _ row1: UInt32, _ col1: UInt32) -> [[String]] {
        let json = take(sc_get_display_window(handle, row0, col0, row1, col1))
        guard
            let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let rows = obj["cells"] as? [[String]]
        else { return [] }
        return rows
    }

    /// The data extent (1-based inclusive), or nil if the sheet is empty.
    func usedRange() -> (minRow: UInt32, minCol: UInt32, maxRow: UInt32, maxCol: UInt32)? {
        let json = take(sc_used_range(handle))
        guard
            let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let minR = obj["minRow"] as? Int, let minC = obj["minCol"] as? Int,
            let maxR = obj["maxRow"] as? Int, let maxC = obj["maxCol"] as? Int,
            // Defensive: `UInt32(_:)` traps on a negative Int. The engine's
            // 1-based coords are always ≥1, but degrade to nil rather than crash
            // if a future contract break ever returned something out of range.
            let mnR = UInt32(exactly: minR), let mnC = UInt32(exactly: minC),
            let mxR = UInt32(exactly: maxR), let mxC = UInt32(exactly: maxC)
        else { return nil }
        return (mnR, mnC, mxR, mxC)
    }

    /// Column letters for a 1-based index (`1` → `"A"`, `27` → `"AA"`).
    func columnLetters(_ index: UInt32) -> String { take(sc_column_letters(handle, index)) }

    /// The per-edit revision clock. Snapshot it, then pass to `changedSince`.
    func currentRevision() -> UInt64 { sc_current_revision(handle) }

    /// Cells changed since `since`; `stale` means re-read the whole window.
    func changedSince(_ since: UInt64) -> (changed: [String], stale: Bool) {
        let json = take(sc_changed_since(handle, since))
        guard
            let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return ([], false) }
        if obj["stale"] as? Bool == true { return ([], true) }
        return (obj["changed"] as? [String] ?? [], false)
    }

    /// Find every cell whose source (when `inFormulas`) or computed display text
    /// (otherwise) contains `query`, case-sensitively when `matchCase`. Returns
    /// the matching A1 addresses, engine-sorted row-major. The engine scans only
    /// the populated cells, so the cost is bounded by the data, not the u32 grid.
    /// An empty query returns no matches. Reaches `sc_find_all` — the same engine
    /// path every other backend drives.
    func findAll(_ query: String, _ inFormulas: Bool, _ matchCase: Bool) -> [String] {
        let json = take(sc_find_all(handle, query, inFormulas ? 1 : 0, matchCase ? 1 : 0))
        guard
            let data = json.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
            let matches = obj["matches"] as? [String]
        else { return [] }
        return matches
    }

    /// Replace every occurrence of `query` with `replacement` in the SOURCE of
    /// every cell, case-sensitively when `matchCase`; returns the number of cells
    /// rewritten. The engine re-parses each rewritten source through its
    /// centralised coerce (`set_raw`), so a rewritten formula stays live and a
    /// rewritten literal stays typed, then recomputes every dependent. Reaches
    /// `sc_replace_all`.
    func replaceAll(_ query: String, _ replacement: String, _ matchCase: Bool) -> Int {
        Int(sc_replace_all(handle, query, replacement, matchCase ? 1 : 0))
    }
}

/// SwiftUI model: an engine-backed 5×5 spreadsheet. `@Published` properties
/// drive the generated GridView / FormulaBarView; mutating methods write
/// through to the engine and recompute.
final class SpreadsheetModel: ObservableObject {
    private let session = SpreadsheetSession()
    let rows = 5
    let cols = 5 // A..E

    /// Display matrix fed to GridView: each row is [rowLabel, A, B, C, D, E].
    @Published var viewportRows: [[String]] = []
    @Published var selectedRow: Double = 0      // template row 0..4
    @Published var selectedCol: Double = 1      // 1..5 (0 = row-label gutter)

    init() {
        seed()
        recompute()
    }

    /// A1 address for grid display row `r` (0-based) and column `c` (1..5).
    func address(_ r: Int, _ c: Int) -> String {
        let letter = Character(UnicodeScalar(65 + UInt32(c - 1))!)
        return "\(letter)\(r + 1)"
    }

    /// The classic cross-footing budget: column E totals each row, row 5 totals
    /// each column, E5 is the grand total — all formulas, so editing any input
    /// ripples through. Identical seed to the web demos.
    private func seed() {
        let cells: [(String, String)] = [
            ("A1", "15"), ("B1", "3"),  ("C1", "12"), ("D1", "8"),  ("E1", "=SUM(A1:D1)"),
            ("A2", "8"),  ("B2", "14"), ("C2", "7"),  ("D2", "22"), ("E2", "=SUM(A2:D2)"),
            ("A3", "12"), ("B3", "9"),  ("C3", "18"), ("D3", "6"),  ("E3", "=SUM(A3:D3)"),
            ("A4", "4"),  ("B4", "11"), ("C4", "3"),  ("D4", "17"), ("E4", "=SUM(A4:D4)"),
            ("A5", "=SUM(A1:A4)"), ("B5", "=SUM(B1:B4)"), ("C5", "=SUM(C1:C4)"),
            ("D5", "=SUM(D1:D4)"), ("E5", "=SUM(E1:E4)"),
        ]
        for (a, v) in cells { session.setCell(a, v) }
    }

    /// Rebuild the display matrix from the engine's computed values.
    func recompute() {
        var matrix: [[String]] = []
        for r in 0..<rows {
            var row = ["\(r + 1)"]
            for c in 1...cols { row.append(session.display(address(r, c))) }
            matrix.append(row)
        }
        viewportRows = matrix
    }

    /// A1 address of the selected cell (or just the row number for the gutter).
    var selectedAddress: String {
        let c = Int(selectedCol), r = Int(selectedRow)
        guard c >= 1 else { return "\(r + 1)" }
        return address(r, c)
    }

    /// The raw source (formula/literal) of the selected cell, for the bar.
    var selectedRaw: String {
        let c = Int(selectedCol)
        guard c >= 1 else { return "" }
        return session.getRaw(address(Int(selectedRow), c))
    }

    func select(row: Int, col: Int) {
        selectedRow = Double(max(0, min(rows - 1, row)))
        selectedCol = Double(max(1, min(cols, col)))
    }

    /// Set the selected cell from a raw string and recompute everything.
    func setSelected(_ raw: String) {
        let c = Int(selectedCol)
        guard c >= 1 else { return }
        session.setCell(address(Int(selectedRow), c), raw)
        recompute()
    }
}

#if DEBUG
extension SpreadsheetModel {
    /// Test/debug only: the raw value JSON the engine returns for a cell.
    func debugValueJSON(_ a1: String) -> String { session.getValueJSON(a1) }
}
#endif
