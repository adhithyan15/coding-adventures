import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_vhdl_parser/coding_adventures_vhdl_parser.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling

/// SHA-256 digests of the embedded editions' `.grammar` files, pinned by the
/// shared `language-frontends-v1` manifest.
const Map<String, String> _expectedGrammarSha256 = <String, String>{
  'vhdl1987':
      '8203cc07043591b2783935edfc27744a973596df170193d72359621391bfd0b3',
  'vhdl1993':
      '6ec4b4c749453b8fee9fd19aa57cad3bf42ae4f6e18d0709cae77d166a614959',
  'vhdl2002':
      'e6ae555e4ee8cd508d73df89815d48db55a04e64682622875f396ba9406aa7a7',
  'vhdl2008':
      'b2fcb212b4b4d3f4fbc6056af963239fb042d376c1badc1e407d354e74785c9a',
  'vhdl2019':
      '55a103beca5f92104a0e8f2d94473e5a13a2f90710d9848b02c303b4b3e19c11',
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

Map<String, dynamic> _projectAst(Object node, List<Token> tokens) {
  if (node is Token) {
    final index = tokens.indexWhere((t) => identical(t, node));
    final tokenIndex = index >= 0 ? index : tokens.indexOf(node);
    return <String, dynamic>{'token': tokenIndex};
  }
  if (node is ASTNode) {
    return <String, dynamic>{
      'rule': node.ruleName,
      'children': node.children.map((c) => _projectAst(c, tokens)).toList(),
    };
  }
  throw StateError('Unexpected AST node type: $node');
}

void main() {
  group('VHDL Parser Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
        'code/specs/fixtures/language-frontends-v1/cases.json',
      );
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'vhdl')
          .toList();

      expect(
        cases,
        isNotEmpty,
        reason: 'Expected at least one canonical VHDL case',
      );

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final source = testCase['source'] as String;
        final version = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final parser = createVhdlParser(source, version: version);
          final ast = parser.parse();
          final projected = _projectAst(ast, parser.tokens);
          final expectedAst = testCase['ast'] as Map<String, dynamic>;

          expect(
            projected,
            equals(expectedAst),
            reason: 'AST mismatch for case $caseId',
          );

          // Verify parseVhdl and parseVhdlTokens
          final astFromSource = parseVhdl(source, version: version);
          expect(
            _projectAst(astFromSource, parser.tokens),
            equals(expectedAst),
          );

          final astFromTokens = parseVhdlTokens(
            parser.tokens,
            version: version,
          );
          expect(
            _projectAst(astFromTokens, parser.tokens),
            equals(expectedAst),
          );
        } else {
          expect(
            () => parseVhdl(source, version: version),
            throwsA(isA<VhdlParseException>()),
            reason: 'Expected syntax error for case $caseId',
          );
        }
      }
    });

    test('Grammar SHA-256 matches manifest entry for every edition', () {
      final grammarsData = _loadBoundedJson(
        'code/specs/fixtures/language-frontends-v1/grammars.json',
      );
      final entries = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'vhdl' && g['kind'] == 'grammar')
          .toList();

      for (final entry in _expectedGrammarSha256.entries) {
        final matching =
            entries.where((g) => g['version'] == entry.key).toList();
        expect(matching, hasLength(1), reason: 'Manifest entry ${entry.key}');
        expect(matching.first['sha256'], equals(entry.value));

        final bytes = _loadBoundedBytes(matching.first['path'] as String);
        expect(sha256.convert(bytes).toString(), equals(entry.value));
      }
    });

    test('Pre-compiled grammars are loaded and valid for every edition', () {
      for (final version in supportedVhdlParserVersions) {
        final grammar = loadVhdlParserGrammar(version: version);
        expect(grammar.rules, isNotEmpty);
        expect(grammar.rules.first.name, equals('design_file'));
      }
    });
  });
}
