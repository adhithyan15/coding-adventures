import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_vhdl_lexer/coding_adventures_vhdl_lexer.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling

/// SHA-256 digests of the embedded editions' `.tokens` files, pinned by the
/// shared `language-frontends-v1` manifest.
const Map<String, String> _expectedTokensSha256 = <String, String>{
  'vhdl1987':
      'fce82870510ca7813fa9db872c5fe035f1096878c4e49ee8ff65fdc868066c5d',
  'vhdl1993':
      '5ea63282bbb9c39fd8b51d5bb555070075f07a96e0abf5d4ab10a8e0eba03b03',
  'vhdl2002':
      'd2dd1e5f898c5d451628094a7cc7323875d5cd5991ccc539d7f52a5f9b348440',
  'vhdl2008':
      'b51084a9ec97a78fc4ab01c5019b5a70fdb8bff061a09272121c8edc2e0a0a81',
  'vhdl2019':
      '306a494deb21d47d68dd2a91eecf455eef1021d8750517881af31a4b1b2cb695',
};

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
  group('VHDL Lexer Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/cases.json');
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'vhdl')
          .toList();

      expect(cases, isNotEmpty,
          reason: 'Expected at least one canonical VHDL case');

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final source = testCase['source'] as String;
        final version = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final tokens = tokenizeVhdl(source, version: version);
          final expectedTokens = (testCase['tokens'] as List<dynamic>)
              .cast<Map<String, dynamic>>();

          expect(tokens.length, equals(expectedTokens.length),
              reason: 'Token count mismatch for case $caseId');

          for (var i = 0; i < tokens.length; i++) {
            final tok = tokens[i];
            final exp = expectedTokens[i];
            expect(tok.type, equals(exp['type']),
                reason: 'Token type mismatch at $i in $caseId');
            expect(tok.value, equals(exp['value']),
                reason: 'Token value mismatch at $i in $caseId');
            expect(tok.line, equals(exp['line']),
                reason: 'Token line mismatch at $i in $caseId');
            expect(tok.column, equals(exp['column']),
                reason: 'Token column mismatch at $i in $caseId');
          }
        } else {
          expect(
            () => tokenizeVhdl(source, version: version),
            throwsA(isA<VhdlLexerException>()),
            reason: 'Expected lexical exception for case $caseId',
          );
        }
      }
    });

    test('Tokens file SHA-256 matches manifest entry for every edition', () {
      final grammarsData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/grammars.json');
      final entries = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'vhdl' && g['kind'] == 'tokens')
          .toList();

      for (final entry in _expectedTokensSha256.entries) {
        final matching =
            entries.where((g) => g['version'] == entry.key).toList();
        expect(matching, hasLength(1), reason: 'Manifest entry ${entry.key}');
        expect(matching.first['sha256'], equals(entry.value));

        final tokenBytes = _loadBoundedBytes(matching.first['path'] as String);
        expect(sha256.convert(tokenBytes).toString(), equals(entry.value));
      }
    });

    test('Pre-compiled grammars are loaded and valid for every edition', () {
      for (final version in supportedVhdlVersions) {
        final grammar = loadVhdlTokenGrammar(version: version);
        expect(grammar.definitions, isNotEmpty);
        expect(grammar.keywords, contains('entity'));
        expect(grammar.caseSensitive, isFalse);
      }
    });
  });
}
