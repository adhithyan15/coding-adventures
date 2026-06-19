// SpreadsheetModel.h — the Qt/C++ host model for the VisiCalc demo, computing
// on the shared Rust `spreadsheet-core` engine through its C ABI
// (spreadsheet-capi). This is the Qt sibling of the SwiftUI demo's
// SpreadsheetModel: it owns NO spreadsheet logic. The engine (cells,
// dependency graph, recalc, formulas) lives behind the C ABI; this class
// marshals QStrings across it and maps the engine's JSON value shape into the
// display text a spreadsheet should show — the same engine, and the same JSON
// contract, the web demos drive as WebAssembly.
//
// Why QtCore-only (no QtGui/QtQuick include here): keeping the model free of
// GUI types lets the headless QtTest (test/tst_model.cpp) link and exercise it
// without a display — the automated proof that "Qt does real formula work on
// the engine," exactly like `swift test` does for the SwiftUI demo.
//
// Exposed to QML as the `model` context property (see main.cpp). main.qml binds
// the generated Grid's `viewportRows` to `model.viewportRows`, and routes the
// FormulaBar's commit through `model.setSelected(...)`, which writes to the
// engine and recomputes — so every dependent cell updates live.

#ifndef VISICALC_QT_SPREADSHEET_MODEL_H
#define VISICALC_QT_SPREADSHEET_MODEL_H

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>

// The opaque session handle is forward-declared so this header doesn't drag the
// C ABI header into every translation unit; SpreadsheetModel.cpp includes it.
struct ScSession;

class SpreadsheetModel : public QObject {
    Q_OBJECT

    // The display matrix fed to the generated GridView: each row is
    // [rowLabel, A, B, C, D, E]. A QVariantList of QVariantList<QString>, which
    // QML sees as a JS array of arrays — the exact shape the Qt Grid emitter's
    // `property var viewportRows` Repeater consumes.
    Q_PROPERTY(QVariantList viewportRows READ viewportRows NOTIFY changed)
    // The current selection, in the grid's display coordinates: row 0..4, and
    // column 1..5 (column 0 is the row-label gutter, which has no address).
    Q_PROPERTY(int selectedRow READ selectedRow NOTIFY selectionChanged)
    Q_PROPERTY(int selectedCol READ selectedCol NOTIFY selectionChanged)
    // The A1 address of the selected cell (e.g. "B3"), for the formula bar.
    Q_PROPERTY(QString cellAddress READ cellAddress NOTIFY selectionChanged)
    // The raw source (formula/literal) of the selected cell, for the bar's text.
    Q_PROPERTY(QString selectedRaw READ selectedRaw NOTIFY selectionChanged)

public:
    explicit SpreadsheetModel(QObject *parent = nullptr);
    ~SpreadsheetModel() override;

    // 5 data rows, columns A..E.
    static constexpr int Rows = 5;
    static constexpr int Cols = 5;

    QVariantList viewportRows() const { return viewportRows_; }
    int selectedRow() const { return selectedRow_; }
    int selectedCol() const { return selectedCol_; }
    QString cellAddress() const;
    QString selectedRaw() const;

    // Move the selection, clamped to the grid (col >= 1 — never the gutter).
    Q_INVOKABLE void select(int row, int col);
    // Write `raw` (a literal like "100" or a formula like "=SUM(A1:A4)") into the
    // selected cell, then recompute the whole display matrix from the engine.
    Q_INVOKABLE void setSelected(const QString &raw);
    // Write `raw` into an explicit A1 address and recompute. Used by tests.
    Q_INVOKABLE void setCell(const QString &a1, const QString &raw);
    // The display string of a cell (what the grid shows). Used by tests.
    Q_INVOKABLE QString display(const QString &a1) const;
    // The raw value JSON the engine returns for a cell. Used by tests to assert
    // the engine contract directly.
    Q_INVOKABLE QString valueJson(const QString &a1) const;

    // ── Viewport primitive (virtualized infinite sheet) ──
    // A dense window of display strings (a list of rows, each a list of
    // QString), 1-based inclusive coords — what a windowed QML grid renders.
    Q_INVOKABLE QVariantList window(int row0, int col0, int row1, int col1) const;
    // Data extent {minRow,minCol,maxRow,maxCol}, or an empty map if the sheet
    // is empty. A host sizes its scrollable area to this.
    Q_INVOKABLE QVariantMap usedRange() const;
    // Column letters for a 1-based index (1 → "A", 27 → "AA").
    Q_INVOKABLE QString columnLetters(int index) const;
    // The per-edit revision clock; snapshot it, then pass to changedSince.
    Q_INVOKABLE quint64 currentRevision() const;
    // A1 addresses changed since `since` (empty if none / stale).
    Q_INVOKABLE QStringList changedSince(quint64 since) const;

    // ── Infinite-sheet view state (InfiniteSheet.qml) ──
    // The virtual grid size, the selection, the formula-bar text, and a revision
    // counter the QML rebinds on to refresh the visible rows after an edit. All
    // 1-based (row/col ≥ 1, col 1 = "A"). Separate from the 5×5 parity selection.
    Q_PROPERTY(int totalRows READ totalRows NOTIFY extentChanged)
    Q_PROPERTY(int totalCols READ totalCols NOTIFY extentChanged)
    Q_PROPERTY(int infRow READ infRow NOTIFY infSelectionChanged)
    Q_PROPERTY(int infCol READ infCol NOTIFY infSelectionChanged)
    Q_PROPERTY(QString infAddress READ infAddress NOTIFY infSelectionChanged)
    Q_PROPERTY(QString infFormula READ infFormula NOTIFY infSelectionChanged)
    Q_PROPERTY(int revision READ revision NOTIFY revisionChanged)
    // Undo/redo availability — bound to revisionChanged, which every mutating op
    // (incl. undo/redo themselves) emits, so the QML buttons enable/disable live.
    Q_PROPERTY(bool canUndo READ canUndo NOTIFY revisionChanged)
    Q_PROPERTY(bool canRedo READ canRedo NOTIFY revisionChanged)

    int totalRows() const { return totalRows_; }
    int totalCols() const { return totalCols_; }
    int infRow() const { return infRow_; }
    int infCol() const { return infCol_; }
    QString infAddress() const;
    QString infFormula() const { return infFormula_; }
    int revision() const { return revision_; }
    bool canUndo() const;
    bool canRedo() const;

    // One row's display strings (a QVariantList of QString, columns 1..totalCols)
    // — what a windowed ListView delegate renders. One engine read per row.
    Q_INVOKABLE QVariantList rowCells(int row) const;
    // Select the infinite-view cell (clamped); pulls its source into infFormula.
    Q_INVOKABLE void selectInf(int row, int col);
    // Commit the formula bar into the selected infinite-view cell: write through,
    // recompute, resize the extent, bump `revision` so the rows re-fetch.
    Q_INVOKABLE void commitInf(const QString &raw);
    // Drag-fill: replicate the `src` cell across the inclusive A1 rectangle
    // `dstStart`..`dstEnd` (relative refs shift, absolute pin, format carried);
    // resizes the extent and bumps `revision` so the visible rows re-fetch.
    Q_INVOKABLE void fill(const QString &src, const QString &dstStart, const QString &dstEnd);
    // Clipboard: copy/cut capture the inclusive rectangle `start`..`end` (a
    // whole-block copy that pastes as a unit); paste places the block so its
    // top-left lands at `dstStart`, shifting the block's references by the
    // destination's offset (a cut clears the source it didn't overwrite). paste
    // returns true when applied, false for a no-op (empty clipboard / malformed
    // address / off-grid). paste resizes the extent and bumps `revision`.
    Q_INVOKABLE void copy(const QString &start, const QString &end);
    Q_INVOKABLE void cut(const QString &start, const QString &end);
    Q_INVOKABLE bool paste(const QString &dstStart);
    // Save / load: serialize() returns a self-contained JSON document of the
    // workbook's SOURCE (formula text + typed literals) + per-cell formats — not
    // the computed values, which recompute on load. deserialize() replaces the
    // workbook from such a document: returns true on success, false for malformed
    // / unsupported input (the workbook is left untouched), recomputes the grid,
    // regrows the extent, and bumps `revision` so the visible rows re-fetch.
    Q_INVOKABLE QString serialize() const;
    Q_INVOKABLE bool deserialize(const QString &data);
    // Undo / redo: walk the engine's snapshot history. Each returns true if it
    // changed the document; on success the grid recomputes, the extent regrows,
    // the formula bar refreshes, and `revision` bumps (which re-evaluates the
    // canUndo/canRedo bindings).
    Q_INVOKABLE bool undo();
    Q_INVOKABLE bool redo();

signals:
    // viewportRows changed (after a recompute) — QML rebinds the grid.
    void changed();
    // The selection moved — QML rebinds cellAddress / selectedRaw / highlight.
    void selectionChanged();
    // Infinite-view signals.
    void extentChanged();
    void infSelectionChanged();
    void revisionChanged();

private:
    // A1 address for grid display row `r` (0-based) and column `c` (1..5).
    static QString address(int r, int c);
    // Seed the classic cross-footing budget (column E totals each row, row 5
    // totals each column, E5 the grand total — all formulas). Identical seed to
    // the SwiftUI and web demos so every render shows the same numbers.
    void seed();
    // Rebuild viewportRows_ from the engine's computed values and emit changed().
    void recompute();

    // Re-derive totalRows_/totalCols_ from the engine's used_range + a margin.
    void computeExtent();
    // The raw source of an A1 cell (the formula bar's text).
    QString rawAt(const QString &a1) const;

    ScSession *session_;
    QVariantList viewportRows_;
    int selectedRow_ = 0; // 0..4
    int selectedCol_ = 1; // 1..5 (0 = gutter)

    // Infinite-view state.
    int totalRows_ = 1000;
    int totalCols_ = 60;
    int infRow_ = 1;     // 1-based
    int infCol_ = 1;     // 1-based (1 = "A")
    QString infFormula_;
    int revision_ = 0;
};

#endif // VISICALC_QT_SPREADSHEET_MODEL_H
