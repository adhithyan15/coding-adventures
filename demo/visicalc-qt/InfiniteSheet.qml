// InfiniteSheet.qml — a virtualized, effectively-infinite spreadsheet view for
// the Qt demo, rendered on the shared Rust engine through the viewport
// primitive (the same get_display_window / used_range / changed_since the SwiftUI
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
// `get_display_window` over that row's 1×totalCols strip (display strings,
// already rendered through each cell's format code) — so the per-frame engine
// work is proportional to visible rows, never to the sheet's height.

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

    // In-memory "saved file" slot for the Save / Load buttons: Save stows the
    // serialized workbook here, Load restores from it. (A real app would write
    // this string to a file or QSettings; the demo keeps it in memory so the
    // round trip is self-contained.)
    property string savedSnapshot: ""

    // Cell geometry (pixels). The gutter and body share rowH so their two
    // ListViews scroll in lockstep. (Roomier to match the web reference.)
    readonly property int rowH: 26
    readonly property int colW: 92
    readonly property int gutterW: 64
    readonly property int headH: 28

    // ── Design tokens ────────────────────────────────────────────────
    // Mirror demo/visicalc-html/infinite.html's palette so every VisiCalc
    // backend reads as one considered surface (dark modern spreadsheet).
    readonly property color cBg:          "#16181d"  // app / base cell
    readonly property color cPanel:       "#1b1e24"  // toolbar + zebra band
    readonly property color cSurface:     "#21252c"  // buttons, pill
    readonly property color cSurfaceHover:"#2b313a"
    readonly property color cSurfaceDown: "#14171c"
    readonly property color cLine:        "#2c313a"  // hairline borders
    readonly property color cLineStrong:  "#3a404b"  // control borders
    readonly property color cHead:        "#20242b"  // row/col headers
    readonly property color cHeadSel:     "#2b3340"  // header of selected row/col
    readonly property color cInk:         "#e8eaed"  // primary text
    readonly property color cMuted:       "#9aa3b2"  // labels, headers
    readonly property color cAccent:      "#4aa3ff"  // selection + focus
    readonly property color cSel:         "#21344a"  // selected-cell fill

    // Cells and the formula field use a monospace face so digits column-align.
    // The generic "monospace" alias only resolves on Linux (fontconfig); macOS
    // and Windows need a concrete family, else Qt warns and falls back to the
    // proportional UI font (digits stop aligning). Pick a real face per OS.
    readonly property string monoFamily:
        Qt.platform.os === "osx"     ? "Menlo" :
        Qt.platform.os === "windows" ? "Consolas" :
                                       "monospace"

    // A compact, modern toolbar button (rounded chip with hover/down/disabled
    // states) — the QML analog of the web demo's segmented controls.
    component ToolButton: Button {
        id: tb
        hoverEnabled: true
        implicitHeight: 30
        leftPadding: 11; rightPadding: 11
        font.pixelSize: 12
        background: Rectangle {
            radius: 5
            color: tb.down ? sheet.cSurfaceDown : (tb.hovered && tb.enabled ? sheet.cSurfaceHover : sheet.cSurface)
            border.color: sheet.cLineStrong
            border.width: 1
            opacity: tb.enabled ? 1.0 : 0.4
        }
        contentItem: Text {
            text: tb.text
            font: tb.font
            color: tb.hovered && tb.enabled ? "#ffffff" : sheet.cInk
            opacity: tb.enabled ? 1.0 : 0.4
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // ── Formula bar ──────────────────────────────────────────────
        // The selected cell's address (e.g. "Z1000") plus an editable source
        // line. Enter commits through `doc.commitInf`, which writes to the
        // engine, recomputes dependents, regrows the extent, and bumps
        // `doc.revision` so the visible rows re-fetch.
        Rectangle {                          // toolbar panel
            Layout.fillWidth: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 10
            Layout.preferredHeight: 48
            color: sheet.cPanel
            border.color: sheet.cLine
            radius: 8

            RowLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 6

                // Address pill.
                Rectangle {
                    Layout.preferredWidth: 46
                    Layout.preferredHeight: 30
                    color: sheet.cSurface
                    border.color: sheet.cLineStrong
                    radius: 5
                    Text {
                        anchors.centerIn: parent
                        text: doc ? doc.infAddress : ""
                        color: sheet.cInk
                        font.pixelSize: 12
                        font.bold: true
                        font.family: sheet.monoFamily
                    }
                }
                Text {
                    text: "fx"
                    color: sheet.cMuted
                    font.pixelSize: 12
                    font.italic: true
                    font.family: sheet.monoFamily
                }
                // Formula field — accent focus ring on edit.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 30
                    color: "#0f1115"
                    radius: 5
                    border.color: formula.activeFocus ? sheet.cAccent : sheet.cLineStrong
                    border.width: formula.activeFocus ? 2 : 1
                    TextInput {
                        id: formula
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 8
                        verticalAlignment: TextInput.AlignVCenter
                        color: sheet.cInk
                        font.pixelSize: 13
                        font.family: sheet.monoFamily
                        clip: true
                        selectByMouse: true
                        // Re-seed from the model on selection change; the user's
                        // edits live here until they press Enter.
                        text: doc ? doc.infFormula : ""
                        onAccepted: if (doc) doc.commitInf(text)
                    }
                }

                // ── Drag-fill ──
                ToolButton {
                    text: "↓ Fill 10"
                    ToolTip.visible: hovered
                    ToolTip.text: "Replicate the selected cell into the 10 rows below it"
                    onClicked: if (doc) {
                        var first = doc.columnLetters(doc.infCol) + (doc.infRow + 1);
                        var last = doc.columnLetters(doc.infCol) + (doc.infRow + 10);
                        doc.fill(doc.infAddress, first, last);
                    }
                }
                Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; Layout.topMargin: 4; Layout.bottomMargin: 4; color: sheet.cLine }

                // ── Clipboard ──
                ToolButton {
                    text: "Copy"
                    ToolTip.visible: hovered
                    ToolTip.text: "Copy the selected cell to the clipboard"
                    onClicked: if (doc) doc.copy(doc.infAddress, doc.infAddress)
                }
                ToolButton {
                    text: "Cut"
                    ToolTip.visible: hovered
                    ToolTip.text: "Cut the selected cell (cleared when you paste)"
                    onClicked: if (doc) doc.cut(doc.infAddress, doc.infAddress)
                }
                ToolButton {
                    text: "Paste"
                    ToolTip.visible: hovered
                    ToolTip.text: "Paste the clipboard at the selected cell, shifting relative references"
                    onClicked: if (doc) doc.paste(doc.infAddress)
                }
                Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; Layout.topMargin: 4; Layout.bottomMargin: 4; color: sheet.cLine }

                // ── File (save / load) ──
                ToolButton {
                    text: "Save"
                    ToolTip.visible: hovered
                    ToolTip.text: "Serialize the whole workbook to memory"
                    onClicked: if (doc) sheet.savedSnapshot = doc.serialize()
                }
                ToolButton {
                    text: "Load"
                    enabled: sheet.savedSnapshot.length > 0
                    ToolTip.visible: hovered
                    ToolTip.text: "Restore the workbook from the last save"
                    onClicked: if (doc) doc.deserialize(sheet.savedSnapshot)
                }
                Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; Layout.topMargin: 4; Layout.bottomMargin: 4; color: sheet.cLine }

                // ── Structure (insert / delete the selected row or column) ──
                // The engine shifts every formula reference across the band; a
                // reference whose whole band is deleted becomes #REF!.
                ToolButton {
                    text: "+ Row"
                    ToolTip.visible: hovered
                    ToolTip.text: "Insert a row above the selected cell (references shift down)"
                    onClicked: if (doc) doc.insertRows(doc.infRow, 1)
                }
                ToolButton {
                    text: "− Row"
                    ToolTip.visible: hovered
                    ToolTip.text: "Delete the selected cell's row (references shift up; refs into it become #REF!)"
                    onClicked: if (doc) doc.deleteRows(doc.infRow, 1)
                }
                ToolButton {
                    text: "+ Col"
                    ToolTip.visible: hovered
                    ToolTip.text: "Insert a column left of the selected cell (references shift right)"
                    onClicked: if (doc) doc.insertCols(doc.infCol, 1)
                }
                ToolButton {
                    text: "− Col"
                    ToolTip.visible: hovered
                    ToolTip.text: "Delete the selected cell's column (references shift left; refs into it become #REF!)"
                    onClicked: if (doc) doc.deleteCols(doc.infCol, 1)
                }
                Rectangle { Layout.preferredWidth: 1; Layout.fillHeight: true; Layout.topMargin: 4; Layout.bottomMargin: 4; color: sheet.cLine }

                // ── History (undo / redo) ──
                ToolButton {
                    text: "↶ Undo"
                    enabled: doc ? doc.canUndo : false
                    ToolTip.visible: hovered
                    ToolTip.text: "Undo the last edit"
                    onClicked: if (doc) doc.undo()
                }
                ToolButton {
                    text: "↷ Redo"
                    enabled: doc ? doc.canRedo : false
                    ToolTip.visible: hovered
                    ToolTip.text: "Redo the last undone edit"
                    onClicked: if (doc) doc.redo()
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
                color: sheet.cHead
                border.color: sheet.cLineStrong
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
                            readonly property bool selHdr: doc && doc.infCol === index + 1
                            width: sheet.colW
                            height: sheet.headH
                            color: selHdr ? sheet.cHeadSel : sheet.cHead
                            border.color: sheet.cLine
                            Text {
                                anchors.centerIn: parent
                                text: doc ? doc.columnLetters(index + 1) : ""
                                color: selHdr ? sheet.cAccent : sheet.cMuted
                                font.pixelSize: 11
                                font.bold: true
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
                    readonly property bool selHdr: doc && doc.infRow === index + 1
                    width: sheet.gutterW
                    height: sheet.rowH
                    color: selHdr ? sheet.cHeadSel : sheet.cHead
                    border.color: sheet.cLine
                    Text {
                        anchors.centerIn: parent
                        text: index + 1
                        color: selHdr ? sheet.cAccent : sheet.cMuted
                        font.pixelSize: 11
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
                                readonly property bool selected: doc && doc.infRow === rowItem.rowNum
                                                                 && doc.infCol === colNum
                                width: sheet.colW
                                height: sheet.rowH
                                // Selected → accent fill; else zebra band on even rows.
                                color: selected ? sheet.cSel
                                       : (rowItem.rowNum % 2 === 0 ? sheet.cPanel : sheet.cBg)
                                border.color: selected ? sheet.cAccent : sheet.cLine
                                border.width: selected ? 2 : 1
                                z: selected ? 1 : 0
                                Text {
                                    anchors.fill: parent
                                    anchors.rightMargin: 6
                                    horizontalAlignment: Text.AlignRight
                                    verticalAlignment: Text.AlignVCenter
                                    elide: Text.ElideRight
                                    text: (colNum - 1) < rowItem.cells.length
                                          ? rowItem.cells[colNum - 1] : ""
                                    color: selected ? "#ffffff" : sheet.cInk
                                    font.pixelSize: 12
                                    font.bold: selected
                                    font.family: sheet.monoFamily
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

        // ── Status line ──────────────────────────────────────────────
        // A hairline-separated footer echoing the live virtual-grid size and
        // the per-edit revision clock (mirrors the web demo's status line).
        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.bottomMargin: 10
            Layout.preferredHeight: 1
            color: sheet.cLine
        }
        Text {
            Layout.fillWidth: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.bottomMargin: 10
            color: sheet.cMuted
            font.pixelSize: 12
            font.family: sheet.monoFamily
            text: doc
                  ? "Virtual grid: " + doc.totalRows + " rows × " + doc.totalCols
                    + " cols  ·  revision " + doc.revision
                  : ""
        }
    }
}
