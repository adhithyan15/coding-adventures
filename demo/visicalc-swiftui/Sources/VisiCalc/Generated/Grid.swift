// Grid.swift — hand-written placeholder (VC2-swiftui).
//
// The mosaic-emit-swiftui pipeline doesn't yet support the `Grid`
// built-in primitive — only the React emitter does. Until the
// SwiftUI Grid emitter lands, this file is a HAND-WRITTEN
// approximation matching Grid.dark.msl's palette so the demo
// visually matches VC2-html / VC2-webcomp / VC2-flutter / VC2-qt.
//
// When the SwiftUI Grid emitter lands, replace this file with the
// auto-generated output (build.sh will overwrite it).

import SwiftUI

struct GridView: View {
    let columnHeaders: [String]
    let viewportRows: [[String]]
    let selectedRow: Int
    let selectedCol: Int
    let onTapCell: (Int, Int) -> Void

    var body: some View {
        VStack(spacing: 0) {
            HeaderRow(headers: columnHeaders)
            ForEach(viewportRows.indices, id: \.self) { r in
                DataRow(
                    rowIndex: r,
                    cells: viewportRows[r],
                    isEven: r.isMultiple(of: 2),
                    selectedCol: r == selectedRow ? selectedCol : -1,
                    onTap: { c in onTapCell(r, c) }
                )
            }
        }
        .border(Color(hex: 0x3F3F46), width: 1)
        .background(Color(hex: 0x1E1E1E))
    }
}

private struct HeaderRow: View {
    let headers: [String]

    var body: some View {
        HStack(spacing: 0) {
            HeaderCell(label: "")
            ForEach(headers, id: \.self) { h in
                HeaderCell(label: h)
            }
        }
        .frame(height: 24)
    }
}

private struct HeaderCell: View {
    let label: String

    var body: some View {
        Text(label)
            .font(.system(size: 12, design: .monospaced))
            .foregroundColor(Color(hex: 0x9D9D9D))
            .frame(width: 96, height: 24)
            .background(Color(hex: 0x2D2D30))
            .border(Color(hex: 0x3F3F46), width: 1)
    }
}

private struct DataRow: View {
    let rowIndex: Int
    let cells: [String]
    let isEven: Bool
    let selectedCol: Int
    let onTap: (Int) -> Void

    var body: some View {
        HStack(spacing: 0) {
            HeaderCell(label: "\(rowIndex + 1)")
            ForEach(cells.indices, id: \.self) { c in
                DataCell(
                    text: cells[c],
                    isSelected: c == selectedCol,
                    onTap: { onTap(c) }
                )
            }
        }
        .frame(height: 22)
        .background(isEven ? Color(hex: 0x1E1E1E) : Color(hex: 0x252526))
    }
}

private struct DataCell: View {
    let text: String
    let isSelected: Bool
    let onTap: () -> Void

    var body: some View {
        Text(text)
            .font(.system(size: 12, design: .monospaced))
            .foregroundColor(isSelected ? .white : Color(hex: 0xCCCCCC))
            .frame(width: 96, height: 22, alignment: .trailing)
            .padding(.trailing, 4)
            .background(isSelected ? Color(hex: 0x264F78) : Color.clear)
            .border(isSelected ? Color(hex: 0x007ACC) : Color(hex: 0x3F3F46), width: 1)
            .onTapGesture(perform: onTap)
    }
}
