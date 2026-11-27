/// Property tests for `coding_adventures_idl_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, all six recursion-shape depth
/// boundaries, positions, valid-program parsing, error shapes, and
/// API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_idl_parser/idl_parser.dart';
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

/// IDL-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileIdl(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '=',
    '/',
    '/YLOG',
    'EQ',
    'NE',
    'LE',
    'LT',
    'GE',
    'GT',
    'AND',
    'OR',
    'XOR',
    'NOT',
    '+',
    '-',
    '*',
    '#',
    '##',
    '^',
    ',',
    'IF',
    'THEN',
    'ELSE',
    'ENDIF',
    'BEGIN',
    'END',
    'FOR',
    'DO',
    'ENDFOR',
    'WHILE',
    'ENDWHILE',
    'REPEAT',
    'UNTIL',
    'ENDREP',
    'BREAK',
    'CONTINUE',
    'PRO',
    'FUNCTION',
    'RETURN',
    'PRINT',
    'PLOT',
    'STOP',
    'x',
    'a',
    '0',
    '5',
    "'",
    '"',
    '\n',
    '\r',
    ' ',
    '\t',
    '_',
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

String nestedParenSource(int n) => 'y = ${'(' * n}5${')' * n}\n';

String nestedIfSource(int n) {
  final buf = StringBuffer();
  for (var i = 0; i < n; i++) {
    buf.write('IF 1 THEN BEGIN ');
  }
  buf.write('y = 5');
  for (var i = 0; i < n; i++) {
    buf.write(' ENDIF');
  }
  buf.write('\n');
  return buf.toString();
}

String nestedCallSource(int n) => 'y = ${'f(' * n}5${')' * n}\n';
String nestedSubscriptSource(int n) => 'y = ${'a[' * n}5${']' * n}\n';
String unaryPrefixChainSource(int n) => 'y = ${'-' * n}5\n';
String nestedArrayLiteralSource(int n) => 'y = ${'[' * n}5${']' * n}\n';

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5491),
          PbtOptions(numTests: 400, seed: BigInt.from(5491)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            try {
              parseIdl(input);
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
          BigInt.from(5492),
          PbtOptions(numTests: 400, seed: BigInt.from(5492)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final ast = tryParseIdl(input);
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
          BigInt.from(5493),
          PbtOptions(numTests: 200, seed: BigInt.from(5493)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final a = tryParseIdl(input);
            final b = tryParseIdl(input);
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
          BigInt.from(5494),
          PbtOptions(numTests: 400, seed: BigInt.from(5494)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final ast = tryParseIdl(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('all six shape boundaries match the reference', () {
      check(
        'depth-boundaries',
        checkProperty<int>(
          BigInt.from(5495),
          PbtOptions(numTests: 60, seed: BigInt.from(5495)),
          (rng) => smallInt(0, 5).sample(rng),
          (shape) {
            // Trip points re-measured on this engine via scratch
            // binary-search probes, all identical to the reference:
            // parens 14, IF 27, calls 13, subscripts 12, unary 135,
            // arrays 13.
            switch (shape) {
              case 0:
                return tryParseIdl(nestedParenSource(13)) != null &&
                    tryParseIdl(nestedParenSource(14)) == null;
              case 1:
                return tryParseIdl(nestedIfSource(26)) != null &&
                    tryParseIdl(nestedIfSource(27)) == null;
              case 2:
                return tryParseIdl(nestedCallSource(12)) != null &&
                    tryParseIdl(nestedCallSource(13)) == null;
              case 3:
                return tryParseIdl(nestedSubscriptSource(11)) != null &&
                    tryParseIdl(nestedSubscriptSource(12)) == null;
              case 4:
                return tryParseIdl(unaryPrefixChainSource(134)) != null &&
                    tryParseIdl(unaryPrefixChainSource(135)) == null;
              default:
                return tryParseIdl(nestedArrayLiteralSource(12)) != null &&
                    tryParseIdl(nestedArrayLiteralSource(13)) == null;
            }
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5496),
          PbtOptions(numTests: 100, seed: BigInt.from(5496)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              'x = 5\n',
              'PLOT, x, /YLOG\n',
              'x = a/YLOG\n',
              'a[0] = 5\n',
              'a = [1, 2, 3]\n',
              'IF x GT 0 THEN y = 1 ELSE y = 2\n',
              'FOR i = 0, 10 DO y = i\n',
              'WHILE x GT 0 DO x = x - 1\n',
              'PRO simple\n PRINT, 1\nEND\n',
              'y = a EQ b AND c EQ d\n',
            ];
            return tryParseIdl(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5497),
          PbtOptions(numTests: 400, seed: BigInt.from(5497)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            try {
              parseIdl(input);
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
          BigInt.from(5498),
          PbtOptions(numTests: 400, seed: BigInt.from(5498)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final soft = tryParseIdl(input);
            try {
              final hard = parseIdl(input);
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
