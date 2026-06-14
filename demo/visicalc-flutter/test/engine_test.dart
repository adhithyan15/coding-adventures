// engine_test.dart — headless proof that the Flutter VisiCalc demo does REAL
// formula work on the shared Rust engine, with no widgets in the loop. This is
// the Flutter sibling of the SwiftUI demo's `swift test` and the Qt demo's
// tst_model: it loads the vendored engine dynamic library through dart:ffi and
// asserts the values are engine-computed and recompute on edit.
//
// Run (after `bash scripts/build.sh` has vendored native/libspreadsheet_capi.*):
//   flutter test
//
// `flutter test` executes on the host Dart VM, where DynamicLibrary.open loads
// the host engine slice — so a green run proves the Dart ↔ C ABI ↔ Rust path.

import 'package:flutter_test/flutter_test.dart';
import 'package:visicalc_flutter/engine.dart';

void main() {
  group('SpreadsheetModel on the Rust engine', () {
    test('seeded totals are engine-computed', () {
      final model = SpreadsheetModel();
      addTearDown(model.dispose);

      // Row totals (column E = SUM of A..D), column totals (row 5), grand total.
      expect(model.viewportRows[0][5], '38'); // E1 = 15+3+12+8
      expect(model.viewportRows[1][5], '51'); // E2 = 8+14+7+22
      expect(model.viewportRows[4][1], '39'); // A5 = 15+8+12+4
      expect(model.viewportRows[4][5], '169'); // E5 = grand total
      // The leading column is the row-label gutter.
      expect(model.viewportRows[0][0], '1');
    });

    test('editing an input recomputes every dependent', () {
      final model = SpreadsheetModel();
      addTearDown(model.dispose);

      // A1 is display row 0, column 1. Change 15 -> 115.
      model.setCell(0, 1, '115');

      expect(model.viewportRows[0][1], '115'); // A1
      expect(model.viewportRows[0][5], '138'); // E1 = 115+3+12+8
      expect(model.viewportRows[4][1], '139'); // A5 = 115+8+12+4
      expect(model.viewportRows[4][5], '269'); // E5 = 138+51+45+35
    });

    test('formula entry computes, and a binary-op error propagates', () {
      final model = SpreadsheetModel();
      addTearDown(model.dispose);

      // Division by zero yields the engine's #DIV/0! error.
      model.setCell(0, 1, '=1/0'); // A1
      expect(model.valueJson('A1'), contains('#DIV/0!'));

      // A binary operator over an error cell propagates the error (Excel-style).
      model.setCell(0, 2, '=A1+1'); // B1
      expect(model.valueJson('B1'), contains('#DIV/0!'));
      expect(model.viewportRows[0][2], '#DIV/0!');
    });
  });
}
