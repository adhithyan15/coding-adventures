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
  });
}
