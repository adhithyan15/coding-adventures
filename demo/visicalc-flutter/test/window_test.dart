// window_test.dart — headless proof that the Flutter demo can drive a
// VIRTUALIZED infinite sheet on the engine's viewport primitive (via dart:ffi
// over the C ABI's sc_get_window / sc_used_range / sc_changed_since), with no
// widgets. The Flutter sibling of the SwiftUI WindowedModelTests and the web
// demo's verify-infinite.mjs.
//
// Run (after `bash scripts/build.sh` vendors native/libspreadsheet_capi.*):
//   flutter test test/window_test.dart

import 'package:flutter_test/flutter_test.dart';
import 'package:visicalc_flutter/engine.dart';

void main() {
  group('viewport primitive over dart:ffi', () {
    SpreadsheetSession seed() {
      final s = SpreadsheetSession();
      // Cross-foot budget + a far-flung formula at Z1000 (row 1000, col 26).
      const cells = {
        'A1': '15', 'B1': '3', 'C1': '12', 'D1': '8', 'E1': '=SUM(A1:D1)',
        'A2': '8', 'B2': '14', 'C2': '7', 'D2': '22', 'E2': '=SUM(A2:D2)',
        'A3': '12', 'B3': '9', 'C3': '18', 'D3': '6', 'E3': '=SUM(A3:D3)',
        'A4': '4', 'B4': '11', 'C4': '3', 'D4': '17', 'E4': '=SUM(A4:D4)',
        'A5': '=SUM(A1:A4)', 'E5': '=SUM(E1:E4)',
        'Z1000': '=SUM(A1:A4)', // 15+8+12+4 = 39
      };
      cells.forEach(s.setCell);
      return s;
    }

    test('window is engine-computed and dense', () {
      final s = seed();
      addTearDown(s.dispose);
      final w = s.window(1, 1, 5, 5);
      expect(w[0][0], '15'); // A1
      expect(w[0][4], '38'); // E1 = SUM(A1:D1)
      expect(w[4][4], '169'); // E5 grand total
      expect(w[0][1], '3'); // dense — B1
    });

    test('far window reaches Z1000 and the gaps are sparse', () {
      final s = seed();
      addTearDown(s.dispose);
      // Around Z1000 (row 1000, col 26).
      expect(s.window(998, 24, 1002, 28)[2][2], '39');
      // The gap between the two data islands is empty.
      for (final row in s.window(100, 1, 110, 10)) {
        for (final cell in row) {
          expect(cell, '');
        }
      }
    });

    test('extent, column letters and changed-since diff', () {
      final s = seed();
      addTearDown(s.dispose);
      final u = s.usedRange()!;
      expect(u['maxRow'], 1000);
      expect(u['maxCol'], 26);
      expect(s.columnLetters(27), 'AA');
      expect(s.columnLetters(53), 'BA');

      final rev = s.currentRevision();
      s.setCell('A1', '115');
      final diff = s.changedSince(rev);
      expect(diff.stale, isFalse);
      expect(diff.changed, contains('A1'));
      expect(diff.changed, contains('Z1000')); // far dependent recomputed
      expect(s.window(1000, 26, 1000, 26)[0][0], '139'); // 115+8+12+4
    });

    test('undo / redo walks the snapshot history with live recompute', () {
      // A fresh, unseeded session so the initial history is empty.
      final s = SpreadsheetSession();
      addTearDown(s.dispose);
      expect(s.canUndo(), isFalse);
      s.setCell('A1', '1');
      s.setCell('B1', '=A1*10'); // 10
      expect(s.canUndo(), isTrue);

      // Undo the formula, then the literal.
      expect(s.undo(), isTrue);
      expect(s.window(1, 2, 1, 2)[0][0], ''); // B1 cleared
      expect(s.undo(), isTrue);
      expect(s.window(1, 1, 1, 1)[0][0], ''); // A1 cleared
      expect(s.canUndo(), isFalse);
      expect(s.undo(), isFalse); // nothing left to undo

      // Redo both: B1 recomputes live (10).
      expect(s.redo(), isTrue);
      expect(s.redo(), isTrue);
      expect(s.window(1, 2, 1, 2)[0][0], '10');
      expect(s.canRedo(), isFalse);

      // A fresh edit forks history (drops the redo branch).
      s.undo(); // back: B1 gone
      expect(s.canRedo(), isTrue);
      s.setCell('C1', '9');
      expect(s.canRedo(), isFalse);
    });
  });

  // The infinite-view binding layer (InfiniteGrid drives these): one engine read
  // per visible row via rowCells, tap-to-select via selectInf (loading the
  // cell's source into the formula bar), and write-through via commitInf.
  group('InfiniteSheetModel', () {
    test('extent grows to reach the far seeded cells', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      // The seed plants Z1000 (row 1000) and BB50 (col 54), so the extent spans
      // both far islands plus the default margins.
      expect(m.totalRows, greaterThanOrEqualTo(1000));
      expect(m.totalCols, greaterThanOrEqualTo(60));
    });

    test('rowCells is one engine-read row, dense then sparse', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      final row1 = m.rowCells(1);
      expect(row1.length, m.totalCols);
      expect(row1[0], '15'); // A1 (unformatted)
      // E1 carries the "#,##0.00" seed format → engine renders the formatted
      // display string (rowCells now reads sc_get_display_window).
      expect(row1[4], '38.00'); // E1 = SUM(A1:D1), formatted
      expect(row1[9], ''); // J1 empty (sparse)
      // A row in the gap between the data islands is entirely blank.
      expect(m.rowCells(200).every((c) => c.isEmpty), isTrue);
    });

    test('selectInf loads the source and clamps to the grid', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      m.selectInf(5, 1); // A5 is a formula — the bar shows the formula, not 39
      expect(m.infAddress, 'A5');
      expect(m.formula, '=SUM(A1:A4)');
      m.selectInf(-3, 0); // clamps to (1, 1)
      expect(m.selRow, 1);
      expect(m.selCol, 1);
    });

    test('commitInf writes through and recomputes dependents', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      m.selectInf(2, 1); // A2
      m.commitInf('108'); // 8 -> 108
      expect(m.rowCells(2)[0], '108'); // A2 (unformatted)
      expect(m.rowCells(2)[4], '151.00'); // E2 = 108+14+7+22, formatted
      expect(m.rowCells(5)[0], '139.00'); // A5 = 15+108+12+4, formatted
      expect(m.rowCells(5)[4], '269.00'); // E5 grand total, formatted
    });

    test('fillDown replicates the selected cell, shifting relative refs', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      // Seed a fresh column via select+commit: H1=2, H2=3, H3=4 (col 8 = H);
      // I1 = H1*10 (col 9 = I). Select I1 and fill down 10 — each filled formula
      // tracks its row (I2 = H2*10 = 30, …).
      m.selectInf(1, 8); m.commitInf('2'); // H1
      m.selectInf(2, 8); m.commitInf('3'); // H2
      m.selectInf(3, 8); m.commitInf('4'); // H3
      m.selectInf(1, 9); m.commitInf('=H1*10'); // I1 = 20
      m.selectInf(1, 9);
      m.fillDown(10);
      expect(m.rowCells(2)[8], '30'); // I2 = H2*10
      expect(m.rowCells(3)[8], '40'); // I3 = H3*10
      expect(m.rowCells(1)[8], '20'); // I1 source untouched
    });

    test('clipboard copyCell/cutCell/pasteCell shifts a formula and moves a cut', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      // Seed H1=5, H2=7; I1 = H1*2 (col 8 = H, col 9 = I). Copy I1, then paste at
      // I2 — the relative ref shifts by the destination's offset, so I2 = H2*2.
      m.selectInf(1, 8); m.commitInf('5'); // H1
      m.selectInf(2, 8); m.commitInf('7'); // H2
      m.selectInf(1, 9); m.commitInf('=H1*2'); // I1 = 10
      m.selectInf(1, 9); m.copyCell(); // copy I1
      m.selectInf(2, 9); expect(m.pasteCell(), isTrue); // paste at I2
      expect(m.rowCells(2)[8], '14'); // I2 = H2*2 = 14
      // Cut A1, move it to C1: source clears, a second paste is a no-op.
      m.selectInf(1, 1); m.commitInf('99'); // A1
      m.selectInf(1, 1); m.cutCell();
      m.selectInf(1, 3); expect(m.pasteCell(), isTrue); // paste at C1
      expect(m.rowCells(1)[2], '99'); // C1 moved
      expect(m.rowCells(1)[0], ''); // A1 cleared
      m.selectInf(1, 5); expect(m.pasteCell(), isFalse); // buffer consumed
    });

    test('saveBook/loadBook round trips and keeps formulas live', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      // Default seed: A1=15, E1 = SUM(A1:D1) = 38 (formatted "38.00").
      final snapshot = m.saveBook();
      expect(snapshot, isNotEmpty);
      // Mutate away from the saved state so a load has to visibly undo it.
      m.selectInf(1, 1); m.commitInf('500'); // E1 → 500+3+12+8 = 523
      expect(m.rowCells(1)[4], '523.00');
      // Restore: A1 → 15, E1 recomputes through its format back to "38.00".
      expect(m.loadBook(snapshot), isTrue);
      expect(m.rowCells(1)[0], '15');
      expect(m.rowCells(1)[4], '38.00');
      // The loaded formula is live, not frozen: edit a precedent and E1 recomputes.
      m.selectInf(1, 1); m.commitInf('5'); // 5+3+12+8 = 28
      expect(m.rowCells(1)[4], '28.00');
      // Garbage in is rejected (false), leaving the workbook intact.
      expect(m.loadBook('not a workbook'), isFalse);
      expect(m.rowCells(1)[4], '28.00');
    });

    test('undoEdit/redoEdit reverse and replay a model edit', () {
      final m = InfiniteSheetModel();
      addTearDown(m.dispose);
      // (The model seeds its budget via commitInf, so history is non-empty from
      // construction — undoing into the seed is expected.) Make one fresh edit.
      m.selectInf(1, 8); m.commitInf('=A1+1'); // H1 = 15+1 = 16
      expect(m.rowCells(1)[7], '16');
      expect(m.canUndo, isTrue);
      // Undo it: H1 goes away; redo brings it back, recomputed live.
      expect(m.undoEdit(), isTrue);
      expect(m.rowCells(1)[7], '');
      expect(m.canRedo, isTrue);
      expect(m.redoEdit(), isTrue);
      expect(m.rowCells(1)[7], '16');
    });
  });
}
