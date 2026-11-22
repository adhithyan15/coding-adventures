import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_sql_lexer/sql_lexer.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling
const String _expectedSqlTokensSha256 =
    'c2d117a3ca1b300291eadb00c93cbedcc110dccd695fe1f57b3ea69a4b81accf';

File _locateFixture(String relativePath) {
  var dir = Directory.current;
  for (var i = 0; i < 6; i++) {
    final candidate = File('${dir.path}/$relativePath');
    if (candidate.existsSync()) {
      return candidate;
    }
    final parent = dir.parent;
    if (parent.path == dir.path) break;
    dir = parent;
  }
  throw StateError('Could not find fixture: $relativePath');
}

List<int> _loadBoundedBytes(String relativePath) {
  final file = _locateFixture(relativePath);
  final raf = file.openSync(mode: FileMode.read);
  try {
    final bytes = raf.readSync(_maxFixtureBytes + 1);
    if (bytes.length > _maxFixtureBytes) {
      throw StateError('Fixture file exceeds byte limit: ${bytes.length}');
    }
    return bytes;
  } finally {
    raf.closeSync();
  }
}

Map<String, dynamic> _loadBoundedJson(String relativePath) {
  final bytes = _loadBoundedBytes(relativePath);
  final content = utf8.decode(bytes);
  return jsonDecode(content) as Map<String, dynamic>;
}

void main() {
  group('SQL Lexer Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/cases.json');
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'sql')
          .toList();

      expect(cases, isNotEmpty,
          reason: 'Expected at least one canonical SQL case');

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final version = testCase['version'] as String;
        final source = testCase['source'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final tokens = tokenizeSql(source, version: version);
          final expectedTokens = (testCase['tokens'] as List<dynamic>)
              .cast<Map<String, dynamic>>();

          expect(tokens.length, equals(expectedTokens.length),
              reason: 'Token count mismatch in case $caseId');

          for (var i = 0; i < tokens.length; i++) {
            final actual = tokens[i];
            final expected = expectedTokens[i];
            expect(actual.type, equals(expected['type']),
                reason: 'Type mismatch at token $i in case $caseId');
            expect(actual.value, equals(expected['value']),
                reason: 'Value mismatch at token $i in case $caseId');
            expect(actual.line, equals(expected['line']),
                reason: 'Line mismatch at token $i in case $caseId');
            expect(actual.column, equals(expected['column']),
                reason: 'Column mismatch at token $i in case $caseId');
          }
        }
      }
    });

    test('Grammar SHA-256 digest matches language-frontends-v1 grammars.json',
        () {
      final grammarsData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/grammars.json');
      final grammarEntries = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'sql' && g['kind'] == 'tokens')
          .toList();

      expect(grammarEntries, isNotEmpty);
      final entry = grammarEntries.first;
      expect(entry['sha256'], equals(_expectedSqlTokensSha256));

      final tokenBytes = _loadBoundedBytes(entry['path'] as String);
      final actualSha256 = sha256.convert(tokenBytes).toString();
      expect(actualSha256, equals(_expectedSqlTokensSha256));
    });
  });
}
