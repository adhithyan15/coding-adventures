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

    // used_range extends to the far cell.
    const QVariantMap u = m.usedRange();
    QCOMPARE(u.value("maxRow").toInt(), 1000);
    QCOMPARE(u.value("maxCol").toInt(), 26);

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

QTEST_MAIN(TstWindow)
#include "tst_window.moc"
