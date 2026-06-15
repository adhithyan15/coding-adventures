// InfiniteGrid.swift — a virtualized, effectively-infinite sheet for the SwiftUI
// demo, rendered on the shared Rust engine through the viewport primitive (the
// same `get_window` / `used_range` / `changed_since` the web demo's
// infinite.html uses through WASM).
//
// The sheet is u32 × u32 and sparse; only the VISIBLE window of cells is ever
// built into the view. A two-axis ScrollView holds a clear spacer sized to the
// data extent (so the scrollbars get the right range); a scroll-offset
// preference tells us which rectangle is on screen, and the view asks the engine
// for just that window via `WindowedSheetModel.window(...)`.

import SwiftUI

/// Engine-backed model for the virtualized sheet: seeds a deliberately far-flung,
/// sparse dataset and exposes windowed reads + the data extent.
final class WindowedSheetModel: ObservableObject {
    private let session = SpreadsheetSession()

    /// Cell geometry (points). `InfiniteGridView` uses the same values.
    static let rowH: CGFloat = 22
    static let colW: CGFloat = 80
    static let gutterW: CGFloat = 56
    static let headH: CGFloat = 24

    /// The virtual grid size, derived from the data extent plus a margin so you
    /// can scroll past the data into blank space.
    @Published private(set) var totalRows: UInt32 = 1000
    @Published private(set) var totalCols: UInt32 = 60

    init() {
        seed()
        resize()
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
            ("Z1000", "=SUM(A1:A4)"),                  // 1000 rows down: 39
            ("BA50", "far cell"), ("BB50", "=Z1000*2"), // col 53/54, row 50: 78
        ]
        for (a, v) in cells { session.setCell(a, v) }
    }

    /// Size the virtual grid to the data extent plus a comfortable margin.
    func resize() {
        let u = session.usedRange()
        totalRows = max((u?.maxRow ?? 1) + 200, 1000)
        totalCols = max((u?.maxCol ?? 1) + 30, 60)
    }

    /// Display strings for the inclusive 1-based window. The host clamps these
    /// to the visible area before calling.
    func window(rows: ClosedRange<UInt32>, cols: ClosedRange<UInt32>) -> [[String]] {
        session.window(rows.lowerBound, cols.lowerBound, rows.upperBound, cols.upperBound)
    }

    func columnLetters(_ index: UInt32) -> String { session.columnLetters(index) }
    func raw(_ a1: String) -> String { session.getRaw(a1) }

    /// Write a cell, recompute, resize the extent, and return the cells the edit
    /// dirtied (for an incremental refresh; the view re-reads its window anyway).
    @discardableResult
    func setCell(_ a1: String, _ raw: String) -> (changed: [String], stale: Bool) {
        let rev = session.currentRevision()
        session.setCell(a1, raw)
        resize()
        return session.changedSince(rev)
    }
}

/// The virtualized grid view. Renders only the cells in the current scroll
/// window; frozen column-letter headers + a row-number gutter ride the scroll.
struct InfiniteGridView: View {
    @ObservedObject var model: WindowedSheetModel
    @State private var offset = CGPoint.zero

    private static let rowH = WindowedSheetModel.rowH
    private static let colW = WindowedSheetModel.colW
    private static let gutterW = WindowedSheetModel.gutterW
    private static let headH = WindowedSheetModel.headH
    private static func x(forCol c: UInt32) -> CGFloat { gutterW + (CGFloat(c) - 1) * colW }
    private static func y(forRow r: UInt32) -> CGFloat { headH + (CGFloat(r) - 1) * rowH }

    private let space = "infiniteSheet"

    var body: some View {
        GeometryReader { geo in
            let vis = visible(geo.size)
            let win = model.window(rows: vis.rows, cols: vis.cols)
            ZStack(alignment: .topLeading) {
                ScrollView([.horizontal, .vertical]) {
                    Color.clear
                        .frame(
                            width: Self.x(forCol: model.totalCols + 1),
                            height: Self.y(forRow: model.totalRows + 1)
                        )
                        .background(
                            GeometryReader { inner in
                                Color.clear.preference(
                                    key: OffsetKey.self,
                                    value: inner.frame(in: .named(space)).origin
                                )
                            }
                        )
                        .overlay(alignment: .topLeading) { dataCells(vis: vis, win: win) }
                }
                .coordinateSpace(name: space)
                .onPreferenceChange(OffsetKey.self) { offset = $0 }

                frozenChrome(vis: vis)
            }
        }
        .background(Color(hex: 0x1E1E1E))
    }

    /// The visible window in 1-based cell coordinates, with a small overscan.
    private func visible(_ vp: CGSize) -> (rows: ClosedRange<UInt32>, cols: ClosedRange<UInt32>) {
        let st = -offset.y, sl = -offset.x
        let over = 3
        let fr = clampRow(Int((st / Self.rowH).rounded(.down)) + 1 - over)
        let lr = clampRow(Int(((st + vp.height) / Self.rowH).rounded(.up)) + over)
        let fc = clampCol(Int((sl / Self.colW).rounded(.down)) + 1 - over)
        let lc = clampCol(Int(((sl + vp.width) / Self.colW).rounded(.up)) + over)
        return (fr...max(fr, lr), fc...max(fc, lc))
    }

    private func clampRow(_ v: Int) -> UInt32 { UInt32(min(max(1, v), Int(model.totalRows))) }
    private func clampCol(_ v: Int) -> UInt32 { UInt32(min(max(1, v), Int(model.totalCols))) }

    /// The data cells of the visible window, absolutely positioned in grid space.
    private func dataCells(
        vis: (rows: ClosedRange<UInt32>, cols: ClosedRange<UInt32>), win: [[String]]
    ) -> some View {
        ZStack(alignment: .topLeading) {
            ForEach(Array(vis.rows), id: \.self) { r in
                ForEach(Array(vis.cols), id: \.self) { c in
                    let ri = Int(r - vis.rows.lowerBound)
                    let ci = Int(c - vis.cols.lowerBound)
                    let text = (ri < win.count && ci < win[ri].count) ? win[ri][ci] : ""
                    Text(text)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundColor(Color(hex: 0xCCCCCC))
                        .frame(width: Self.colW - 1, height: Self.rowH - 1, alignment: .trailing)
                        .padding(.trailing, 4)
                        .border(Color(hex: 0x3F3F46), width: 0.5)
                        .offset(x: Self.x(forCol: c), y: Self.y(forRow: r))
                }
            }
        }
    }

    /// Frozen column-letter header row + row-number gutter; each follows the
    /// scroll on its cross axis only (offset added back so it tracks the data).
    private func frozenChrome(vis: (rows: ClosedRange<UInt32>, cols: ClosedRange<UInt32>)) -> some View {
        ZStack(alignment: .topLeading) {
            ForEach(Array(vis.cols), id: \.self) { c in
                Text(model.columnLetters(c))
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundColor(Color(hex: 0x9D9D9D))
                    .frame(width: Self.colW - 1, height: Self.headH, alignment: .center)
                    .background(Color(hex: 0x2D2D30))
                    .offset(x: Self.x(forCol: c) + offset.x, y: 0)
            }
            ForEach(Array(vis.rows), id: \.self) { r in
                Text("\(r)")
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundColor(Color(hex: 0x9D9D9D))
                    .frame(width: Self.gutterW, height: Self.rowH - 1, alignment: .center)
                    .background(Color(hex: 0x2D2D30))
                    .offset(x: 0, y: Self.y(forRow: r) + offset.y)
            }
            Rectangle().fill(Color(hex: 0x2D2D30))
                .frame(width: Self.gutterW, height: Self.headH)
        }
    }
}

/// Carries the scrolled content's origin up to the view via a preference.
private struct OffsetKey: PreferenceKey {
    static var defaultValue: CGPoint = .zero
    static func reduce(value: inout CGPoint, nextValue: () -> CGPoint) { value = nextValue() }
}
