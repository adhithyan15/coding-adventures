/// Property tests for `coding_adventures_cobol_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact IF-nesting boundary,
/// positions, valid-program parsing, error shapes, and API agreement
/// — all with fixed seeds.
library;

import 'package:coding_adventures_cobol_parser/cobol_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

String card(String code) => '000000 $code';

String program(List<String> lines) => lines.map(card).join('\n');

/// COBOL-dense hostile text: division headers, verbs, PICTURE
/// fragments, and card-format noise plus out-of-alphabet bytes.
Gen<String> hostileCobol(int minLen, int maxLen) {
  const pieces = [
    'IDENTIFICATION DIVISION.',
    'PROGRAM-ID.',
    'ENVIRONMENT DIVISION.',
    'DATA DIVISION.',
    'PROCEDURE DIVISION.',
    'WORKING-STORAGE SECTION.',
    'PIC',
    'PICTURE',
    'X(20)',
    '9(3)V99',
    'VALUE',
    'MOVE',
    'DISPLAY',
    'PERFORM',
    'COMPUTE',
    'IF',
    'ELSE',
    'STOP RUN.',
    'GO TO',
    '01',
    '77',
    'FILLER',
    '.',
    '(',
    ')',
    ':',
    '=',
    '+',
    '-',
    '*',
    '/',
    '**',
    '"',
    "'",
    '000000 ',
    '\n',
    '\r',
    ' ',
    '\t',
    'A',
    'ZERO',
    '0',
    '9',
    '@',
    '`',
    '\\',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Every token in [node], in tree order.
List<Token> treeTokens(ASTNode node) {
  final out = <Token>[];
  void walk(Object current) {
    if (current is Token) {
      out.add(current);
    } else if (current is ASTNode) {
      for (final child in current.children) {
        walk(child);
      }
    }
  }

  walk(node);
  return out;
}

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5451),
          PbtOptions(numTests: 400, seed: BigInt.from(5451)),
          (rng) => hostileCobol(0, 40).sample(rng),
          (input) {
            try {
              parseCobol(input);
            } on LexerError {
              return true;
            } on GrammarParseError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('successful parses root at program', () {
      check(
        'root-shape',
        checkProperty<String>(
          BigInt.from(5452),
          PbtOptions(numTests: 400, seed: BigInt.from(5452)),
          (rng) => hostileCobol(0, 40).sample(rng),
          (input) {
            final ast = tryParseCobol(input);
            if (ast == null) return true;
            return ast.ruleName == 'program';
          },
        ),
      );
    });

    test('parse is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5453),
          PbtOptions(numTests: 200, seed: BigInt.from(5453)),
          (rng) => hostileCobol(0, 40).sample(rng),
          (input) {
            final a = tryParseCobol(input);
            final b = tryParseCobol(input);
            if (a == null || b == null) return a == null && b == null;
            return a == b;
          },
        ),
      );
    });

    test('tree tokens keep sane positions', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(5454),
          PbtOptions(numTests: 400, seed: BigInt.from(5454)),
          (rng) => hostileCobol(0, 40).sample(rng),
          (input) {
            final ast = tryParseCobol(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('if-nesting boundary is exactly fifty-seven levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5455),
          PbtOptions(numTests: 60, seed: BigInt.from(5455)),
          (rng) => smallInt(0, 80).sample(rng),
          (depth) {
            // Measured on this engine via scratch binary-search
            // probe: 57 full-condition IFs parse, 58 trip the 128
            // cap. (The reference pins no exact boundary — it only
            // asserts 4096 fails — so this guards the Dart engine's
            // own counting as a regression tripwire.)
            final src = program([
              'IDENTIFICATION DIVISION.',
              'PROGRAM-ID. P.',
              'PROCEDURE DIVISION.',
              'MAIN.',
              for (var i = 0; i < depth; i++)
                '    IF GROSS-PAY IS GREATER THAN ZERO',
              '        DISPLAY ZERO.',
              '    STOP RUN.',
            ]);
            return (tryParseCobol(src) != null) == (depth <= 57);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5456),
          PbtOptions(numTests: 100, seed: BigInt.from(5456)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            final programs = [
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. HELLO.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. PAY.',
                'DATA DIVISION.',
                'WORKING-STORAGE SECTION.',
                '01  EMPLOYEE-RECORD.',
                '77  GROSS-PAY  PIC 9(6)V99 VALUE ZERO.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    MOVE ZERO TO GROSS-PAY.',
                '    DISPLAY EMP-NAME GROSS-PAY.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    IF A IF B DISPLAY C.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    COMPUTE RESULT = A + B * C ** D.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'ENVIRONMENT DIVISION.',
                'CONFIGURATION SECTION.',
                'SOURCE-COMPUTER. UNIVAC-II.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    PERFORM SHOW VARYING I FROM 1 BY 1 UNTIL I GREATER 3.',
                '    STOP RUN.',
                'SHOW.',
                '    DISPLAY I.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    DISPLAY WS(2:3).',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    ADD 1 TO R ROUNDED.',
                '    STOP RUN.',
              ]),
              program([
                'IDENTIFICATION DIVISION.',
                'PROGRAM-ID. P.',
                'PROCEDURE DIVISION.',
                'MAIN.',
                '    GO TO MAIN.',
                '    STOP RUN.',
              ]),
            ];
            return tryParseCobol(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5457),
          PbtOptions(numTests: 400, seed: BigInt.from(5457)),
          (rng) => hostileCobol(0, 40).sample(rng),
          (input) {
            try {
              parseCobol(input);
              return true;
            } on LexerError catch (e) {
              return e.errorId.isNotEmpty && e.line >= 1 && e.column >= 1;
            } on GrammarParseError catch (e) {
              if (e.message.isEmpty) return false;
              final token = e.token;
              if (token == null) return true;
              return token.line >= 1 && token.column >= 1;
            }
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5458),
          PbtOptions(numTests: 400, seed: BigInt.from(5458)),
          (rng) => hostileCobol(0, 40).sample(rng),
          (input) {
            final soft = tryParseCobol(input);
            try {
              final hard = parseCobol(input);
              if (soft == null) return false;
              if (soft != hard) return false;
            } on LexerError {
              if (soft != null) return false;
            } on GrammarParseError {
              if (soft != null) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
