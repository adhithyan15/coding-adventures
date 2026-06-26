// InfiniteGrid.swift — a virtualized, effectively-infinite, EDITABLE sheet for
// the SwiftUI demo, rendered on the shared Rust engine through the viewport
// primitive (the same `get_display_window` / `used_range` / `changed_since` the web
// demo's infinite.html uses through WASM).
//
// The sheet is u32 × u32 and sparse. Virtualization is delegated to SwiftUI's
// `LazyVStack`: it instantiates only the rows currently on screen, so scrolling
// thousands of rows builds only a screen's worth of cells at a time — and each
// visible row fetches just its own values from the engine
// (`WindowedSheetModel.rowCells`). The column-letter header is pinned to the top
// (`pinnedViews: [.sectionHeaders]`), the row-number gutter is the first cell of
// every row, and tapping a cell selects it for editing in the formula bar.

import SwiftUI

/// Engine-backed model for the virtualized sheet: seeds a deliberately far-flung,
/// sparse dataset, exposes windowed reads + the data extent, and tracks the
/// selection + formula-bar text so the grid is editable.
final class WindowedSheetModel: ObservableObject {
    private let session = SpreadsheetSession()

    /// The virtual grid size, derived from the data extent plus a margin so you
    /// can scroll past the data into blank space.
    @Published private(set) var totalRows: UInt32 = 1000
    @Published private(set) var totalCols: UInt32 = 60
    /// Bumped on every edit so the visible rows re-fetch from the engine.
    @Published private(set) var revision: Int = 0
    /// Selection in 1-based grid coordinates (row ≥ 1, col ≥ 1 = column A).
    @Published private(set) var selectedRow: Int = 1
    @Published private(set) var selectedCol: Int = 1
    /// The formula bar's text — the selected cell's source, edited in place.
    @Published var formulaText: String = ""
    /// The find/replace boxes' text + a short status (match/replace count) shown
    /// in the footer. The query searches every cell's SOURCE (case-insensitive).
    @Published var findText: String = ""
    @Published var replaceText: String = ""
    @Published private(set) var findStatus: String = ""

    /// Column letters cached once per resize (`columnLetters` is a pure engine
    /// call, but the header would otherwise re-issue it for every column on
    /// every recompose).
    private var letterCache: [String] = []

    init() {
        seed()
        resize()
        select(row: 1, col: 1)
    }

    /// The classic cross-footing budget PLUS far-flung cells (a formula at
    /// Z1000, a couple near BA50/BB50) to prove the sheet is sparse and
    /// unbounded — identical seed to the web demo's infinite.html.
    private func seed() {
        let cells: [(String, String)] = [
            ("A1", "15"), ("B1", "3"), ("C1", "12"), ("D1", "8"), ("E1", "=SUM(A1:D1)"),
            ("A2", "8"), ("B2", "14"), ("C2", "7"), ("D2", "22"), ("E2", "=SUM(A2:D2)"),
            ("A3", "12"), ("B3", "9"), ("C3", "18"), ("D3", "6"), ("E3", "=SUM(A3:D3)"),
            ("A4", "4"), ("B4", "11"), ("C4", "3"), ("D4", "17"), ("E4", "=SUM(A4:D4)"),
            ("A5", "=SUM(A1:A4)"), ("B5", "=SUM(B1:B4)"), ("C5", "=SUM(C1:C4)"),
            ("D5", "=SUM(D1:D4)"), ("E5", "=SUM(E1:E4)"),
            ("Z1000", "=SUM(A1:A4)"),                   // 1000 rows down: 39
            ("BA50", "far cell"), ("BB50", "=Z1000*2"), // col 53/54, row 50: 78
        ]
        for (a, v) in cells { session.setCell(a, v) }

        // Attach Excel-style format codes so the engine's display path is visible
        // in the windowed view (which renders via sc_get_display_window): the
        // cross-foot totals read with thousands grouping + two decimals, and the
        // far-flung Z1000 total as a percent. Values are unchanged — only how the
        // display strings render. Identical to the web/Qt/Flutter/Compose/XAML demos.
        let formats: [(String, String)] = [
            ("E1", "#,##0.00"), ("E2", "#,##0.00"), ("E3", "#,##0.00"),
            ("E4", "#,##0.00"), ("E5", "#,##0.00"),
            ("A5", "#,##0.00"), ("B5", "#,##0.00"), ("C5", "#,##0.00"), ("D5", "#,##0.00"),
            ("Z1000", "0.0%"), // 39 → "3900.0%": proves the format applies far off-origin
        ]
        for (a, code) in formats { session.setFormat(a, code) }
    }

    /// Size the virtual grid to the data extent plus a comfortable margin, and
    /// refresh the column-letter cache.
    func resize() {
        let u = session.usedRange()
        totalRows = max((u?.maxRow ?? 1) + 200, 1000)
        totalCols = max((u?.maxCol ?? 1) + 30, 60)
        letterCache = (1...Int(totalCols)).map { session.columnLetters(UInt32($0)) }
    }

    // ── Reads ────────────────────────────────────────────────────────

    /// Display strings for the inclusive 1-based window. (Kept for the headless
    /// `WindowedModelTests`; the view uses `rowCells`.)
    func window(rows: ClosedRange<UInt32>, cols: ClosedRange<UInt32>) -> [[String]] {
        session.window(rows.lowerBound, cols.lowerBound, rows.upperBound, cols.upperBound)
    }

    /// The display strings for one full row (all columns) — what a visible row
    /// renders (display strings, already rendered through each cell.s format code).
    /// One engine `get_display_window` call per on-screen row.
    func rowCells(_ row: Int) -> [String] {
        let w = session.window(UInt32(row), 1, UInt32(row), totalCols)
        return w.first ?? Array(repeating: "", count: Int(totalCols))
    }

    /// Column letters for a 1-based index (`1` → `"A"`, `27` → `"AA"`).
    func columnLetters(_ index: UInt32) -> String {
        let i = Int(index)
        return (i >= 1 && i <= letterCache.count) ? letterCache[i - 1]
            : session.columnLetters(index)
    }

    /// The A1 address for 1-based grid `(row, col)`.
    func address(_ row: Int, _ col: Int) -> String { "\(columnLetters(UInt32(col)))\(row)" }

    // ── Selection + editing ──────────────────────────────────────────

    func select(row: Int, col: Int) {
        selectedRow = max(1, min(Int(totalRows), row))
        selectedCol = max(1, min(Int(totalCols), col))
        formulaText = session.getRaw(address(selectedRow, selectedCol))
    }

    /// Commit the formula bar into the selected cell: write through to the
    /// engine, resize the extent, and bump `revision` so the visible rows
    /// re-fetch (every dependent cell, however far away, recomputes).
    func commitFormula() {
        session.setCell(address(selectedRow, selectedCol), formulaText)
        resize()
        formulaText = session.getRaw(address(selectedRow, selectedCol))
        revision += 1
    }

    /// Drag-fill: replicate the selected cell into the `rows` rows below it. The
    /// engine shifts each copy's relative references (`=A1`→`=A2`, …), pins
    /// absolute (`$`) refs, carries the format, and recomputes every dependent;
    /// then resize the extent and bump `revision` so the visible rows re-fetch.
    /// The SwiftUI sibling of the Flutter/Compose/XAML `fillDown` and the Qt
    /// "Fill ↓ 10" button. (Int is 64-bit here, so `selectedRow + rows` over the
    /// u32-bounded extent cannot overflow.)
    func fillDown(_ rows: Int) {
        let src = address(selectedRow, selectedCol)
        let first = address(selectedRow + 1, selectedCol)
        let last = address(selectedRow + rows, selectedCol)
        session.fill(src, first, last)
        resize()
        revision += 1
    }

    /// Structural edits: insert / delete the selected cell's row or column. The
    /// engine shifts every formula reference at or after the band (a reference
    /// whose whole band is deleted becomes `#REF!`) and recomputes; resize the
    /// extent and bump `revision` so the visible rows re-fetch. Operate on a
    /// single row/column at the cursor. The SwiftUI sibling of the Qt/Flutter/
    /// Compose/XAML "+ Row / − Row / + Col / − Col" controls.
    func insertRow() { session.insertRows(selectedRow, 1); afterStructural() }
    func deleteRow() { session.deleteRows(selectedRow, 1); afterStructural() }
    func insertCol() { session.insertCols(selectedCol, 1); afterStructural() }
    func deleteCol() { session.deleteCols(selectedCol, 1); afterStructural() }

    /// Shared tail for a structural edit: regrow the extent, re-read the (now
    /// possibly moved/blanked) selected cell's source into the formula bar, and
    /// bump `revision` so the visible rows re-fetch.
    private func afterStructural() {
        resize()
        formulaText = session.getRaw(address(selectedRow, selectedCol))
        revision += 1
    }

    /// Number format: apply an Excel-style format code to the selected cell's
    /// DISPLAY only — the stored value is untouched, so `getRaw` still returns
    /// the original source and dependent formulas keep computing on the real
    /// number. An empty code clears the format (back to General). The engine's
    /// display window then renders the cell through the code (`1234` + `#,##0.00`
    /// → `"1,234.00"`). Bump `revision` so the visible rows re-fetch. The SwiftUI
    /// sibling of the web demo's "Format" group and the Qt/Flutter/Compose/XAML
    /// `applyFormat`.
    func applyFormat(_ code: String) {
        session.setFormat(address(selectedRow, selectedCol), code)
        revision += 1
    }

    /// Range sort: reorder the rows of the seeded budget block A1:E4 by the
    /// SELECTED column (clamped into the block's columns A..E = 1...5), ascending
    /// or descending. Each row moves as a record; the E-column SUM formulas travel
    /// with their row (the engine shifts their refs), so every total stays correct.
    /// Returns false for a no-op (already sorted / bad args). Bumps `revision` so
    /// the visible rows re-fetch. The SwiftUI sibling of the web/Qt/Flutter/Compose/
    /// XAML "Sort" group.
    @discardableResult
    func sortBlock(_ ascending: Bool) -> Bool {
        let keyCol = UInt32(min(5, max(1, selectedCol)))
        let ok = session.sortRange("A1", "E4", keyCol, ascending)
        revision += 1
        return ok
    }

    /// Find: every cell whose SOURCE contains `query` (case-insensitive), as A1
    /// addresses in row-major order. The SwiftUI sibling of the web demo's findAll
    /// and the Qt/Flutter/Compose/XAML ports — it searches formula text, so "=SUM"
    /// or a literal like "15" both hit. An empty query returns no matches.
    func findAll(_ query: String) -> [String] { session.findAll(query, true, false) }

    /// Replace: rewrite `query` → `replacement` in every cell's source
    /// (case-insensitive), returning the number of cells changed. The engine
    /// re-parses each rewrite (so a formula stays live, a literal stays typed) and
    /// recomputes dependents; resize the extent, re-sync the formula bar, and bump
    /// `revision` so the visible rows re-fetch.
    @discardableResult
    func replaceAll(_ query: String, _ replacement: String) -> Int {
        let n = session.replaceAll(query, replacement, false)
        resize()
        formulaText = session.getRaw(address(selectedRow, selectedCol))
        revision += 1
        return n
    }

    /// Find button: locate the matches for `findText`, jump the selection to the
    /// first hit, and set `findStatus` to the match count for the footer. An empty
    /// query clears the status and does nothing.
    func runFind() {
        let hits = findAll(findText)
        if let first = hits.first { selectA1(first) }
        if findText.isEmpty {
            findStatus = ""
        } else if hits.isEmpty {
            findStatus = "no match"
        } else {
            findStatus = "\(hits.count) match\(hits.count == 1 ? "" : "es")"
        }
    }

    /// Replace button: rewrite `findText` → `replaceText` everywhere and recompute;
    /// `findStatus` shows how many cells changed.
    func runReplace() {
        let n = replaceAll(findText, replaceText)
        findStatus = "\(n) replaced"
    }

    /// Move the selection onto an A1 address (e.g. a find hit like "Z1000"),
    /// parsing the column letters (past Z) and row digits and clamping into the
    /// grid via `select`. A no-op on a malformed address.
    func selectA1(_ a1: String) {
        let trimmed = a1.trimmingCharacters(in: .whitespaces)
        var letters = "", digits = ""
        for ch in trimmed {
            if ch.isLetter, digits.isEmpty { letters.append(ch) }
            else if ch.isNumber { digits.append(ch) }
            else { return } // malformed: bail out
        }
        // A u32-bounded sheet's widest column ("FXSHRXW", 2^32) is 7 letters, so a
        // longer run is malformed — bail before the `col * 26` accumulate (which
        // would otherwise trap on Int overflow for a ~13+ letter run; `select`
        // clamps the result, but only after the multiply).
        guard !letters.isEmpty, letters.count <= 7, let row = Int(digits) else { return }
        var col = 0
        for ch in letters.uppercased() {
            guard let v = ch.asciiValue, v >= 65, v <= 90 else { return }
            col = col * 26 + Int(v - 64) // 'A' = 1
        }
        select(row: row, col: col)
    }

    /// Clipboard: copy/cut the selected cell, then paste it at the selection. The
    /// engine shifts the pasted formula's relative references by the
    /// destination's offset, pins absolute (`$`) refs, carries the format; a cut
    /// clears the source on paste. `pasteCell` returns `false` (a no-op) for an
    /// empty clipboard, and resizes the extent + bumps `revision` on success.
    func copyCell() {
        let a = address(selectedRow, selectedCol)
        session.copy(a, a)
    }
    func cutCell() {
        let a = address(selectedRow, selectedCol)
        session.cut(a, a)
    }
    @discardableResult
    func pasteCell() -> Bool {
        let ok = session.paste(address(selectedRow, selectedCol))
        if ok {
            resize()
            revision += 1
        }
        return ok
    }

    /// Save / load: serialize the whole workbook to a JSON document, and restore
    /// it. The document stores only the source + formats — computed values
    /// recompute on load, so a loaded formula stays live. `loadBook` returns
    /// `false` (workbook untouched) for malformed input; on success it resizes
    /// the extent, refreshes the formula bar, and bumps `revision` so the view
    /// re-reads.
    func saveBook() -> String {
        session.serialize()
    }
    @discardableResult
    func loadBook(_ data: String) -> Bool {
        let ok = session.deserialize(data)
        if ok {
            resize()
            formulaText = session.getRaw(address(selectedRow, selectedCol))
            revision += 1
        }
        return ok
    }

    /// Undo / redo: walk the engine's snapshot history. On success the extent
    /// resizes, the formula bar refreshes, and `revision` bumps (which re-renders
    /// the SwiftUI view, re-evaluating the canUndo/canRedo button gates); a
    /// restored formula stays live.
    func canUndo() -> Bool { session.canUndo() }
    func canRedo() -> Bool { session.canRedo() }
    @discardableResult
    func undoEdit() -> Bool {
        let ok = session.undo()
        if ok {
            resize()
            formulaText = session.getRaw(address(selectedRow, selectedCol))
            revision += 1
        }
        return ok
    }
    @discardableResult
    func redoEdit() -> Bool {
        let ok = session.redo()
        if ok {
            resize()
            formulaText = session.getRaw(address(selectedRow, selectedCol))
            revision += 1
        }
        return ok
    }

    /// Write `raw` into an explicit A1 cell and return the cells it dirtied.
    /// (Kept for the headless `WindowedModelTests`.)
    @discardableResult
    func setCell(_ a1: String, _ raw: String) -> (changed: [String], stale: Bool) {
        let rev = session.currentRevision()
        session.setCell(a1, raw)
        resize()
        revision += 1
        return session.changedSince(rev)
    }
}

/// The virtualized, editable grid view.
struct InfiniteGridView: View {
    @ObservedObject var model: WindowedSheetModel

    // In-memory "saved file" slot for the Save / Load buttons: Save stows the
    // serialized workbook here, Load restores from it. (A real app would write it
    // to a file; the demo keeps the round trip self-contained.)
    @State private var savedSnapshot = ""
    // Drives the formula field's accent focus ring.
    @FocusState private var formulaFocused: Bool

    // Roomier geometry, to match the web reference.
    static let rowH: CGFloat = 26
    static let colW: CGFloat = 92
    static let gutterW: CGFloat = 64
    static let headH: CGFloat = 28

    // ── Design tokens ──────────────────────────────────────────────────
    // Mirror demo/visicalc-html/infinite.html's palette so every VisiCalc backend
    // reads as one considered surface (dark modern spreadsheet). Same token set as
    // the Qt / Flutter / Compose / XAML ports.
    private let cBg = Color(hex: 0x16181D) // app / base cell
    private let cPanel = Color(hex: 0x1B1E24) // toolbar + zebra band
    private let cSurface = Color(hex: 0x21252C) // pill
    private let cField = Color(hex: 0x0F1115) // formula input well
    private let line = Color(hex: 0x2C313A) // hairline borders
    private let lineStrong = Color(hex: 0x3A404B) // control borders
    private let headBg = Color(hex: 0x20242B) // row/col headers
    private let headSel = Color(hex: 0x2B3340) // header of selected row/col
    private let ink = Color(hex: 0xE8EAED) // primary text
    private let muted = Color(hex: 0x9AA3B2) // labels, headers
    private let accent = Color(hex: 0x4AA3FF) // selection + focus
    private let sel = Color(hex: 0x21344A) // selected-cell fill

    var body: some View {
        VStack(spacing: 0) {
            formulaBar
            ScrollView([.vertical, .horizontal]) {
                LazyVStack(alignment: .leading, spacing: 0, pinnedViews: [.sectionHeaders]) {
                    Section {
                        // LazyVStack realises only the rows on screen, so this
                        // 1...totalRows loop never builds the whole sheet.
                        ForEach(1...Int(model.totalRows), id: \.self) { r in
                            rowView(r)
                        }
                    } header: {
                        headerRow
                    }
                }
            }
            .background(cBg)
            statusBar
        }
        .background(cBg)
    }

    // Hairline-separated footer: the live virtual-grid size + per-edit revision
    // clock (mirrors the web/Qt/Flutter/Compose/XAML status lines).
    private var statusBar: some View {
        VStack(spacing: 0) {
            Rectangle().fill(line).frame(height: 1)
            HStack {
                Text("Virtual grid: \(model.totalRows) rows × \(model.totalCols) cols  ·  revision \(model.revision)"
                    + (model.findStatus.isEmpty ? "" : "  ·  \(model.findStatus)"))
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundColor(muted)
                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
        }
    }

    // The editable formula bar for the selected cell: a panel holding the address
    // pill, an `fx` marker, the source line (with an accent focus ring), and
    // segmented button groups (drag-fill · clipboard · file · history) divided by
    // thin rules.
    private var formulaBar: some View {
        HStack(spacing: 6) {
            // Address pill.
            Text(model.address(model.selectedRow, model.selectedCol))
                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                .foregroundColor(ink)
                .frame(width: 46, height: 30)
                .background(cSurface)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(lineStrong, lineWidth: 1))
                .cornerRadius(5)
            // fx marker.
            Text("fx")
                .font(.system(size: 12, design: .monospaced)).italic()
                .foregroundColor(muted)
            // Formula field — accent focus ring on edit.
            TextField("value or =SUM(A1:A4)", text: $model.formulaText)
                .textFieldStyle(.plain)
                .font(.system(size: 13, design: .monospaced))
                .foregroundColor(ink)
                .focused($formulaFocused)
                .padding(.horizontal, 8)
                .frame(height: 30)
                .frame(maxWidth: .infinity)
                .background(cField)
                .overlay(RoundedRectangle(cornerRadius: 5)
                    .stroke(formulaFocused ? accent : lineStrong, lineWidth: formulaFocused ? 2 : 1))
                .cornerRadius(5)
                .onSubmit { model.commitFormula() }
            // ── Drag-fill ──
            Button("↓ Fill 10") { model.fillDown(10) }
                .help("Replicate the selected cell into the 10 rows below it")
            toolSep
            // ── Clipboard ──
            Button("Copy") { model.copyCell() }
                .help("Copy the selected cell to the clipboard")
            Button("Cut") { model.cutCell() }
                .help("Cut the selected cell (cleared when you paste)")
            Button("Paste") { model.pasteCell() }
                .help("Paste the clipboard at the selected cell, shifting relative references")
            toolSep
            // ── File (save / load) ──
            Button("Save") { savedSnapshot = model.saveBook() }
                .help("Serialize the whole workbook to memory")
            Button("Load") { if !savedSnapshot.isEmpty { model.loadBook(savedSnapshot) } }
                .disabled(savedSnapshot.isEmpty)
                .help("Restore the workbook from the last save")
            toolSep
            // ── Structure (insert / delete the selected row or column) ──
            Button("+ Row") { model.insertRow() }
                .help("Insert a row above the selected cell (references shift down)")
            Button("− Row") { model.deleteRow() }
                .help("Delete the selected cell's row (references shift up; refs into it become #REF!)")
            Button("+ Col") { model.insertCol() }
                .help("Insert a column left of the selected cell (references shift right)")
            Button("− Col") { model.deleteCol() }
                .help("Delete the selected cell's column (references shift left; refs into it become #REF!)")
            toolSep
            // ── Format (apply an Excel-style number-format code to the selected
            // cell's DISPLAY; the stored value is untouched). Fixed codes:
            // thousands+2dp, percent, currency, and General (clears).
            Button(".00") { model.applyFormat("#,##0.00") }
                .help("Format the selected cell as 1,234.00 (display only)")
            Button("%") { model.applyFormat("0.0%") }
                .help("Format the selected cell as a percentage (display only)")
            Button("$") { model.applyFormat("$#,##0.00") }
                .help("Format the selected cell as currency (display only)")
            Button("Gen") { model.applyFormat("") }
                .help("Clear the number format (back to General)")
            toolSep
            // ── Sort (reorder the budget block A1:E4 by the selected column) ──
            Button("▲ Sort") { model.sortBlock(true) }
                .help("Sort the budget block A1:E4 by the selected column, ascending (rows move as records; formulas track)")
            Button("▼ Sort") { model.sortBlock(false) }
                .help("Sort the budget block A1:E4 by the selected column, descending")
            toolSep
            // ── History (undo / redo). The buttons gate off canUndo/canRedo,
            // re-evaluated whenever `revision` (a @Published) bumps after an edit.
            Button("↶ Undo") { model.undoEdit() }
                .disabled(!model.canUndo())
                .help("Undo the last edit")
            Button("↷ Redo") { model.redoEdit() }
                .disabled(!model.canRedo())
                .help("Redo the last undone edit")
            toolSep
            // ── Find / replace (search cell sources; rewrite matches) ──
            TextField("find", text: $model.findText)
                .textFieldStyle(.plain)
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(ink)
                .padding(.horizontal, 8)
                .frame(width: 96, height: 30)
                .background(cField)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(lineStrong, lineWidth: 1))
                .cornerRadius(5)
                .onSubmit { model.runFind() }
            Button("Find") { model.runFind() }
                .help("Find every cell whose source contains the query (case-insensitive) and jump to the first hit")
            TextField("replace", text: $model.replaceText)
                .textFieldStyle(.plain)
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(ink)
                .padding(.horizontal, 8)
                .frame(width: 96, height: 30)
                .background(cField)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(lineStrong, lineWidth: 1))
                .cornerRadius(5)
                .onSubmit { model.runReplace() }
            Button("Replace") { model.runReplace() }
                .help("Replace the query with the replacement in every cell's source and recompute")
        }
        .buttonStyle(ChipButtonStyle())
        .padding(8)
        .background(cPanel)
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(line, lineWidth: 1))
        .cornerRadius(8)
        .padding(.horizontal, 10)
        .padding(.top, 10)
        .padding(.bottom, 6)
    }

    // A thin vertical rule between toolbar button groups.
    private var toolSep: some View {
        Rectangle().fill(line).frame(width: 1, height: 22)
    }

    // Frozen column-letter header (pinned to the top of the scroll view). The
    // selected column's header tints to the accent.
    private var headerRow: some View {
        HStack(spacing: 0) {
            headerCell("", width: Self.gutterW, selected: false)
            ForEach(1...Int(model.totalCols), id: \.self) { c in
                headerCell(model.columnLetters(UInt32(c)), width: Self.colW,
                           selected: c == model.selectedCol)
            }
        }
    }

    private func headerCell(_ text: String, width: CGFloat, selected: Bool) -> some View {
        Text(text)
            .font(.system(size: 11, weight: .semibold, design: .monospaced))
            .foregroundColor(selected ? accent : muted)
            .frame(width: width, height: Self.headH)
            .background(selected ? headSel : headBg)
            .border(line, width: 0.5)
    }

    private func rowView(_ r: Int) -> some View {
        // One engine read for the whole row; re-fetched when `revision` changes
        // (this view observes the model, so an edit re-runs it).
        let cells = model.rowCells(r)
        let rowSelected = r == model.selectedRow
        return HStack(spacing: 0) {
            // Gutter — the selected row's label tints to the accent.
            Text("\(r)")
                .font(.system(size: 11, weight: .semibold, design: .monospaced))
                .foregroundColor(rowSelected ? accent : muted)
                .frame(width: Self.gutterW, height: Self.rowH)
                .background(rowSelected ? headSel : headBg)
                .border(line, width: 0.5)
            ForEach(1...Int(model.totalCols), id: \.self) { c in
                dataCell(text: c - 1 < cells.count ? cells[c - 1] : "", r: r, c: c)
            }
        }
    }

    private func dataCell(text: String, r: Int, c: Int) -> some View {
        let selected = r == model.selectedRow && c == model.selectedCol
        // Zebra: even rows take the panel tint, odd rows the base cell color.
        let band = r % 2 == 0 ? cPanel : cBg
        return Text(text)
            .font(.system(size: 12, weight: selected ? .semibold : .regular, design: .monospaced))
            .foregroundColor(selected ? .white : ink)
            .frame(width: Self.colW, height: Self.rowH, alignment: .trailing)
            .padding(.trailing, 6)
            .background(selected ? sel : band)
            .border(selected ? accent : line, width: selected ? 2 : 0.5)
            .contentShape(Rectangle())
            .onTapGesture { model.select(row: r, col: c) }
    }
}

/// A compact, modern toolbar button — a rounded chip with hover / pressed /
/// disabled states, the SwiftUI analog of the web demo's segmented controls and
/// the Qt port's `component ToolButton`.
struct ChipButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        ChipLabel(configuration: configuration)
    }

    // A nested view so the chip can hold @State for hover and read @Environment
    // for the enabled flag (a ButtonStyle struct itself can't observe either).
    private struct ChipLabel: View {
        let configuration: Configuration
        @Environment(\.isEnabled) private var isEnabled
        @State private var hover = false

        var body: some View {
            let bg: Color = configuration.isPressed ? Color(hex: 0x14171C)
                : (hover && isEnabled ? Color(hex: 0x2B313A) : Color(hex: 0x21252C))
            let fg: Color = !isEnabled ? Color(hex: 0x9AA3B2)
                : (hover ? .white : Color(hex: 0xE8EAED))
            return configuration.label
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(fg)
                .padding(.horizontal, 11)
                .frame(height: 30)
                .background(bg)
                .overlay(RoundedRectangle(cornerRadius: 5).stroke(Color(hex: 0x3A404B), lineWidth: 1))
                .cornerRadius(5)
                .opacity(isEnabled ? 1 : 0.6)
                .onHover { hover = $0 }
        }
    }
}
