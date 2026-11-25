import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_csharp_lexer/csharp_lexer.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling

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
  group('C# Lexer Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/cases.json');
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'csharp')
          .toList();

      expect(cases, isNotEmpty,
          reason: 'Expected at least one canonical csharp case');

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final version = testCase['version'] as String;
        final source = testCase['source'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final tokens = tokenizeCSharp(source, version: version);
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
        } else if (outcome == 'error') {
          expect(
            () => tokenizeCSharp(source, version: version),
            throwsA(isA<CSharpLexerException>()),
            reason: 'Expected lexical error in case $caseId',
          );
        } else {
          fail('Unsupported outcome: $outcome in case $caseId');
        }
      }
    });

    test('All 12 C# .tokens files match SHA-256 digests in grammars.json', () {
      final grammarsData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/grammars.json');
      final grammars = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'csharp' && g['kind'] == 'tokens')
          .toList();

      expect(grammars.length, equals(12));

      for (final entry in grammars) {
        final relPath = entry['path'] as String;
        final expectedSha = entry['sha256'] as String;
        final bytes = _loadBoundedBytes(relPath);
        final actualSha = sha256.convert(bytes).toString();
        expect(actualSha, equals(expectedSha),
            reason: 'SHA-256 mismatch for $relPath');
      }
    });

    test('Edition-specific features across all 12 C# editions', () {
      // 1.0: base keywords
      final t1_0 = tokenizeCSharp('class Foo { int x = 1; }', version: '1.0');
      expect(
          t1_0.any((t) => t.type == 'KEYWORD' && t.value == 'class'), isTrue);
      expect(t1_0.any((t) => t.type == 'KEYWORD' && t.value == 'int'), isTrue);
      final t1_0_yield = tokenizeCSharp('int yield = 5;', version: '1.0');
      expect(t1_0_yield.any((t) => t.type == 'NAME' && t.value == 'yield'),
          isTrue);

      // 2.0: yield keyword
      final t2_0 = tokenizeCSharp('yield return 1;', version: '2.0');
      expect(
          t2_0.any((t) => t.type == 'KEYWORD' && t.value == 'yield'), isTrue);

      // 3.0: lambda =>
      final t3_0 = tokenizeCSharp('x => x + 1', version: '3.0');
      expect(t3_0.any((t) => t.type == 'LAMBDA' || t.type == 'LAMBDA_ARROW'),
          isTrue);

      // 4.0: dynamic (NAME per ECMA-334)
      final t4_0 = tokenizeCSharp('dynamic d = 1;', version: '4.0');
      expect(t4_0.any((t) => t.type == 'NAME' && t.value == 'dynamic'), isTrue);
      final t4_0_async = tokenizeCSharp('int async = 10;', version: '4.0');
      expect(t4_0_async.any((t) => t.type == 'NAME' && t.value == 'async'),
          isTrue);

      // 5.0: async and await keywords
      final t5_0 = tokenizeCSharp('async void Run() { await Task.Delay(1); }',
          version: '5.0');
      expect(
          t5_0.any((t) => t.type == 'KEYWORD' && t.value == 'async'), isTrue);
      expect(
          t5_0.any((t) => t.type == 'KEYWORD' && t.value == 'await'), isTrue);

      // 6.0: null-conditional ?.
      final t6_0 = tokenizeCSharp('var len = str?.Length;', version: '6.0');
      expect(t6_0.any((t) => t.type == 'NULL_CONDITIONAL_DOT'), isTrue);

      // 7.0: binary literal and digit separator
      final t7_0 = tokenizeCSharp('int bin = 0b1010_0101;', version: '7.0');
      expect(t7_0.any((t) => t.type == 'NUMBER'), isTrue);

      // 8.0: range operator ..
      final t8_0 = tokenizeCSharp('var slice = arr[a..b];', version: '8.0');
      expect(t8_0.any((t) => t.type == 'DOT_DOT'), isTrue);

      // 9.0: record and init (NAME contextual keywords)
      final t9_0 =
          tokenizeCSharp('record Point(int X, int Y);', version: '9.0');
      expect(t9_0.any((t) => t.type == 'NAME' && t.value == 'record'), isTrue);

      // 10.0: global keyword
      final t10_0 = tokenizeCSharp('global using System;', version: '10.0');
      expect(t10_0.any((t) => t.value == 'global'), isTrue);

      // 11.0: raw string literals
      final t11_0 = tokenizeCSharp('"""hello world"""', version: '11.0');
      expect(t11_0.any((t) => t.type == 'RAW_STRING'), isTrue);

      // 12.0: default edition
      final t12_0 = tokenizeCSharp('class Bar {}');
      expect(
          t12_0.any((t) => t.type == 'KEYWORD' && t.value == 'class'), isTrue);
      expect(t12_0.any((t) => t.type == 'NAME' && t.value == 'Bar'), isTrue);
    });
  });
}
