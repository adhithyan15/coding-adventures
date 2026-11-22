import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_typescript_lexer/coding_adventures_typescript_lexer.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling

const Map<String, String> _expectedTokensSha256 = <String, String>{
  'typescript':
      '997f1464454d82e31e33eb01de4553fdcf8f953afaffe01ef569751402e9269c',
  'ts1.0': '248a641162daf30f7280d953e0854f3b1df904d61be3d9b4ab8e895f5f74575a',
  'ts2.0': 'bd666f659874c06bc2ad9c87bcdc587b2d2b62206e4556cafa4edb4e3a5adc26',
  'ts3.0': 'c1f5c2dbef83d3e360ab5fd046a78998152825843689119b265b7dabeb2df0ed',
  'ts4.0': 'd1fead964706cb9a4b2800a908831ba61063ef68b3dd176fc5a7b82220b62d9e',
  'ts5.0': '9a8b7c8ccc8f1fd67e2c9e0d63e7f256e12c03235e565242d9d017fa30276d72',
  'ts5.8': 'f8f527a9af04ef04aed1dc943ed8f727a3cd84cf5e4ef07e47c27f7fba833cb0',
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
  group('TypeScript Lexer Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/cases.json');
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'typescript')
          .toList();

      expect(cases, isNotEmpty,
          reason: 'Expected at least one canonical TypeScript case');

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final source = testCase['source'] as String;
        final version = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final tokens = tokenizeTypescript(source, version: version);
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
            () => tokenizeTypescript(source, version: version),
            throwsA(isA<TypescriptLexerException>()),
            reason: 'Expected lexical exception for case $caseId',
          );
        }
      }
    });

    test('Tokens file SHA-256 matches manifest entry for all 7 editions', () {
      final grammarsData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/grammars.json');
      final grammars = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'typescript' && g['kind'] == 'tokens')
          .toList();

      expect(grammars, hasLength(7));
      for (final entry in grammars) {
        final version = entry['version'] as String;
        final expectedHash = _expectedTokensSha256[version];
        expect(expectedHash, isNotNull,
            reason: 'Unknown version in manifest: $version');
        expect(entry['sha256'], equals(expectedHash));

        final tokenBytes = _loadBoundedBytes(entry['path'] as String);
        final computedSha256 = sha256.convert(tokenBytes).toString();
        expect(computedSha256, equals(expectedHash));
      }
    });

    test('Precompiled grammar rules load across all 7 editions', () {
      for (final version in supportedTypescriptVersions) {
        final grammar = loadTypescriptTokenGrammar(version: version);
        expect(grammar.definitions, isNotEmpty);
        expect(grammar.version, equals(1));
      }
    });
  });
}
