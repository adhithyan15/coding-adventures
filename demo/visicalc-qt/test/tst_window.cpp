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
    void fillReplicatesShiftingReferences();
    void clipboardCopyCutPaste();
    void saveLoadRoundTrips();
    void undoRedoWalksHistory();
    void structuralInsertDeleteShiftsReferences();
    void numberFormatAppliesToSelectedCell();
    void sortRangeReordersRowsByKeyColumn();
    void findAndReplaceLocatesAndRewritesCells();
    void multiSheetWorkbookAndCrossSheetRefs();
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
    QCOMPARE(at(w, 1, 1, 1, 1), QStringLiteral("15"));      // A1 (unformatted)
    // E1/E5 carry the "#,##0.00" seed format → the engine renders the formatted
    // display string (window() now reads sc_get_display_window).
    QCOMPARE(at(w, 1, 1, 1, 5), QStringLiteral("38.00"));   // E1 = SUM(A1:D1)
    QCOMPARE(at(w, 1, 1, 5, 5), QStringLiteral("169.00"));  // E5 grand total
}

void TstWindow::farWindowReachesZ1000AndGapsAreSparse() {
    SpreadsheetModel m;
    // Plant a far-flung formula 1000 rows down (Z = col 26).
    m.setCell("Z1000", "=SUM(A1:A4)"); // 15+8+12+4 = 39
    // A window around it returns the computed value — reachable without
    // materialising the millions of cells in between. Z1000 carries the "0.0%"
    // seed format, so 39 renders as "3900.0%": the format applies 1000 rows down.
    QCOMPARE(at(m.window(998, 24, 1002, 28), 998, 24, 1000, 26), QStringLiteral("3900.0%"));
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
    // And the recomputed value shows through a fresh window read: 115+8+12+4 =
    // 139, formatted as a percent ("0.0%") → "13900.0%".
    QCOMPARE(at(m.window(1000, 26, 1000, 26), 1000, 26, 1000, 26), QStringLiteral("13900.0%"));
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
    QCOMPARE(row1.at(0).toString(), QStringLiteral("15"));    // A1 (unformatted)
    QCOMPARE(row1.at(4).toString(), QStringLiteral("38.00")); // E1 = SUM(A1:D1), formatted
    QCOMPARE(row1.at(9).toString(), QString());               // J1 empty (sparse)
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
    QCOMPARE(m.rowCells(2).at(0).toString(), QStringLiteral("108"));    // A2 (unformatted)
    QCOMPARE(m.rowCells(2).at(4).toString(), QStringLiteral("151.00")); // E2 = 108+14+7+22, formatted
    QCOMPARE(m.rowCells(5).at(0).toString(), QStringLiteral("139.00")); // A5 = 15+108+12+4, formatted
    QCOMPARE(m.rowCells(5).at(4).toString(), QStringLiteral("269.00")); // E5 grand total, formatted
}

// Drag-fill (the "Fill down" control drives model.fill): replicate a formula
// down a column, each copy's relative reference tracking its row.
void TstWindow::fillReplicatesShiftingReferences() {
    SpreadsheetModel m;
    // Seed a fresh column away from the default budget: H1=2, H2=3, H3=4;
    // I1 = H1*10. Fill I1 down into I2:I3 — each tracks its row.
    m.setCell("H1", "2");
    m.setCell("H2", "3");
    m.setCell("H3", "4");
    m.setCell("I1", "=H1*10"); // 20
    m.fill("I1", "I2", "I3");
    // I2 = H2*10 = 30, I3 = H3*10 = 40 (each filled formula's relative ref tracked
    // its row). Read through a window over col 9 (= I); the values prove the shift.
    QCOMPARE(m.window(2, 9, 2, 9).at(0).toList().at(0).toString(), QStringLiteral("30"));
    QCOMPARE(m.window(3, 9, 3, 9).at(0).toList().at(0).toString(), QStringLiteral("40"));
    // The source cell is untouched by its own fill.
    QCOMPARE(m.window(1, 9, 1, 9).at(0).toList().at(0).toString(), QStringLiteral("20"));
}

// Clipboard (the Copy/Cut/Paste controls drive model.copy/cut/paste): copy a
// 1×2 block and paste it as a unit; cut a cell and move it.
void TstWindow::clipboardCopyCutPaste() {
    SpreadsheetModel m;
    m.setCell("H1", "5");
    m.setCell("I1", "=H1*2"); // 10 (col 8 = H, col 9 = I)
    // Copy the block H1:I1 and paste at H2 — the block shifts down one row.
    m.copy("H1", "I1");
    QVERIFY(m.paste("H2")); // applied
    QCOMPARE(m.window(2, 9, 2, 9).at(0).toList().at(0).toString(), QStringLiteral("10")); // I2 = H2*2
    // Cut a cell, move it, confirm the source clears and a second paste is a no-op.
    m.setCell("A1", "99");
    m.cut("A1", "A1");
    QVERIFY(m.paste("K1")); // col 11 = K
    QCOMPARE(m.window(1, 11, 1, 11).at(0).toList().at(0).toString(), QStringLiteral("99"));
    QCOMPARE(m.window(1, 1, 1, 1).at(0).toList().at(0).toString(), QString()); // A1 cleared
    QVERIFY(!m.paste("M1")); // buffer consumed
}

// Save / load (the Save / Load controls drive model.serialize/deserialize):
// serialize the workbook, scribble over it, then restore the snapshot and
// confirm the workbook comes back — and that a loaded formula stays LIVE.
void TstWindow::saveLoadRoundTrips() {
    SpreadsheetModel m;
    // Default seed: A1=15, E1 = SUM(A1:D1) = 38 (formatted "38.00").
    const QString snapshot = m.serialize();
    QVERIFY2(!snapshot.isEmpty(), "serialize produced a JSON document");

    // Mutate away from the saved state so a successful load has to undo it.
    m.setCell("A1", "500"); // E1 → 500+3+12+8 = 523
    QCOMPARE(m.window(1, 5, 1, 5).at(0).toList().at(0).toString(), QStringLiteral("523.00"));

    // Restore the snapshot: E1 recomputes through its format back to "38.00".
    QVERIFY(m.deserialize(snapshot));
    QCOMPARE(m.window(1, 1, 1, 1).at(0).toList().at(0).toString(), QStringLiteral("15"));  // A1 restored
    QCOMPARE(m.window(1, 5, 1, 5).at(0).toList().at(0).toString(), QStringLiteral("38.00")); // E1 formatted
    // The loaded formula is live, not frozen: edit a precedent and E1 recomputes.
    m.setCell("A1", "5"); // 5+3+12+8 = 28
    QCOMPARE(m.window(1, 5, 1, 5).at(0).toList().at(0).toString(), QStringLiteral("28.00"));
    // Garbage in is rejected (false), leaving the workbook intact.
    QVERIFY(!m.deserialize(QStringLiteral("not a workbook")));
    QCOMPARE(m.window(1, 5, 1, 5).at(0).toList().at(0).toString(), QStringLiteral("28.00"));
}

// Undo / redo (the Undo / Redo controls drive model.undo/redo): make two edits,
// walk history back and forward, and confirm a restored formula recomputes live.
void TstWindow::undoRedoWalksHistory() {
    SpreadsheetModel m;
    // (The model seeds its budget through set_cell, so those seed edits are
    // themselves undoable — history is non-empty from construction.)
    // Two fresh edits on a clear column: H1 = 2, I1 = H1*5 = 10 (col 8/9).
    m.setCell("H1", "2");
    m.setCell("I1", "=H1*5");
    QCOMPARE(m.window(1, 9, 1, 9).at(0).toList().at(0).toString(), QStringLiteral("10"));
    QVERIFY(m.canUndo());

    // Undo the formula, then the literal.
    QVERIFY(m.undo());
    QCOMPARE(m.window(1, 9, 1, 9).at(0).toList().at(0).toString(), QString()); // I1 gone
    QVERIFY(m.undo());
    QCOMPARE(m.window(1, 8, 1, 8).at(0).toList().at(0).toString(), QString()); // H1 gone

    // Redo both: I1 recomputes live (10).
    QVERIFY(m.canRedo());
    QVERIFY(m.redo());
    QVERIFY(m.redo());
    QCOMPARE(m.window(1, 9, 1, 9).at(0).toList().at(0).toString(), QStringLiteral("10"));
    QVERIFY(!m.canRedo());
    QVERIFY(!m.redo()); // nothing left to redo

    // A fresh edit forks history (drops the redo branch).
    QVERIFY(m.undo()); // back: I1 gone
    QVERIFY(m.canRedo());
    m.setCell("A1", "7");
    QVERIFY(!m.canRedo());
}

// Structural edits (the + Row / − Row / + Col / − Col controls drive
// model.insertRows/deleteRows/insertCols/deleteCols): inserting and deleting
// rows/columns shifts every formula reference across the band, and deleting a
// referenced band turns that reference into #REF!.
void TstWindow::structuralInsertDeleteShiftsReferences() {
    SpreadsheetModel m;
    // A fresh column away from the seeded budget: H1=10, H2=20, H3 = H1+H2 = 30.
    m.setCell("H1", "10");
    m.setCell("H2", "20");
    m.setCell("H3", "=H1+H2");
    QCOMPARE(m.window(3, 8, 3, 8).at(0).toList().at(0).toString(), QStringLiteral("30"));

    // The engine's formula printer parenthesizes binary ops ("=(H1+H3)"), so
    // compare the selected cell's source with the parens stripped.
    auto bareFormula = [&m]() { return QString(m.infFormula()).remove('(').remove(')'); };

    // Insert a row at row 2: H2/H3 shift down to H3/H4, row 2 is blank, and the
    // formula's refs shift with their cells (=H1+H2 → =H1+H3).
    m.insertRows(2, 1);
    QCOMPARE(m.window(2, 8, 2, 8).at(0).toList().at(0).toString(), QString());          // inserted row blank
    QCOMPARE(m.window(4, 8, 4, 8).at(0).toList().at(0).toString(), QStringLiteral("30")); // formula at H4
    m.selectInf(4, 8);
    QCOMPARE(bareFormula(), QStringLiteral("=H1+H3"));

    // Delete that inserted row: everything shifts back.
    m.deleteRows(2, 1);
    QCOMPARE(m.window(3, 8, 3, 8).at(0).toList().at(0).toString(), QStringLiteral("30"));
    m.selectInf(3, 8);
    QCOMPARE(bareFormula(), QStringLiteral("=H1+H2"));

    // Delete row 1 (referenced by the formula): H2 shifts up to H1, the formula
    // shifts up to H2, and its destroyed H1 reference becomes #REF!.
    m.deleteRows(1, 1);
    QCOMPARE(m.window(2, 8, 2, 8).at(0).toList().at(0).toString(), QStringLiteral("#REF!"));

    // Columns shift the same way: K1=5, L1 = K1*3 = 15. Insert a column at K and
    // the formula (now at M1) keeps pointing at its precedent (now L1).
    SpreadsheetModel m2;
    m2.setCell("K1", "5");
    m2.setCell("L1", "=K1*3");
    m2.insertCols(11, 1); // col 11 = K
    QCOMPARE(m2.window(1, 13, 1, 13).at(0).toList().at(0).toString(), QStringLiteral("15")); // M1
    m2.selectInf(1, 13);
    QCOMPARE(QString(m2.infFormula()).remove('(').remove(')'), QStringLiteral("=L1*3"));
}

// Number formatting (the .00 / % / $ / Gen controls drive model.setFormatInf):
// applying a format code changes only how the selected cell DISPLAYS; the stored
// value is unchanged. An empty code clears the format.
void TstWindow::numberFormatAppliesToSelectedCell() {
    SpreadsheetModel m;
    m.setCell("H1", "1234"); // col 8, away from the budget
    m.selectInf(1, 8);
    auto disp = [&m]() { return m.window(1, 8, 1, 8).at(0).toList().at(0).toString(); };
    QCOMPARE(disp(), QStringLiteral("1234")); // unformatted
    m.setFormatInf(QStringLiteral("#,##0.00"));
    QCOMPARE(disp(), QStringLiteral("1,234.00"));
    m.setFormatInf(QStringLiteral("0.0%"));
    QCOMPARE(disp(), QStringLiteral("123400.0%"));
    m.setFormatInf(QStringLiteral("$#,##0.00"));
    QCOMPARE(disp(), QStringLiteral("$1,234.00"));
    m.setFormatInf(QString()); // clear → General
    QCOMPARE(disp(), QStringLiteral("1234"));
    // The format is display-only: the stored source never changed.
    m.selectInf(1, 8);
    QCOMPARE(m.infFormula(), QStringLiteral("1234"));
}

// Range sort (the ▲/▼ Sort buttons): reorder the budget block A1:E4 by a key
// column. The default seed has column A = 15,8,12,4 (rows 1..4) and each E cell
// is =SUM(A:D) for its row. Sorting by column A ascending moves each row as a
// record — column A becomes 4,8,12,15 and every E total travels with its row
// (the engine shifts the moved SUM formulas' refs). Descending reverses it.
void TstWindow::sortRangeReordersRowsByKeyColumn() {
    SpreadsheetModel m;
    auto colA = [&m](int row) { return m.window(row, 1, row, 1).at(0).toList().at(0).toString(); };
    auto colE = [&m](int row) { return m.window(row, 5, row, 5).at(0).toList().at(0).toString(); };
    // Pre-sort seed order.
    QCOMPARE(colA(1), QStringLiteral("15"));
    QCOMPARE(colA(4), QStringLiteral("4"));

    // Ascending by column A (keyCol = 1): rows reorder to 4,8,12,15.
    QVERIFY(m.sortRange("A1", "E4", 1, true));
    QCOMPARE(colA(1), QStringLiteral("4"));
    QCOMPARE(colA(2), QStringLiteral("8"));
    QCOMPARE(colA(3), QStringLiteral("12"));
    QCOMPARE(colA(4), QStringLiteral("15"));
    // Each row's E total tracked its row (E = SUM of that row's A..D), formatted.
    QCOMPARE(colE(1), QStringLiteral("35.00"));  // 4+11+3+17
    QCOMPARE(colE(4), QStringLiteral("38.00"));  // 15+3+12+8

    // Descending reverses the key order.
    QVERIFY(m.sortRange("A1", "E4", 1, false));
    QCOMPARE(colA(1), QStringLiteral("15"));
    QCOMPARE(colA(4), QStringLiteral("4"));

    // Bad args are a no-op returning false (no crash).
    QVERIFY(!m.sortRange("A1", "A1", 1, true));   // single-row range
    QVERIFY(!m.sortRange("A1", "E4", 9, true));   // key column outside the range
}

// Find / replace (the Find / Replace-all toolbar group drives findMatches /
// replaceAll): locate cells by text and bulk-edit their source. Seed two number
// cells + a formula referencing one of them.
void TstWindow::findAndReplaceLocatesAndRewritesCells() {
    SpreadsheetModel m;
    // Use a token (555) absent from the default budget seed (whose far cell
    // "=Z1000*2" would otherwise also contain "100").
    m.setCell("H1", "555");
    m.setCell("H2", "555");
    m.setCell("I1", "=H1+1"); // displays 556
    // find by computed value: "555" is the display of H1 and H2.
    const QStringList byVal = m.findMatches("555", false, false);
    QVERIFY(byVal.contains("H1") && byVal.contains("H2"));
    // find by source: "H1" only in I1's formula text.
    QCOMPARE(m.findMatches("H1", true, false), QStringList{"I1"});
    // empty query → no matches.
    QVERIFY(m.findMatches("", false, false).isEmpty());
    // replace the literal 555 → 7 in the two number cells (count 2).
    QCOMPARE(m.replaceAll("555", "7", false), 2);
    QCOMPARE(at(m.window(1, 8, 1, 8), 1, 8, 1, 8), QStringLiteral("7"));  // H1
    // replace H1 → H2 in the formula source; it re-parses + recomputes (=H2+1 = 8).
    QCOMPARE(m.replaceAll("H1", "H2", false), 1);
    QCOMPARE(at(m.window(1, 9, 1, 9), 1, 9, 1, 9), QStringLiteral("8"));  // I1
    // no-match / empty query → 0.
    QCOMPARE(m.replaceAll("zzz", "q", false), 0);
    QCOMPARE(m.replaceAll("", "q", false), 0);
}

// Multi-sheet workbook + cross-sheet references: the model seeds Sheet1 (active)
// plus a Summary sheet (A3 = A1+A2 = 300). A formula on Sheet1 can reference it
// (=Summary!A3) and recompute live; add/rename/delete reindex + rewrite/#REF!.
void TstWindow::multiSheetWorkbookAndCrossSheetRefs() {
    SpreadsheetModel m;
    QCOMPARE(m.sheetNames().value("sheets").toStringList(),
             (QStringList{"Sheet1", "Summary"}));
    QCOMPARE(m.activeSheet(), 0);

    // Cross-sheet ref on Sheet1: G1 = =Summary!A3 → 300.
    m.setCell("G1", "=Summary!A3");
    QCOMPARE(at(m.window(1, 7, 1, 7), 1, 7, 1, 7), QStringLiteral("300"));
    // Switch to Summary, edit A1 100→500 (A3 → 700), switch back → recompute live.
    QVERIFY(m.selectSheet(1));
    m.setCell("A1", "500");
    QVERIFY(m.selectSheet(0));
    QCOMPARE(at(m.window(1, 7, 1, 7), 1, 7, 1, 7), QStringLiteral("700"));

    // Add a sheet → becomes active; duplicate/empty names rejected.
    QVERIFY(m.addSheet("Detail"));
    QCOMPARE(m.activeSheet(), 2);
    QVERIFY(!m.addSheet("Sheet1"));
    QVERIFY(!m.addSheet(""));

    // Rename Summary → Totals: the qualifier follows, value holds.
    QVERIFY(m.selectSheet(0));
    QVERIFY(m.renameSheet(1, "Totals"));
    QCOMPARE(m.sheetNames().value("sheets").toStringList(),
             (QStringList{"Sheet1", "Totals", "Detail"}));
    QCOMPARE(at(m.window(1, 7, 1, 7), 1, 7, 1, 7), QStringLiteral("700"));

    // Delete Totals → the inbound ref becomes #REF!; can't delete the last sheet.
    QVERIFY(m.deleteSheet(1));
    QCOMPARE(m.sheetNames().value("sheets").toStringList(),
             (QStringList{"Sheet1", "Detail"}));
    QVERIFY(m.valueJson("G1").contains("#REF!"));

    // Save/load round-trips a second sheet + a live cross-sheet formula.
    QVERIFY(m.addSheet("Data"));
    m.setCell("A1", "9"); // Data!A1
    QVERIFY(m.selectSheet(0));
    m.setCell("F1", "=Data!A1"); // 9
    const QString docJson = m.serialize();
    SpreadsheetModel m2;
    QVERIFY(m2.deserialize(docJson));
    QVERIFY(m2.sheetNames().value("sheets").toStringList().contains("Data"));
    QCOMPARE(at(m2.window(1, 6, 1, 6), 1, 6, 1, 6), QStringLiteral("9"));
}

QTEST_MAIN(TstWindow)
#include "tst_window.moc"
