// main.qml — VisiCalc Qt host (VC2-qt), now computing on the shared Rust
// `spreadsheet-core` engine through its C ABI.
//
// Mounts the auto-generated `FormulaBar` and `Grid` components from build/.
// Both come from `bash scripts/build.sh`, which runs `mosaic-compile --backend
// qt` against the shared `demo/visicalc/mosaic/{FormulaBar,Grid}.*` triples.
// No hand-written widgets in this file.
//
// The grid's DATA comes from `model.viewportRows` — `model` is the C++
// SpreadsheetModel that main.cpp exposes as a context property. It computes
// every value on the Rust engine (the same engine the web demos run as
// WebAssembly), so editing the formula bar writes through to the engine and
// every dependent cell recomputes.
//
// Run:
//   # The engine-backed binary (this is the real demo):
//   cmake -B build && cmake --build build && ./build/visicalc_qt_app
//   # or with qmake:  qmake && make && ./visicalc_qt_app
//
//   # `qml main.qml` still opens the layout, but there is no `model` there, so
//   # the grid is empty — use the compiled binary to see the spreadsheet.

import QtQuick 2.15
import QtQuick.Window 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

// FormulaBar / Grid come from the local `qml/` module, whose qmldir references
// the auto-generated build/*.qml. We split the tracked manifest (qml/qmldir)
// from the generated artifacts (build/*.qml) because the repo's top-level
// .gitignore excludes any directory named build/.
import "qml"

Window {
    id: root
    width: 1180
    height: 680
    minimumWidth: 560
    minimumHeight: 420
    visible: true
    title: "VisiCalc — Mosaic Qt demo · Rust engine"
    color: "#1E1E1E"

    // The only host-local UI state is the formula bar's text: it mirrors
    // keystrokes and is synced to the selected cell's source on selection. All
    // spreadsheet data lives in the engine, behind `model`.
    property string formulaText: model ? model.selectedRaw : ""

    // Which view is showing: the classic 5×5 cross-foot budget (the auto-
    // generated Grid), or the virtualized infinite sheet (InfiniteSheet.qml,
    // a sibling component rendered on the same engine via the viewport
    // primitive). The Layout ignores items whose `visible` is false, so only
    // the active view participates in layout.
    property bool infinite: true

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Title bar + view toggle.
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            Layout.topMargin: 16
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: root.infinite
                      ? "VISICALC · INFINITE SHEET · RUST ENGINE"
                      : "VISICALC · MOSAIC QT DEMO · RUST ENGINE"
                color: "#9D9D9D"
                font.pixelSize: 11
                font.letterSpacing: 1.0
            }
            Button {
                text: root.infinite ? "Classic grid" : "Infinite sheet"
                onClicked: root.infinite = !root.infinite
            }
        }

        // FormulaBar — the AUTO-GENERATED component. The host pushes the
        // selected cell's address + source in; on commit (Enter) it writes the
        // edited text to the engine via `model.setSelected`, which recomputes.
        FormulaBar {
            id: formulaBar
            visible: !root.infinite
            Layout.fillWidth: true
            Layout.topMargin: 8
            cellAddress: model ? model.cellAddress : ""
            formula: root.formulaText
            readOnly: false

            onFormulaChange: (value) => root.formulaText = value
            onCommit: {
                if (model) {
                    model.setSelected(root.formulaText);
                    root.formulaText = model.selectedRaw; // canonicalised source
                }
            }
            onCancel: { root.formulaText = model ? model.selectedRaw : "" }
        }

        // Grid — AUTO-GENERATED. Its `viewportRows` is bound to the engine's
        // computed display matrix; selecting a cell (onNavigate) updates the
        // model's selection and pulls that cell's source into the formula bar.
        Grid {
            visible: !root.infinite
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 16

            columnHeaders: ["", "A", "B", "C", "D", "E"]
            columnWidths: [48, 96, 96, 96, 96, 96]
            viewportRows: model ? model.viewportRows : []
            selectedRow: model ? model.selectedRow : 0
            selectedCol: model ? model.selectedCol : 1
            editRow: -1
            editCol: -1
            editContent: ""
            totalHeight: 0

            onNavigate: (row, col) => {
                if (model) {
                    model.select(row, col);
                    root.formulaText = model.selectedRaw;
                }
            }
            onFormulaChange: (_value) => { /* in-cell live-edit deferred */ }
            onEditCommit:    () => { /* in-cell live-edit deferred */ }
            onEditCancel:    () => { /* in-cell live-edit deferred */ }
        }

        // InfiniteSheet — the virtualized, effectively-infinite view, rendered
        // on the same engine through the viewport primitive. A sibling QML file
        // in this directory (auto-importable). Only laid out when active.
        InfiniteSheet {
            visible: root.infinite
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 8
        }
    }
}
