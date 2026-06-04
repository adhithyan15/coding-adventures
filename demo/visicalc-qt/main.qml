// main.qml — VisiCalc Qt host (VC2-qt).
//
// Mounts the auto-generated `FormulaBar` and `Grid` components from
// build/.  Both come from `bash scripts/build.sh` which runs
// `mosaic-compile --backend qt` against the shared
// `demo/visicalc/mosaic/{FormulaBar,Grid}.{mil,desktop.mll,dark.msl}`
// triples.  Grid.desktop.mll is a UI34 `pkg::mosaic-pkg-grid::Grid`
// one-liner — the QML component you see below is the package's
// authoritative Grid composition lowered to QtQuick by the Qt
// emitter.  No hand-written widgets in this file.
//
// Hard-coded 5×5 sample spreadsheet matches the data in every other
// VC2-* demo so all five renders look visually identical.
//
// Run:
//   qml main.qml          # one-shot QML viewer (Qt 6)
//   # or via CMake:
//   cmake -B build && cmake --build build && ./build/visicalc-qt

import QtQuick 2.15
import QtQuick.Window 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

// FormulaBar comes from the local `qml/` module, whose qmldir
// references the auto-generated FormulaBar.qml in `build/`. We
// split tracked manifest (qml/qmldir) from generated artifact
// (build/FormulaBar.qml) because the repo's top-level .gitignore
// excludes any directory named build/.
import "qml"

// `id: root` gives the children a stable handle for property and
// function references like `root.sampleRows` and `root.cellAddress()`.
// Without this id the references would resolve as undefined (QML's
// type lookup finds the `Window` *type* but there is no implicit
// `window` identifier — that was the v0.1.0 bug).
Window {
    id: root
    width: 720
    height: 520
    visible: true
    title: "VisiCalc — Mosaic Qt demo"
    color: "#1E1E1E"

    // Host state — mirrors the Flutter / React reducer pattern at
    // smaller scale. The 5×5 sample spreadsheet is hard-coded; tap
    // on a cell to select it and pull its value into the formula bar.
    property var sampleRows: [
        ["15", "3",  "12", "8",  "5"],
        ["8",  "14", "7",  "22", "11"],
        ["12", "9",  "18", "6",  "25"],
        ["4",  "11", "3",  "17", "9"],
        ["7",  "5",  "13", "10", "19"]
    ]
    property int selectedRow: 0
    property int selectedCol: 0
    property string formulaText: "=SUM(B1:B5)"

    function cellAddress() {
        return String.fromCharCode(65 + selectedCol) + (selectedRow + 1);
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Title bar.
        Text {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.topMargin: 16
            text: "VISICALC · MOSAIC QT DEMO"
            color: "#9D9D9D"
            font.pixelSize: 11
            font.letterSpacing: 1.0
        }

        // FormulaBar — the AUTO-GENERATED component from
        // build/FormulaBar.qml. Two-way wiring: the host pushes
        // `cellAddress` / `formula` into properties, and listens
        // for `formulaChange` / `commit` / `cancel` signals.
        FormulaBar {
            id: formulaBar
            Layout.fillWidth: true
            Layout.topMargin: 8
            cellAddress: root.cellAddress()
            formula: root.formulaText
            readOnly: false

            onFormulaChange: (value) => root.formulaText = value
            onCommit: { /* no-op for v1; keeps the formula text as-is */ }
            onCancel: { root.formulaText = root.sampleRows[root.selectedRow][root.selectedCol] }
        }

        // Grid — AUTO-GENERATED from
        // demo/visicalc/mosaic/Grid.{mil,desktop.mll,dark.msl} via
        // `mosaic-compile --backend qt`.  Grid.desktop.mll is a UI34
        // `pkg::mosaic-pkg-grid::Grid` one-liner; the QML component
        // you see here is the package's authoritative Grid +
        // Cell composition lowered to QtQuick by mosaic-emit-qt.
        //
        // List-typed slots (columnHeaders / viewportRows /
        // columnWidths) are passed as JS arrays through the
        // property bindings; the Qt emitter declares them as
        // `property var`.  The four signals (`navigate`,
        // `formulaChange`, `editCommit`, `editCancel`) are wired
        // back into the host's selection state.
        Grid {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16

            columnHeaders: ["A", "B", "C", "D", "E"]
            // 6 widths: row-label column + 5 data columns.
            columnWidths: [48, 96, 96, 96, 96, 96]
            viewportRows: root.sampleRows
            selectedRow: root.selectedRow
            selectedCol: root.selectedCol
            // Negative coordinates ⇒ no cell is editing — matches
            // every other VC2-* demo's default state.
            editRow: -1
            editCol: -1
            editContent: ""
            totalHeight: 0

            onNavigate: (row, col) => {
                root.selectedRow = row;
                root.selectedCol = col;
                root.formulaText = root.sampleRows[row][col];
            }
            onFormulaChange: (_value) => { /* live-edit deferred */ }
            onEditCommit:    () => { /* live-edit deferred */ }
            onEditCancel:    () => { /* live-edit deferred */ }
        }
    }
}
