import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_typescript_parser/coding_adventures_typescript_parser.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int _maxFixtureBytes = 2097152; // 2 MB ceiling

const Map<String, String> _expectedGrammarSha256 = <String, String>{
  'typescript':
      '6e231c7ad4b784a457a85e76473a4b4c082dc50904580d59ec2a1c76fad484ae',
  'ts1.0': 'f3ee66339172492bb07a91804da9757dd0840fe633aeeda1f8972ce46c18c3b5',
  'ts2.0': '6a39273ffbd4e6f8ef0d0eac50ca2abb2d39a9e23ec432a61737ac5cf0d8320f',
  'ts3.0': 'b1da5d0f68b1f6e4b2a288a4ebbb2507b2c841548e4619953308d4ca939ad372',
  'ts4.0': 'ea29f008fa31a15aab39cab615810a8a06d22f489d9bc3f8990e46776f1e17a6',
  'ts5.0': '7aaa68116ada56c036107a64411a22137445c7a721bd1168cda632e3a4ef747e',
  'ts5.8': 'fb9d1d1587be02eece726d5247dcdaefe0a9a023bb53259811e6809cf9d0cfa2',
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
  group('TypeScript Parser Conformance', () {
    test('Canonical cases replay from language-frontends-v1 cases.json', () {
      final casesData = _loadBoundedJson(
        'code/specs/fixtures/language-frontends-v1/cases.json',
      );
      final cases = (casesData['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'typescript')
          .toList();

      expect(
        cases,
        isNotEmpty,
        reason: 'Expected at least one canonical TypeScript case',
      );

      for (final testCase in cases) {
        final caseId = testCase['id'] as String;
        final source = testCase['source'] as String;
        final version = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final parser = createTypescriptParser(source, version: version);
          final ast = parser.parse();
          final projected = _projectAst(ast, parser.tokens);
          final expectedAst = testCase['ast'] as Map<String, dynamic>;

          expect(
            projected,
            equals(expectedAst),
            reason: 'AST mismatch for case $caseId',
          );

          // Verify parseTypescript and parseTypescriptTokens
          final astFromSource = parseTypescript(source, version: version);
          expect(
            _projectAst(astFromSource, parser.tokens),
            equals(expectedAst),
          );

          final astFromTokens = parseTypescriptTokens(
            parser.tokens,
            version: version,
          );
          expect(
            _projectAst(astFromTokens, parser.tokens),
            equals(expectedAst),
          );
        } else {
          expect(
            () => parseTypescript(source, version: version),
            throwsA(isA<TypescriptParseException>()),
            reason: 'Expected syntax error for case $caseId',
          );
        }
      }
    });

    test('Grammar SHA-256 matches manifest entry for all 7 editions', () {
      final grammarsData = _loadBoundedJson(
        'code/specs/fixtures/language-frontends-v1/grammars.json',
      );
      final grammars = (grammarsData['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'typescript' && g['kind'] == 'grammar')
          .toList();

      expect(grammars, hasLength(7));
      for (final entry in grammars) {
        final version = entry['version'] as String;
        final expectedHash = _expectedGrammarSha256[version];
        expect(expectedHash, isNotNull,
            reason: 'Unknown version in manifest: $version');
        expect(entry['sha256'], equals(expectedHash));

        final grammarBytes = _loadBoundedBytes(entry['path'] as String);
        final computedSha256 = sha256.convert(grammarBytes).toString();
        expect(computedSha256, equals(expectedHash));
      }
    });

    test(
        'Pre-compiled grammar rules are loaded and valid across all 7 editions',
        () {
      for (final version in supportedTypescriptParserVersions) {
        final grammar = loadTypescriptParserGrammar(version: version);
        expect(grammar.rules, isNotEmpty);
        expect(grammar.rules.first.name, equals('program'));
      }
    });
  });
}
