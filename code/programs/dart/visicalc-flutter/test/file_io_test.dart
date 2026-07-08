// file_io_test.dart — headless proof that the Flutter VisiCalc demo can open and
// save real spreadsheet FILES over the shared Rust engine, with no widgets in
// the loop. The Dart sibling of the C-ABI's own round-trip tests: it drives the
// dart:ffi byte bindings (sc_load_*/sc_save_*/sc_bytes_free) end to end.
//
// Run (after `bash scripts/build.sh` has vendored native/libspreadsheet_capi.*):
//   flutter test test/file_io_test.dart
//
// A green run proves the Dart ↔ C ABI ↔ Rust byte path: raw file bytes (a ZIP
// for .xlsx, an OLE2 file for .xls, text for CSV/TSV/JSON) cross the boundary
// intact — including bytes a NUL-terminated string would have truncated.

import 'dart:typed_data' show Uint8List;

import 'package:flutter_test/flutter_test.dart';
import 'package:visicalc_flutter/engine.dart';

void main() {
  // A tiny workbook with one live formula, so we can tell a values-only codec
  // (CSV/TSV/JSON/.xls) from a formula-preserving one (.xlsx) after a round trip.
  SpreadsheetSession seeded() {
    final s = SpreadsheetSession();
    s.setCell('A1', '15');
    s.setCell('B1', '3');
    s.setCell('C1', '=A1+B1'); // 18
    return s;
  }

  group('file open / save over the Rust engine (dart:ffi bytes)', () {
    test('.xlsx save is a real ZIP and reload keeps the live formula', () {
      final src = seeded();
      addTearDown(src.dispose);
      final bytes = src.saveXlsx();

      // A non-empty buffer beginning with the ZIP local-file magic "PK\x03\x04"
      // — proof the raw binary crossed FFI without NUL truncation.
      expect(bytes.length, greaterThan(4));
      expect(bytes.sublist(0, 4), <int>[0x50, 0x4B, 0x03, 0x04]);

      final dst = SpreadsheetSession();
      addTearDown(dst.dispose);
      expect(dst.loadXlsx(bytes), isTrue);
      expect(dst.getRaw('C1'), '=A1+B1'); // formula survives .xlsx
      expect(dst.display('C1'), '18'); // and still computes
    });

    test('.xls save is a real OLE2 file (0xD0CF magic) and reloads values', () {
      final src = seeded();
      addTearDown(src.dispose);
      final bytes = src.saveXls();

      expect(bytes.length, greaterThan(8));
      // OLE2 / Compound File Binary signature D0 CF 11 E0 — the high bit in 0xD0
      // is exactly what a lossy UTF-8 string round trip would have mangled.
      expect(bytes.sublist(0, 4), <int>[0xD0, 0xCF, 0x11, 0xE0]);

      final dst = SpreadsheetSession();
      addTearDown(dst.dispose);
      expect(dst.loadXls(bytes), isTrue);
      expect(dst.display('C1'), '18'); // values-only, but the value is right
    });

    test('CSV / TSV / JSON round-trip a header + data row', () {
      // These three are values-only tabular codecs. JSON's canonical shape is an
      // array of objects, so row 1 is the HEADER (the keys) and row 2 the first
      // data record; CSV/TSV are positional grids. A header row + one data row
      // round-trips consistently through all three: the header lands back on
      // row 1, the data on row 2.
      SpreadsheetSession table() {
        final s = SpreadsheetSession();
        s.setCell('A1', 'qty'); // header labels (text)
        s.setCell('B1', 'unit');
        s.setCell('C1', 'total');
        s.setCell('A2', '15'); // one data row
        s.setCell('B2', '3');
        s.setCell('C2', '=A2*B2'); // 45 — a formula the codec stores as its value
        return s;
      }

      for (final format in const ['csv', 'tsv', 'json']) {
        final src = table();
        final bytes = src.exportBytesForFormat(format);
        src.dispose();
        expect(bytes.isNotEmpty, isTrue, reason: '$format save produced bytes');

        final dst = SpreadsheetSession();
        expect(dst.loadForFormat(format, bytes), isTrue,
            reason: '$format reopened');
        expect(dst.display('A1'), 'qty', reason: '$format header round-tripped');
        expect(dst.display('C2'), '45', reason: '$format value round-tripped');
        dst.dispose();
      }
    });

    test('a bad payload is rejected and leaves the workbook untouched', () {
      final s = seeded();
      addTearDown(s.dispose);
      // Not a ZIP — .xlsx open must fail (return false) without disturbing C1.
      expect(s.loadXlsx(Uint8List.fromList('not a spreadsheet'.codeUnits)),
          isFalse);
      expect(s.display('C1'), '18');
      // Empty input is a no-op false, too.
      expect(s.loadCsv(Uint8List(0)), isFalse);
      expect(s.display('C1'), '18');
    });

    test('the model exposes format-parameterised export / import', () {
      final model = InfiniteSheetModel();
      addTearDown(model.dispose);
      for (final format in InfiniteSheetModel.fileFormats) {
        final bytes = model.exportBytes(format);
        expect(bytes.isNotEmpty, isTrue, reason: '$format export');
        expect(model.importBytes(format, bytes), isTrue,
            reason: '$format import');
      }
      // An unknown format is a safe no-op both ways.
      expect(model.exportBytes('numbers').isEmpty, isTrue);
      expect(model.importBytes('numbers', Uint8List.fromList([1, 2, 3])),
          isFalse);
    });
  });
}

// Small test-only shims so the round-trip loop can pick a codec by name without
// re-implementing the switch the model already has.
extension _FormatDispatch on SpreadsheetSession {
  Uint8List exportBytesForFormat(String format) => switch (format) {
        'xlsx' => saveXlsx(),
        'xls' => saveXls(),
        'csv' => saveCsv(),
        'tsv' => saveTsv(),
        'json' => saveJson(),
        _ => Uint8List(0),
      };

  bool loadForFormat(String format, Uint8List bytes) => switch (format) {
        'xlsx' => loadXlsx(bytes),
        'xls' => loadXls(bytes),
        'csv' => loadCsv(bytes),
        'tsv' => loadTsv(bytes),
        'json' => loadJson(bytes),
        _ => false,
      };
}
