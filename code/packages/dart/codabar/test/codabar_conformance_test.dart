import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_codabar/codabar.dart';
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

/// Rebuilds the module string from expanded runs, gaps included.
String _modulesOf(String input, String? start, String? stop) {
  final buffer = StringBuffer();
  for (final run in expandCodabarRuns(input, start: start, stop: stop)) {
    buffer.write((run.color == 'bar' ? '1' : '0') * run.modules);
  }
  return buffer.toString();
}

void main() {
  final fixture = _loadCases();
  final allCases = fixture['cases'] as List<dynamic>;
  final codabarCases = allCases
      .where((c) => (c as Map<String, dynamic>)['symbology'] == 'codabar')
      .cast<Map<String, dynamic>>()
      .toList();

  group('Codabar Conformance Tests', () {
    test('at least 19 Codabar cases present', () {
      expect(codabarCases.length, greaterThanOrEqualTo(19));
    });

    for (final testCase in codabarCases) {
      final id = testCase['id'] as String;
      final input = _materializeInput(
        testCase['input'] as Map<String, dynamic>,
      );
      final caseOptions = testCase['options'] as Map<String, dynamic>? ?? {};
      final start = caseOptions['start'] as String?;
      final stop = caseOptions['stop'] as String?;
      final expected = testCase['expected'] as Map<String, dynamic>;

      test(id, () {
        if (expected.containsKey('error')) {
          final expectedError = expected['error'] as String;
          expect(
            () => normalizeCodabar(input, start: start, stop: stop),
            throwsA(
              isA<InvalidCodabarInputError>().having(
                (e) => e.errorId,
                'errorId',
                equals(expectedError),
              ),
            ),
          );
          return;
        }

        final normalized = normalizeCodabar(input, start: start, stop: stop);
        final modules = _modulesOf(input, start, stop);
        final runs = _computeRunLengths(modules);

        if (expected.containsKey('normalized')) {
          expect(normalized, equals(expected['normalized']));
          expect(modules, equals(expected['modules']));
          expect(runs, equals((expected['run_lengths'] as List).cast<int>()));
        } else {
          expect(
            _sha256Hex(normalized),
            equals(expected['normalized_sha256']),
          );
          expect(runs.length, equals(expected['run_count']));
          expect(
            _sha256Hex(jsonEncode(runs)),
            equals(expected['run_lengths_sha256']),
          );
        }

        expect(modules.length, equals(expected['module_count']));
        expect(_sha256Hex(modules), equals(expected['module_sha256']));
      });
    }
  });
}
