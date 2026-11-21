import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_lattice_lexer/coding_adventures_lattice_lexer.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling
const String _expectedLatticeTokensSha256 =
    'ac85aab853dd4f9bcf76d6bdd019ebc71cd75fbb3bca3bd47e3f5127ec7f8416';

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
  group('Lattice Lexer Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/cases.json');
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'lattice')
          .toList();

      expect(cases, isNotEmpty,
          reason: 'Expected at least one canonical Lattice case');

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final source = testCase['source'] as String;
        final version = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final tokens = tokenizeLattice(source, version: version);
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
            () => tokenizeLattice(source, version: version),
            throwsA(isA<LatticeLexerException>()),
            reason: 'Expected lexical exception for case $caseId',
          );
        }
      }
    });

    test('Tokens file SHA-256 matches manifest entry', () {
      final grammarsData = _loadBoundedJson(
          'code/specs/fixtures/language-frontends-v1/grammars.json');
      final grammars = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'lattice' && g['kind'] == 'tokens')
          .toList();

      expect(grammars, hasLength(1));
      final entry = grammars.first;
      expect(entry['sha256'], equals(_expectedLatticeTokensSha256));

      final tokenBytes = _loadBoundedBytes(entry['path'] as String);
      final computedSha256 = sha256.convert(tokenBytes).toString();
      expect(computedSha256, equals(_expectedLatticeTokensSha256));
    });

    test('Pre-compiled grammar rules are loaded and valid', () {
      final grammar = loadLatticeTokenGrammar();
      expect(grammar.definitions, isNotEmpty);
      expect(grammar.version, equals(1));
    });
  });
}
