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

    static let rowH: CGFloat = 22
    static let colW: CGFloat = 80
    static let gutterW: CGFloat = 56
    static let headH: CGFloat = 24

    private let line = Color(hex: 0x3F3F46)
    private let headBg = Color(hex: 0x2D2D30)

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
            .background(Color(hex: 0x1E1E1E))
        }
    }

    // The editable formula bar for the selected cell.
    private var formulaBar: some View {
        HStack(spacing: 8) {
            Text(model.address(model.selectedRow, model.selectedCol))
                .font(.system(size: 11, design: .monospaced))
                .foregroundColor(Color(hex: 0x9D9D9D))
                .frame(width: 56, alignment: .leading)
            TextField("value or =SUM(A1:A4)", text: $model.formulaText)
                .textFieldStyle(.plain)
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(Color(hex: 0xCCCCCC))
                .padding(6)
                .background(Color(hex: 0x121212))
                .cornerRadius(3)
                .onSubmit { model.commitFormula() }
            // Drag-fill: replicate the selected cell into the 10 rows below it.
            Button("Fill ↓ 10") { model.fillDown(10) }
                .font(.system(size: 11, design: .monospaced))
                .help("Replicate the selected cell into the 10 rows below it")
            // Clipboard: copy/cut the selected cell, paste at the selection. The
            // engine shifts the pasted formula's relative refs by the offset.
            Button("Copy") { model.copyCell() }
                .font(.system(size: 11, design: .monospaced))
                .help("Copy the selected cell to the clipboard")
            Button("Cut") { model.cutCell() }
                .font(.system(size: 11, design: .monospaced))
                .help("Cut the selected cell (cleared when you paste)")
            Button("Paste") { model.pasteCell() }
                .font(.system(size: 11, design: .monospaced))
                .help("Paste the clipboard at the selected cell, shifting relative references")
        }
        .padding(.bottom, 8)
    }

    // Frozen column-letter header (pinned to the top of the scroll view).
    private var headerRow: some View {
        HStack(spacing: 0) {
            headerCell("", width: Self.gutterW)
            ForEach(1...Int(model.totalCols), id: \.self) { c in
                headerCell(model.columnLetters(UInt32(c)), width: Self.colW)
            }
        }
    }

    private func headerCell(_ text: String, width: CGFloat) -> some View {
        Text(text)
            .font(.system(size: 12, design: .monospaced))
            .foregroundColor(Color(hex: 0x9D9D9D))
            .frame(width: width, height: Self.headH)
            .background(headBg)
            .border(line, width: 0.5)
    }

    private func rowView(_ r: Int) -> some View {
        // One engine read for the whole row; re-fetched when `revision` changes
        // (this view observes the model, so an edit re-runs it).
        let cells = model.rowCells(r)
        return HStack(spacing: 0) {
            Text("\(r)")
                .font(.system(size: 12, design: .monospaced))
                .foregroundColor(Color(hex: 0x9D9D9D))
                .frame(width: Self.gutterW, height: Self.rowH)
                .background(headBg)
                .border(line, width: 0.5)
            ForEach(1...Int(model.totalCols), id: \.self) { c in
                dataCell(text: c - 1 < cells.count ? cells[c - 1] : "", r: r, c: c)
            }
        }
    }

    private func dataCell(text: String, r: Int, c: Int) -> some View {
        let selected = r == model.selectedRow && c == model.selectedCol
        return Text(text)
            .font(.system(size: 12, design: .monospaced))
            .foregroundColor(selected ? .white : Color(hex: 0xCCCCCC))
            .frame(width: Self.colW, height: Self.rowH, alignment: .trailing)
            .padding(.trailing, 4)
            .background(selected ? Color(hex: 0x264F78) : Color(hex: 0x1E1E1E))
            .border(selected ? Color(hex: 0x007ACC) : line, width: selected ? 1 : 0.5)
            .contentShape(Rectangle())
            .onTapGesture { model.select(row: r, col: c) }
    }
}
