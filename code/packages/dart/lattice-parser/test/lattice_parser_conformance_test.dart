import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_lattice_parser/coding_adventures_lattice_parser.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling
const String _expectedLatticeGrammarSha256 =
    '6ba159ce15dfabd631c062fd144d9b23e3f7cf6dab54f15878785eb8c30ec3ac';

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
  group('Lattice Parser Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
        'code/specs/fixtures/language-frontends-v1/cases.json',
      );
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'lattice')
          .toList();

      expect(
        cases,
        isNotEmpty,
        reason: 'Expected at least one canonical Lattice case',
      );

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final source = testCase['source'] as String;
        final version = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final parser = createLatticeParser(source, version: version);
          final ast = parser.parse();
          final projected = _projectAst(ast, parser.tokens);
          final expectedAst = testCase['ast'] as Map<String, dynamic>;

          expect(
            projected,
            equals(expectedAst),
            reason: 'AST mismatch for case $caseId',
          );

          // Verify parseLattice and parseLatticeTokens
          final astFromSource = parseLattice(source, version: version);
          expect(
            _projectAst(astFromSource, parser.tokens),
            equals(expectedAst),
          );

          final astFromTokens = parseLatticeTokens(
            parser.tokens,
            version: version,
          );
          expect(
            _projectAst(astFromTokens, parser.tokens),
            equals(expectedAst),
          );
        } else {
          expect(
            () => parseLattice(source, version: version),
            throwsA(isA<LatticeParseException>()),
            reason: 'Expected syntax error for case $caseId',
          );
        }
      }
    });

    test('Grammar SHA-256 matches manifest entry', () {
      final grammarsData = _loadBoundedJson(
        'code/specs/fixtures/language-frontends-v1/grammars.json',
      );
      final grammars = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'lattice' && g['kind'] == 'grammar')
          .toList();

      expect(grammars, hasLength(1));
      final entry = grammars.first;
      expect(entry['sha256'], equals(_expectedLatticeGrammarSha256));

      final grammarBytes = _loadBoundedBytes(entry['path'] as String);
      final computedSha256 = sha256.convert(grammarBytes).toString();
      expect(computedSha256, equals(_expectedLatticeGrammarSha256));
    });

    test('Pre-compiled grammar rules are loaded and valid', () {
      final grammar = loadLatticeParserGrammar();
      expect(grammar.rules, isNotEmpty);
      expect(grammar.rules.first.name, equals('stylesheet'));
    });
  });
}
