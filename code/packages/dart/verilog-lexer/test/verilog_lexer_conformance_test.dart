import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:coding_adventures_verilog_lexer/verilog_lexer.dart';
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

  group('Verilog Lexer Conformance: cases.json', () {
    test('replays verilog-baseline token stream', () {
      final cases = (casesDoc['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['id'] == 'verilog-baseline')
          .toList();
      expect(cases, isNotEmpty, reason: 'Missing verilog-baseline test case');

      final testCase = cases.first;
      final source = testCase['source'] as String;
      final version = testCase['version'] as String;
      final expectedTokens = testCase['tokens'] as List<dynamic>;

      final actualTokens = tokenizeVerilog(source, version: version);
      expect(actualTokens.length, equals(expectedTokens.length));

      for (var i = 0; i < actualTokens.length; i++) {
        final expected = expectedTokens[i] as Map<String, dynamic>;
        final actual = actualTokens[i];

        expect(actual.type, equals(expected['type']),
            reason: 'Token $i type mismatch');
        expect(actual.value, equals(expected['value']),
            reason: 'Token $i value mismatch');
        expect(actual.line, equals(expected['line']),
            reason: 'Token $i line mismatch');
        expect(actual.column, equals(expected['column']),
            reason: 'Token $i column mismatch');
      }
    });
  });

  group('Verilog Lexer Conformance: grammars.json', () {
    test('verifies SHA-256 digests of all 4 Verilog token grammars', () {
      final grammarEntries = (grammarsDoc['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'verilog' && g['kind'] == 'tokens')
          .toList();
      expect(grammarEntries.length, equals(4));

      for (final entry in grammarEntries) {
        final path = entry['path'] as String;
        final expectedSha = entry['sha256'] as String;

        // Resolve path relative to root
        final candidatePaths = [
          path,
          '../../../../$path',
          '../../../$path',
          '../../$path',
        ];
        File? file;
        for (final cp in candidatePaths) {
          final f = File(cp);
          if (f.existsSync()) {
            file = f;
            break;
          }
        }
        expect(file, isNotNull, reason: 'Grammar file missing: $path');
        final bytes = file!.readAsBytesSync();
        final actualSha = sha256.convert(bytes).toString();

        expect(actualSha, equals(expectedSha),
            reason: 'SHA-256 mismatch for $path');
      }
    });
  });

  group('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
    test(
        'resiliently handles 10,000 mutated Verilog variants without unhandled crash',
        () {
      final rng = Random(42);
      const baseSource = '''
`define WIDTH 8
`define MAX(a, b) ((a) > (b) ? (a) : (b))
module counter #(parameter N = `WIDTH) (
    input wire clk,
    input wire rst_n,
    output reg [N-1:0] count
);
always @(posedge clk or negedge rst_n) begin
    if (!rst_n)
        count <= 8'h00;
    else
        count <= count + 1'b1;
end
endmodule
''';
      final baseUnits = baseSource.codeUnits;

      for (var iter = 0; iter < 10000; iter++) {
        final mutated = List<int>.from(baseUnits);
        final mutationType = rng.nextInt(5);

        switch (mutationType) {
          case 0:
            // Single character flip
            final pos = rng.nextInt(mutated.length);
            mutated[pos] ^= (1 << rng.nextInt(7));
          case 1:
            // Character insertion
            final pos = rng.nextInt(mutated.length);
            mutated.insert(pos, rng.nextInt(128));
          case 2:
            // Character deletion
            final pos = rng.nextInt(mutated.length);
            mutated.removeAt(pos);
          case 3:
            // Chunk replacement
            final pos = rng.nextInt(mutated.length - 5);
            final len = rng.nextInt(5) + 1;
            for (var k = 0; k < len; k++) {
              mutated[pos + k] = rng.nextInt(128);
            }
          case 4:
            // Truncation
            final truncLen = rng.nextInt(mutated.length);
            mutated.length = truncLen;
        }

        final mutatedText = String.fromCharCodes(mutated);
        try {
          tokenizeVerilog(mutatedText);
        } catch (e) {
          expect(
            e,
            anyOf(
              isA<VerilogLexerException>(),
              isA<LexerError>(),
              isA<FormatException>(),
            ),
          );
        }
      }
    });
  });
}
