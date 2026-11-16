import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_java_lexer/java_lexer.dart';
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
  group('Java Lexer Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/cases.json');
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'java')
          .toList();

      expect(cases, isNotEmpty,
          reason: 'Expected at least one canonical java case');

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final version = testCase['version'] as String;
        final source = testCase['source'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final tokens = tokenizeJava(source, version: version);
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
            () => tokenizeJava(source, version: version),
            throwsA(isA<JavaLexerException>()),
            reason: 'Expected lexical error in case $caseId',
          );
        } else {
          fail('Unsupported outcome: $outcome in case $caseId');
        }
      }
    });

    test('All 10 Java .tokens files match SHA-256 digests in grammars.json',
        () {
      final grammarsData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/grammars.json');
      final grammars = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'java' && g['kind'] == 'tokens')
          .toList();

      expect(grammars.length, equals(10));

      for (final entry in grammars) {
        final relPath = entry['path'] as String;
        final expectedSha = entry['sha256'] as String;
        final bytes = _loadBoundedBytes(relPath);
        final actualSha = sha256.convert(bytes).toString();
        expect(actualSha, equals(expectedSha),
            reason: 'SHA-256 mismatch for $relPath');
      }
    });

    test('Edition-specific keywords across all 10 Java editions', () {
      // 1.0: base keywords
      final t1_0 = tokenizeJava('int x = 1;', version: '1.0');
      expect(t1_0.any((t) => t.type == 'KEYWORD' && t.value == 'int'), isTrue);

      // 1.1: inner classes / transient
      final t1_1 = tokenizeJava('transient int y = 2;', version: '1.1');
      expect(t1_1.any((t) => t.type == 'KEYWORD' && t.value == 'transient'),
          isTrue);

      // 1.4: assert is keyword in 1.4, identifier in 1.1
      final t1_4 = tokenizeJava('assert condition;', version: '1.4');
      expect(
          t1_4.any((t) => t.type == 'KEYWORD' && t.value == 'assert'), isTrue);
      final t1_1_assert = tokenizeJava('int assert = 5;', version: '1.1');
      expect(t1_1_assert.any((t) => t.type == 'NAME' && t.value == 'assert'),
          isTrue);

      // 5: enum and annotations (@)
      final t5 = tokenizeJava('@Override enum Color { RED }', version: '5');
      expect(t5.any((t) => t.type == 'KEYWORD' && t.value == 'enum'), isTrue);
      expect(t5.any((t) => t.type == 'AT'), isTrue);
      final t1_4_enum = tokenizeJava('int enum = 5;', version: '1.4');
      expect(
          t1_4_enum.any((t) => t.type == 'NAME' && t.value == 'enum'), isTrue);

      // 7: binary literals and underscores
      final t7 = tokenizeJava('int b = 0b1010_0101;', version: '7');
      expect(t7.any((t) => t.type == 'NUMBER'), isTrue);

      // 8: lambdas (->) and double-colon (::)
      final t8 = tokenizeJava('x -> x::toString', version: '8');
      expect(t8.any((t) => t.type == 'ARROW'), isTrue);
      expect(t8.any((t) => t.type == 'DOUBLE_COLON'), isTrue);

      // 10: var is restricted identifier (lexed as NAME per JLS)
      final t10 = tokenizeJava('var x = "test";', version: '10');
      expect(t10.any((t) => t.type == 'NAME' && t.value == 'var'), isTrue);

      // 14: yield in switch expressions
      final t14 = tokenizeJava('yield 42;', version: '14');
      expect(t14.any((t) => t.type == 'KEYWORD' && t.value == 'yield'), isTrue);
      final t10_yield = tokenizeJava('int yield = 10;', version: '10');
      expect(
          t10_yield.any((t) => t.type == 'NAME' && t.value == 'yield'), isTrue);

      // 17: sealed and permits (lexed as NAME per JLS)
      final t17 = tokenizeJava('sealed class A permits B {}', version: '17');
      expect(t17.any((t) => t.type == 'NAME' && t.value == 'sealed'), isTrue);
      expect(t17.any((t) => t.type == 'NAME' && t.value == 'permits'), isTrue);

      // 21: default edition features
      final t21 = tokenizeJava('record Point(int x, int y) {}');
      expect(t21.any((t) => t.type == 'NAME' && t.value == 'record'), isTrue);
      expect(
          t21.any((t) => t.type == 'KEYWORD' && t.value == 'class'), isFalse);
    });
  });
}
