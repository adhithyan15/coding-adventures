import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_starlark_lexer/starlark_lexer.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const int maxFixtureBytes = 2097152; // 2 MB fixture boundary

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

void main() {
  late Map<String, dynamic> casesDoc;
  late Map<String, dynamic> grammarsDoc;

  setUpAll(() {
    const candidatesCases = [
      '../../../specs/fixtures/language-frontends-v1/cases.json',
      '../../specs/fixtures/language-frontends-v1/cases.json',
      'code/specs/fixtures/language-frontends-v1/cases.json',
    ];
    String? foundCasesPath;
    for (final p in candidatesCases) {
      if (File(p).existsSync()) {
        foundCasesPath = p;
        break;
      }
    }
    if (foundCasesPath == null) {
      fail('cases.json fixture not found in candidate locations');
    }
    casesDoc = _readBoundedJson(foundCasesPath);

    const candidatesGrammars = [
      '../../../specs/fixtures/language-frontends-v1/grammars.json',
      '../../specs/fixtures/language-frontends-v1/grammars.json',
      'code/specs/fixtures/language-frontends-v1/grammars.json',
    ];
    String? foundGrammarsPath;
    for (final p in candidatesGrammars) {
      if (File(p).existsSync()) {
        foundGrammarsPath = p;
        break;
      }
    }
    if (foundGrammarsPath == null) {
      fail('grammars.json fixture not found in candidate locations');
    }
    grammarsDoc = _readBoundedJson(foundGrammarsPath);
  });

  group('Shared Conformance: grammars.json digest integrity', () {
    test('verifies SHA-256 digest of starlark.tokens', () {
      final grammarsList = grammarsDoc['grammars'] as List<dynamic>;
      final starlarkTokensSpec = grammarsList.firstWhere(
        (g) => g['family'] == 'starlark' && g['kind'] == 'tokens',
      ) as Map<String, dynamic>;

      final expectedHash = starlarkTokensSpec['sha256'] as String;
      expect(
        expectedHash,
        equals(
            '7b0f726b893d4152bd5b57b492d8bfe2371633bbf52885f5a273fc2d600c89a9'),
      );

      final tokenFile = File('../../../grammars/starlark/starlark.tokens');
      if (tokenFile.existsSync()) {
        final bytes = tokenFile.readAsBytesSync();
        final actualHash = sha256.convert(bytes).toString();
        expect(actualHash, equals(expectedHash));
      }
    });
  });

  group('Shared Conformance: cases.json baseline token stream', () {
    test('starlark-baseline matches exact token sequence', () {
      final casesList = casesDoc['cases'] as List<dynamic>;
      final baseline = casesList.firstWhere(
        (c) => c['id'] == 'starlark-baseline',
      ) as Map<String, dynamic>;

      final source = baseline['source'] as String;
      final expectedTokens = baseline['tokens'] as List<dynamic>;

      final tokens = tokenizeStarlark(source);
      expect(tokens.length, equals(expectedTokens.length));

      for (var i = 0; i < expectedTokens.length; i++) {
        final expected = expectedTokens[i] as Map<String, dynamic>;
        final actual = tokens[i];

        expect(actual.type, equals(expected['type']));
        expect(actual.value, equals(expected['value']));
        expect(actual.line, equals(expected['line']));
        expect(actual.column, equals(expected['column']));
      }
    });
  });
}
