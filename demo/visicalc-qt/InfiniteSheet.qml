// InfiniteSheet.qml — a virtualized, effectively-infinite spreadsheet view for
// the Qt demo, rendered on the shared Rust engine through the viewport
// primitive (the same get_window / used_range / changed_since the SwiftUI
// InfiniteGridView and the web infinite.html drive).
//
// The sheet is u32 × u32 and sparse; only the cells in the VISIBLE rows are ever
// built. The body is a QtQuick `ListView`, which natively virtualizes — it
// instantiates a row delegate only while that row is on screen and recycles it
// when it scrolls off. So a 1000-row-tall sheet costs the handful of rows you
// can actually see, not 1000 delegates.
//
// Two-axis scroll + frozen chrome, all kept in sync by binding offsets:
//
//   ┌────────┬──────────────────────────────┐
//   │ corner │  column-letter header  (A B…) │  ← frozen on top, scrolls →
//   ├────────┼──────────────────────────────┤
//   │  row   │                              │
//   │ number │   body: ListView of rows     │
//   │ gutter │   (each row = Repeater of    │  gutter frozen left, scrolls ↓
//   │  1 2 … │    totalCols cells)          │
//   └────────┴──────────────────────────────┘
//
//   • header.contentX  ← body horizontal Flickable.contentX   (header tracks ↔)
//   • gutter.contentY  ← body ListView.contentY               (gutter tracks ↕)
//
// Each visible row delegate calls `doc.rowCells(rowNum)` ONCE — a single engine
// `get_window` over that row's 1×totalCols strip — so the per-frame engine work
// is proportional to visible rows, never to the sheet's height.

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Item {
    id: sheet

    // The C++ SpreadsheetModel context property is named `model`, but `model`
    // is ALSO the implicit name of a ListView/Repeater's data model inside a
    // delegate — referencing `model` there would resolve to the wrong thing.
    // Alias it once as `doc` and use that everywhere to dodge the shadowing.
    readonly property var doc: model

    // Cell geometry (pixels). The gutter and body share rowH so their two
    // ListViews scroll in lockstep.
    readonly property int rowH: 24
    readonly property int colW: 90
    readonly property int gutterW: 64
    readonly property int headH: 26

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── Formula bar ──────────────────────────────────────────────
        // The selected cell's address (e.g. "Z1000") plus an editable source
        // line. Enter commits through `doc.commitInf`, which writes to the
        // engine, recomputes dependents, regrows the extent, and bumps
        // `doc.revision` so the visible rows re-fetch.
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            Layout.topMargin: 8
            spacing: 8

            Text {
                text: doc ? doc.infAddress : ""
                color: "#9D9D9D"
                font.pixelSize: 12
                font.family: "monospace"
                Layout.preferredWidth: sheet.gutterW
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 28
                color: "#2D2D30"
                border.color: "#3F3F46"
                TextInput {
                    id: formula
                    anchors.fill: parent
                    anchors.leftMargin: 6
                    anchors.rightMargin: 6
                    verticalAlignment: TextInput.AlignVCenter
                    color: "#CCCCCC"
                    font.pixelSize: 13
                    font.family: "monospace"
                    clip: true
                    selectByMouse: true
                    // Re-seed from the model on selection change; the user's
                    // edits live here until they press Enter.
                    text: doc ? doc.infFormula : ""
                    onAccepted: if (doc) doc.commitInf(text)
                }
            }
        }

        // ── Column-letter header (frozen vertically, scrolls horizontally) ──
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            Layout.topMargin: 6
            spacing: 0

            // Corner cell above the gutter.
            Rectangle {
                Layout.preferredWidth: sheet.gutterW
                Layout.preferredHeight: sheet.headH
                color: "#2D2D30"
                border.color: "#3F3F46"
            }
            Flickable {
                id: header
                Layout.fillWidth: true
                Layout.preferredHeight: sheet.headH
                contentWidth: (doc ? doc.totalCols : 0) * sheet.colW
                contentHeight: sheet.headH
                interactive: false              // driven by the body's scroll
                clip: true
                contentX: bodyFlick.contentX     // track the body's HORIZONTAL pan
                Row {
                    Repeater {
                        model: doc ? doc.totalCols : 0
                        delegate: Rectangle {
                            width: sheet.colW
                            height: sheet.headH
                            color: "#2D2D30"
                            border.color: "#3F3F46"
                            Text {
                                anchors.centerIn: parent
                                text: doc ? doc.columnLetters(index + 1) : ""
                                color: "#9D9D9D"
                                font.pixelSize: 12
                                font.family: "monospace"
                            }
                        }
                    }
                }
            }
        }

        // ── Body: row-number gutter + virtualized cell grid ─────────────
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            Layout.bottomMargin: 8
            spacing: 0

            // Row-number gutter — its own ListView so it virtualizes too,
            // non-interactive and slaved to the body's vertical scroll.
            ListView {
                id: gutter
                Layout.preferredWidth: sheet.gutterW
                Layout.fillHeight: true
                model: doc ? doc.totalRows : 0
                interactive: false
                clip: true
                contentY: body.contentY
                boundsBehavior: Flickable.StopAtBounds
                delegate: Rectangle {
                    width: sheet.gutterW
                    height: sheet.rowH
                    color: "#2D2D30"
                    border.color: "#3F3F46"
                    Text {
                        anchors.centerIn: parent
                        text: index + 1
                        color: "#9D9D9D"
                        font.pixelSize: 12
                        font.family: "monospace"
                    }
                }
            }

            // The cells. A horizontal Flickable supplies left/right scroll; the
            // vertical ListView inside it supplies up/down scroll + row
            // virtualization. The ListView is as wide as the whole column span,
            // so the Flickable's contentX pans it horizontally.
            Flickable {
                id: bodyFlick
                Layout.fillWidth: true
                Layout.fillHeight: true
                contentWidth: (doc ? doc.totalCols : 0) * sheet.colW
                contentHeight: height
                flickableDirection: Flickable.HorizontalFlick
                clip: true
                boundsBehavior: Flickable.StopAtBounds

                ListView {
                    id: body
                    width: bodyFlick.contentWidth
                    height: bodyFlick.height
                    model: doc ? doc.totalRows : 0
                    clip: true
                    boundsBehavior: Flickable.StopAtBounds
                    cacheBuffer: sheet.rowH * 4

                    delegate: Row {
                        id: rowItem
                        // ListView's `index` is 0-based; the engine is 1-based.
                        readonly property int rowNum: index + 1
                        // One engine read for the whole row. Touching
                        // `doc.revision` makes this re-fetch after any commit.
                        readonly property var cells: doc
                            ? (doc.revision, doc.rowCells(rowNum))
                            : []

                        Repeater {
                            model: doc ? doc.totalCols : 0
                            delegate: Rectangle {
                                id: cell
                                readonly property int colNum: index + 1
                                width: sheet.colW
                                height: sheet.rowH
                                color: (doc && doc.infRow === rowItem.rowNum
                                        && doc.infCol === colNum)
                                       ? "#094771" : "#1E1E1E"
                                border.color: "#3F3F46"
                                border.width: 1
                                Text {
                                    anchors.fill: parent
                                    anchors.rightMargin: 4
                                    horizontalAlignment: Text.AlignRight
                                    verticalAlignment: Text.AlignVCenter
                                    elide: Text.ElideRight
                                    text: (colNum - 1) < rowItem.cells.length
                                          ? rowItem.cells[colNum - 1] : ""
                                    color: "#CCCCCC"
                                    font.pixelSize: 12
                                    font.family: "monospace"
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    onClicked: if (doc) doc.selectInf(rowItem.rowNum, cell.colNum)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
