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
}
