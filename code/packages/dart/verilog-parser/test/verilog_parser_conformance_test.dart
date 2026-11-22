import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:coding_adventures_verilog_lexer/verilog_lexer.dart';
import 'package:coding_adventures_verilog_parser/verilog_parser.dart';
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

File _findFile(String relativePath) {
  final candidates = [
    '../../../../$relativePath',
    '../../../$relativePath',
    '../../$relativePath',
    relativePath,
  ];
  for (final c in candidates) {
    final f = File(c);
    if (f.existsSync()) return f;
  }
  throw StateError('Cannot locate file in candidates: $relativePath');
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

  group('Verilog Parser Conformance: cases.json', () {
    test('replays verilog-baseline AST projection', () {
      final cases = (casesDoc['cases'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((c) => c['family'] == 'verilog')
          .toList();

      expect(cases, isNotEmpty, reason: 'Expected Verilog conformance cases');

      for (final testCase in cases) {
        final source = testCase['source'] as String;
        final edition = testCase['version'] as String;
        final outcome = testCase['outcome'] as String;

        if (outcome == 'ok') {
          final parser = createVerilogParser(source, version: edition);
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
            () => parseVerilog(source, version: edition),
            throwsA(isA<VerilogParserException>()),
            reason: 'Expected parse error for failure case ${testCase['id']}',
          );
        }
      }
    });
  });

  group('Verilog Grammar Integrity: grammars.json', () {
    test('verifies SHA-256 digests of all 4 Verilog parser grammars', () {
      final grammars = (grammarsDoc['grammars'] as List<dynamic>)
          .cast<Map<String, dynamic>>()
          .where((g) => g['family'] == 'verilog' && g['kind'] == 'grammar')
          .toList();

      expect(grammars.length, equals(4),
          reason: 'Expected 4 Verilog grammar specifications');

      for (final entry in grammars) {
        final path = entry['path'] as String;
        final expectedSha = entry['sha256'] as String;
        final file = _findFile(path);
        final bytes = file.readAsBytesSync();
        if (bytes.length > maxFixtureBytes) {
          fail('Grammar exceeds fixture byte boundary: $path');
        }
        final actualSha = sha256.convert(bytes).toString();
        expect(
          actualSha,
          equals(expectedSha),
          reason: 'Grammar SHA-256 digest mismatch for $path',
        );
      }
    });
  });

  group('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
    test('resiliently processes 10,000 mutated Verilog variants', () {
      final rng = Random(0x564552494C4F47);
      const seedCode = '''
module counter(input wire clk, input wire rst, output reg [7:0] q);
  always @(posedge clk or posedge rst) begin
    if (rst) begin
      q <= 8'b0;
    end else begin
      q <= q + 1;
    end
  end
endmodule
''';

      const mutationSnippets = [
        'begin',
        'end',
        'module',
        'endmodule',
        'assign',
        'wire',
        'reg',
        ';',
        ',',
        ':',
        '(',
        ')',
        '[',
        ']',
        '{',
        '}',
        '!',
        '~',
        '+',
        '-',
        '&',
        '|',
        '^',
        '`define',
        '`ifdef',
        '`endif',
        '\\escaped_id',
        '/* block comment */',
        '// line comment\n',
        ' ',
        '\t',
        '\n',
        '0',
        '1',
        '8\'hFF',
        '4\'b1010',
      ];

      var handledCount = 0;

      for (var i = 0; i < 10000; i++) {
        final mutType = rng.nextInt(6);
        String mutated;

        switch (mutType) {
          case 0:
            // Spliced insertion
            final pos = rng.nextInt(seedCode.length);
            final snip = mutationSnippets[rng.nextInt(mutationSnippets.length)];
            mutated =
                seedCode.substring(0, pos) + snip + seedCode.substring(pos);
            break;
          case 1:
            // Deletion
            final start = rng.nextInt(seedCode.length);
            final length = rng.nextInt(min(20, seedCode.length - start));
            mutated = seedCode.substring(0, start) +
                seedCode.substring(start + length);
            break;
          case 2:
            // Character replacement
            final chars = seedCode.split('');
            final reps = rng.nextInt(5) + 1;
            for (var r = 0; r < reps; r++) {
              final idx = rng.nextInt(chars.length);
              chars[idx] = String.fromCharCode(rng.nextInt(128));
            }
            mutated = chars.join();
            break;
          case 3:
            // Truncation
            final cut = rng.nextInt(seedCode.length);
            mutated = seedCode.substring(0, cut);
            break;
          case 4:
            // Deep delimiter / block nesting attack
            final depth = rng.nextInt(100);
            mutated =
                'module m; assign y = ${"(" * depth}1${")" * depth}; endmodule';
            break;
          default:
            // Randomized token sequence
            final count = rng.nextInt(15) + 1;
            mutated = List.generate(
              count,
              (_) => mutationSnippets[rng.nextInt(mutationSnippets.length)],
            ).join(' ');
            break;
        }

        try {
          parseVerilog(mutated);
          handledCount++;
        } on VerilogParserException {
          handledCount++;
        } on VerilogLexerException {
          handledCount++;
        } on FormatException {
          handledCount++;
        }
      }

      expect(handledCount, equals(10000));
    });
  });
}
