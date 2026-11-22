import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_starlark_parser/starlark_parser.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int maxFixtureBytes = 2097152; // 2 MB boundary

File _findFile(String relativePath) {
  final stripped = relativePath.startsWith('code/')
      ? relativePath.substring(5)
      : relativePath;
  final candidates = [
    relativePath,
    '../../../../$relativePath',
    '../../../$relativePath',
    '../../../$stripped',
    '../../$relativePath',
    '../$relativePath',
  ];
  for (final c in candidates) {
    final f = File(c);
    if (f.existsSync()) return f;
  }
  throw StateError('Cannot locate file in candidates: $relativePath');
}

Map<String, dynamic> _readBoundedJson(String path) {
  final file = File(path);
  if (!file.existsSync()) {
    fail('Fixture file does not exist: $path');
  }
  final bytes = file.readAsBytesSync();
  if (bytes.length > maxFixtureBytes) {
    fail('Fixture file exceeds 2 MB boundary: $path (${bytes.length} bytes)');
  }
  return jsonDecode(utf8.decode(bytes)) as Map<String, dynamic>;
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
  late Map<String, dynamic> casesDoc;
  late Map<String, dynamic> grammarsDoc;

  setUpAll(() {
    final casesFile =
        _findFile('code/specs/fixtures/language-frontends-v1/cases.json');
    casesDoc = _readBoundedJson(casesFile.path);

    final grammarsFile =
        _findFile('code/specs/fixtures/language-frontends-v1/grammars.json');
    grammarsDoc = _readBoundedJson(grammarsFile.path);
  });

  group('Starlark Parser Conformance: cases.json', () {
    test('replays starlark-baseline AST projection', () {
      final cases = (casesDoc['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'starlark')
          .toList();

      expect(cases, isNotEmpty, reason: 'Expected Starlark conformance cases');

      for (final testCase in cases) {
        final source = testCase['source'] as String;
        final edition = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final parser = createStarlarkParser(source, version: edition);
          final ast = parser.parse();
          final projected = _projectAst(ast, parser.tokens);
          final expectedAst = testCase['ast'] as Map<String, dynamic>;

          expect(
            projected,
            equals(expectedAst),
            reason: 'AST projection mismatch for case ${testCase['id']}',
          );
        } else {
          expect(
            () => parseStarlark(source, version: edition),
            throwsA(isA<StarlarkParserException>()),
            reason: 'Expected parse error for failure case ${testCase['id']}',
          );
        }
      }
    });
  });

  group('Starlark Grammar Integrity: grammars.json', () {
    test('verifies SHA-256 digest of starlark.grammar', () {
      final grammars = (grammarsDoc['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'starlark' && g['kind'] == 'grammar')
          .toList();

      expect(grammars.length, equals(1),
          reason: 'Expected 1 Starlark grammar specification');

      final entry = grammars.first;
      final expectedSha = entry['sha256'] as String;
      expect(
        expectedSha,
        equals(
            '415005e733a195126779f749defccce57fe6b606fa75dd4e566187ca53345127'),
      );

      final grammarFile = File('../../../grammars/starlark/starlark.grammar');
      if (grammarFile.existsSync()) {
        final bytes = grammarFile.readAsBytesSync();
        final actualSha = sha256.convert(bytes).toString();
        expect(actualSha, equals(expectedSha));
      }
    });
  });
}
