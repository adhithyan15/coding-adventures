// tst_window.cpp — headless proof that the Qt demo can drive a VIRTUALIZED
// infinite sheet on the engine's viewport primitive (the C ABI's sc_get_window
// / sc_used_range / sc_changed_since), with no GUI. The Qt sibling of the
// SwiftUI demo's WindowedModelTests and the web demo's verify-infinite.mjs.
//
// It seeds deliberately far-flung, sparse cells on top of the model's default
// cross-foot budget, then asserts the windowed reads are correct, bounded,
// sparse, and diff on edit.
//
// Build & run (qmake — no CMake required):
//   cd test && qmake tst_window.pro && make && ./tst_window

#include <QtTest/QtTest>

#include "SpreadsheetModel.h"

class TstWindow : public QObject {
    Q_OBJECT

private slots:
    void windowIsEngineComputedAndDense();
    void farWindowReachesZ1000AndGapsAreSparse();
    void extentColumnLettersAndChangedSince();
    void infiniteViewSelectEditAndRowCells();
};

// Helper: the display string at window (1-based) cell (row, col), given the
// window's origin (row0, col0).
static QString at(const QVariantList &win, int row0, int col0, int row, int col) {
    const QVariantList r = win.at(row - row0).toList();
    return r.at(col - col0).toString();
}

void TstWindow::windowIsEngineComputedAndDense() {
    SpreadsheetModel m;
    // The default seed is the 5×5 cross-foot budget; a window over it is
    // engine-computed and dense.
    const QVariantList w = m.window(1, 1, 5, 5);
    QCOMPARE(w.size(), 5);
    QCOMPARE(at(w, 1, 1, 1, 1), QStringLiteral("15"));  // A1
    QCOMPARE(at(w, 1, 1, 1, 5), QStringLiteral("38"));  // E1 = SUM(A1:D1)
    QCOMPARE(at(w, 1, 1, 5, 5), QStringLiteral("169")); // E5 grand total
}

void TstWindow::farWindowReachesZ1000AndGapsAreSparse() {
    SpreadsheetModel m;
    // Plant a far-flung formula 1000 rows down (Z = col 26).
    m.setCell("Z1000", "=SUM(A1:A4)"); // 15+8+12+4 = 39
    // A window around it returns the computed value — reachable without
    // materialising the millions of cells in between.
    QCOMPARE(at(m.window(998, 24, 1002, 28), 998, 24, 1000, 26), QStringLiteral("39"));
    // The gap between the two data islands is empty (the sheet is sparse).
    const QVariantList gap = m.window(100, 1, 110, 10);
    for (const QVariant &rowVar : gap) {
        for (const QVariant &cell : rowVar.toList()) {
            QCOMPARE(cell.toString(), QString());
        }
    }
}

void TstWindow::extentColumnLettersAndChangedSince() {
    SpreadsheetModel m;
    m.setCell("Z1000", "=SUM(A1:A4)");

    // used_range extends to the far cells. The default seed plants Z1000 (row
    // 1000) and BB50 (col 54 = "BB"), so the extent spans both far islands.
    const QVariantMap u = m.usedRange();
    QCOMPARE(u.value("maxRow").toInt(), 1000);
    QCOMPARE(u.value("maxCol").toInt(), 54);

    // Column letters past Z.
    QCOMPARE(m.columnLetters(27), QStringLiteral("AA"));
    QCOMPARE(m.columnLetters(53), QStringLiteral("BA"));

    // Editing A1 dirties its far dependent Z1000 (=SUM(A1:A4)) via the diff.
    const quint64 rev = m.currentRevision();
    m.setCell("A1", "115");
    const QStringList changed = m.changedSince(rev);
    QVERIFY2(changed.contains("A1"), "A1 changed");
    QVERIFY2(changed.contains("Z1000"),
             qPrintable("far dependent recomputed: " + changed.join(",")));
    // And the recomputed value shows through a fresh window read: 115+8+12+4.
    QCOMPARE(at(m.window(1000, 26, 1000, 26), 1000, 26, 1000, 26), QStringLiteral("139"));
}

// The infinite-view binding layer (InfiniteSheet.qml drives these): one engine
// read per visible row via rowCells, tap-to-select via selectInf (which pulls
// the cell's source into the formula bar), and write-through via commitInf
// (which recomputes dependents, regrows the extent, and bumps `revision`).
void TstWindow::infiniteViewSelectEditAndRowCells() {
    SpreadsheetModel m;

    // The constructor seeds the cross-foot budget PLUS far-flung cells
    // (Z1000, BA50/BB50) and computes the extent: at least 1000×60, and grown
    // to reach the far cells. Z1000 → totalRows ≥ 1000 + margin.
    QVERIFY2(m.totalRows() >= 1000, "extent grows to reach the far seeded cells");
    QVERIFY2(m.totalCols() >= 60, "extent has the default column margin");

    // rowCells returns one row's display strings (columns 1..totalCols). Row 1
    // is the budget's first row: A1..E1 = 15,3,12,8,38, then blanks.
    const QVariantList row1 = m.rowCells(1);
    QCOMPARE(row1.size(), m.totalCols());
    QCOMPARE(row1.at(0).toString(), QStringLiteral("15")); // A1
    QCOMPARE(row1.at(4).toString(), QStringLiteral("38")); // E1 = SUM(A1:D1)
    QCOMPARE(row1.at(9).toString(), QString());            // J1 empty (sparse)
    // A row in the gap between the data islands is entirely blank.
    const QVariantList gapRow = m.rowCells(200);
    for (const QVariant &cell : gapRow) QCOMPARE(cell.toString(), QString());

    // selectInf moves the selection (1-based) and pulls the cell's SOURCE into
    // the formula bar — A5 is a formula, so we see the formula, not its value.
    m.selectInf(5, 1);
    QCOMPARE(m.infRow(), 5);
    QCOMPARE(m.infCol(), 1);
    QCOMPARE(m.infAddress(), QStringLiteral("A5"));
    QCOMPARE(m.infFormula(), QStringLiteral("=SUM(A1:A4)"));

    // selectInf clamps to the virtual grid (never below 1, never past the extent).
    m.selectInf(-3, 0);
    QCOMPARE(m.infRow(), 1);
    QCOMPARE(m.infCol(), 1);

    // commitInf writes the formula bar through to the selected cell. Edit A2
    // 8 → 108 and every dependent recomputes: E2 (row total) and A5/E5 (col /
    // grand totals) all move, visible through a fresh rowCells read.
    const int revBefore = m.revision();
    m.selectInf(2, 1);                 // A2
    m.commitInf(QStringLiteral("108"));
    QVERIFY2(m.revision() > revBefore, "commit bumps the revision so rows re-fetch");
    QCOMPARE(m.rowCells(2).at(0).toString(), QStringLiteral("108")); // A2
    QCOMPARE(m.rowCells(2).at(4).toString(), QStringLiteral("151")); // E2 = 108+14+7+22
    QCOMPARE(m.rowCells(5).at(0).toString(), QStringLiteral("139")); // A5 = 15+108+12+4
    QCOMPARE(m.rowCells(5).at(4).toString(), QStringLiteral("269")); // E5 grand total
}

QTEST_MAIN(TstWindow)
#include "tst_window.moc"
