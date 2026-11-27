/// Property tests for `coding_adventures_maple_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, all six recursion-shape depth
/// boundaries, positions, valid-program parsing, error shapes, and
/// API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_maple_parser/maple_parser.dart';
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

/// Maple-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileMaple(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    ':=',
    '=',
    '<>',
    '<',
    '>',
    '<=',
    '>=',
    '->',
    '+',
    '-',
    '*',
    '/',
    '^',
    ',',
    ';',
    ':',
    'if',
    'then',
    'elif',
    'else',
    'end',
    'fi',
    'and',
    'or',
    'not',
    'AND',
    'true',
    'false',
    'a',
    'x',
    'sin',
    '0',
    '5',
    '"',
    "'",
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

String nestedParenSource(int n) => '${'(' * n}5${')' * n};\n';
String nestedListSource(int n) => '${'[' * n}5${']' * n};\n';
String notChainSource(int n) => '${'not ' * n}a;\n';
String unaryMinusChainSource(int n) => '${'-' * n}a;\n';
String powerChainSource(int n) => '1${'^1' * n};\n';

String nestedIfSource(int n) {
  final buf = StringBuffer();
  for (var i = 0; i < n; i++) {
    buf.write('if 1 then ');
  }
  buf.write('5');
  for (var i = 0; i < n; i++) {
    buf.write(' end if');
  }
  buf.write(';\n');
  return buf.toString();
}

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5471),
          PbtOptions(numTests: 400, seed: BigInt.from(5471)),
          (rng) => hostileMaple(0, 60).sample(rng),
          (input) {
            try {
              parseMaple(input);
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
          BigInt.from(5472),
          PbtOptions(numTests: 400, seed: BigInt.from(5472)),
          (rng) => hostileMaple(0, 60).sample(rng),
          (input) {
            final ast = tryParseMaple(input);
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
          BigInt.from(5473),
          PbtOptions(numTests: 200, seed: BigInt.from(5473)),
          (rng) => hostileMaple(0, 60).sample(rng),
          (input) {
            final a = tryParseMaple(input);
            final b = tryParseMaple(input);
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
          BigInt.from(5474),
          PbtOptions(numTests: 400, seed: BigInt.from(5474)),
          (rng) => hostileMaple(0, 60).sample(rng),
          (input) {
            final ast = tryParseMaple(input);
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
          BigInt.from(5475),
          PbtOptions(numTests: 60, seed: BigInt.from(5475)),
          (rng) => smallInt(0, 5).sample(rng),
          (shape) {
            // Trip points re-measured on this engine via scratch
            // binary-search probes, all identical to the reference:
            // parens 12, list 11, not/unary-minus 136, power/if 68.
            switch (shape) {
              case 0:
                return tryParseMaple(nestedParenSource(11)) != null &&
                    tryParseMaple(nestedParenSource(12)) == null;
              case 1:
                return tryParseMaple(nestedListSource(10)) != null &&
                    tryParseMaple(nestedListSource(11)) == null;
              case 2:
                return tryParseMaple(notChainSource(135)) != null &&
                    tryParseMaple(notChainSource(136)) == null;
              case 3:
                return tryParseMaple(unaryMinusChainSource(135)) != null &&
                    tryParseMaple(unaryMinusChainSource(136)) == null;
              case 4:
                return tryParseMaple(powerChainSource(67)) != null &&
                    tryParseMaple(powerChainSource(68)) == null;
              default:
                return tryParseMaple(nestedIfSource(67)) != null &&
                    tryParseMaple(nestedIfSource(68)) == null;
            }
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5476),
          PbtOptions(numTests: 100, seed: BigInt.from(5476)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '1;\n',
              'x := 5;\n',
              'f := (x, y) -> x + y;\n',
              '[a, b, c];\n',
              '{a, b, c};\n',
              '1 + 2 * 3;\n',
              'a and b or not c;\n',
              'if x > 0 then 1 else -1 fi;\n',
              'if a then if b then c end if else d end if;\n',
              'x := 1; y := 2; x + y;\n',
            ];
            return tryParseMaple(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5477),
          PbtOptions(numTests: 400, seed: BigInt.from(5477)),
          (rng) => hostileMaple(0, 60).sample(rng),
          (input) {
            try {
              parseMaple(input);
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
          BigInt.from(5478),
          PbtOptions(numTests: 400, seed: BigInt.from(5478)),
          (rng) => hostileMaple(0, 60).sample(rng),
          (input) {
            final soft = tryParseMaple(input);
            try {
              final hard = parseMaple(input);
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
