// tst_model.cpp — headless proof that the Qt VisiCalc demo does REAL formula
// work on the shared Rust engine, with no GUI in the loop. This is the Qt
// sibling of the SwiftUI demo's `swift test`: it instantiates the same
// engine-backed SpreadsheetModel the QML grid binds to and asserts that the
// values are engine-computed and recompute on edit.
//
// It links the vendored libspreadsheet_capi.a (the Rust engine's C ABI), so a
// green run here means the C++ ↔ C ABI ↔ Rust path is sound end-to-end.
//
// Build & run (qmake — no CMake required):
//   cd test && qmake && make && ./tst_model

#include <QtTest/QtTest>

#include "SpreadsheetModel.h"

class TstModel : public QObject {
    Q_OBJECT

private slots:
    // The seeded cross-footing budget is computed by the engine, not hard-coded.
    void seededTotalsAreEngineComputed();
    // Editing an input ripples through every dependent total on recompute.
    void editingRecomputesDependents();
    // A formula entry computes, and a division-by-zero error propagates through
    // a binary operator (matching Excel; the engine's documented behaviour).
    void formulaAndErrorPropagation();
};

void TstModel::seededTotalsAreEngineComputed() {
    SpreadsheetModel model;

    // Row totals (column E = SUM of A..D for that row).
    QCOMPARE(model.display("E1"), QStringLiteral("38")); // 15+3+12+8
    QCOMPARE(model.display("E2"), QStringLiteral("51")); // 8+14+7+22
    // Column totals (row 5 = SUM of rows 1..4 for that column).
    QCOMPARE(model.display("A5"), QStringLiteral("39")); // 15+8+12+4
    // Grand total — SUM of the row totals.
    QCOMPARE(model.display("E5"), QStringLiteral("169"));

    // And it surfaces through the QML-facing display matrix: viewportRows[0] is
    // [rowLabel, A1..E1], so column index 5 is E1.
    const QVariantList rows = model.viewportRows();
    QCOMPARE(rows.size(), 5);
    const QVariantList row0 = rows.at(0).toList();
    QCOMPARE(row0.size(), 6);
    QCOMPARE(row0.at(0).toString(), QStringLiteral("1"));  // row label
    QCOMPARE(row0.at(5).toString(), QStringLiteral("38")); // E1
}

void TstModel::editingRecomputesDependents() {
    SpreadsheetModel model;

    // Change A1 from 15 to 115; every dependent total must move.
    model.setCell("A1", "115");

    QCOMPARE(model.display("A1"), QStringLiteral("115"));
    QCOMPARE(model.display("E1"), QStringLiteral("138")); // 115+3+12+8
    QCOMPARE(model.display("A5"), QStringLiteral("139")); // 115+8+12+4
    QCOMPARE(model.display("E5"), QStringLiteral("269")); // 138+51+45+35
}

void TstModel::formulaAndErrorPropagation() {
    SpreadsheetModel model;

    // A literal formula that divides by zero yields the engine's #DIV/0! error.
    model.setCell("A1", "=1/0");
    QVERIFY(model.valueJson("A1").contains(QStringLiteral("#DIV/0!")));

    // A binary operator over an error cell propagates the error (Excel-style).
    model.setCell("B1", "=A1+1");
    QVERIFY(model.valueJson("B1").contains(QStringLiteral("#DIV/0!")));
    QCOMPARE(model.display("B1"), QStringLiteral("#DIV/0!"));
}

QTEST_MAIN(TstModel)
#include "tst_model.moc"
