import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_upc_a/upc_a.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 131072;

String _sha256Hex(String value) {
  return sha256.convert(utf8.encode(value)).toString();
}

File _locateCasesFixture() {
  var dir = Directory.current;
  while (true) {
    final candidate1 = File(
      '${dir.path}/code/specs/fixtures/barcode-symbologies-v1/cases.json',
    );
    if (candidate1.existsSync()) return candidate1;
    final candidate2 = File(
      '${dir.path}/specs/fixtures/barcode-symbologies-v1/cases.json',
    );
    if (candidate2.existsSync()) return candidate2;
    final parent = dir.parent;
    if (parent.path == dir.path) break;
    dir = parent;
  }
  throw StateError('Could not find barcode-symbologies-v1 cases.json');
}

Map<String, dynamic> _loadCases() {
  final file = _locateCasesFixture();
  final raf = file.openSync(mode: FileMode.read);
  try {
    final bytes = raf.readSync(_maxFixtureBytes + 1);
    if (bytes.length > _maxFixtureBytes) {
      throw StateError('Fixture file exceeds byte limit: ${bytes.length}');
    }
    final content = utf8.decode(bytes);
    return jsonDecode(content) as Map<String, dynamic>;
  } finally {
    raf.closeSync();
  }
}

String _materializeInput(Map<String, dynamic> input) {
  if (input.containsKey('text')) {
    return input['text'] as String;
  }
  final repeat = input['repeat'] as Map<String, dynamic>;
  final text = repeat['text'] as String;
  final count = repeat['count'] as int;
  return text * count;
}

List<int> _computeRunLengths(String modules) {
  final runs = <int>[];
  if (modules.isEmpty) return runs;

  var current = modules[0];
  var count = 1;
  for (var i = 1; i < modules.length; i++) {
    final ch = modules[i];
    if (ch == current) {
      count++;
    } else {
      runs.add(count);
      current = ch;
      count = 1;
    }
  }
  runs.add(count);
  return runs;
}

void main() {
  final fixture = _loadCases();
  final allCases = fixture['cases'] as List<dynamic>;
  final upcACases = allCases
      .where((c) => (c as Map<String, dynamic>)['symbology'] == 'upc-a')
      .cast<Map<String, dynamic>>()
      .toList();

  group('UPC-A Conformance Tests', () {
    test('at least 12 UPC-A cases present', () {
      expect(upcACases.length, greaterThanOrEqualTo(12));
    });

    for (final testCase in upcACases) {
      final id = testCase['id'] as String;
      final input = _materializeInput(
        testCase['input'] as Map<String, dynamic>,
      );
      final expected = testCase['expected'] as Map<String, dynamic>;

      test(id, () {
        if (expected.containsKey('error')) {
          final expectedError = expected['error'] as String;
          expect(
            () => normalizeUpcA(input),
            throwsA(
              isA<InvalidUpcAInputError>().having(
                (e) => e.errorId,
                'errorId',
                equals(expectedError),
              ),
            ),
          );
          return;
        }

        final normalized = normalizeUpcA(input);
        final encoded = encodeUpcA(input);
        // Rebuild the module string the way the symbology lays out:
        // start guard + six L digits + center + six R digits + end guard.
        final rebuilt =
            '${upcASideGuard}${encoded.sublist(0, 6).map((d) => d.pattern).join()}'
            '$upcACenterGuard'
            '${encoded.sublist(6).map((d) => d.pattern).join()}$upcASideGuard';
        final runs = _computeRunLengths(rebuilt);

        expect(normalized, equals(expected['normalized']));
        expect(rebuilt, equals(expected['modules']));
        expect(runs, equals((expected['run_lengths'] as List).cast<int>()));
        expect(normalized[11], equals(expected['checksum']));
        expect(rebuilt.length, equals(expected['module_count']));
        expect(_sha256Hex(rebuilt), equals(expected['module_sha256']));
      });
    }
  });
}
