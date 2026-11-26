/// Property tests for `coding_adventures_reduce_parser`, using
/// the in-lane `pbt` package: the failure vocabulary over hostile
/// text, the root shape, determinism, all five recursion-shape
/// depth boundaries, positions, valid-program parsing, error shapes,
/// and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_reduce_parser/reduce_parser.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Reduce-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileReduce(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '{',
    '}',
    '[',
    ']',
    '<<',
    '>>',
    ':=',
    '=',
    'neq',
    '<',
    '>',
    '<=',
    '>=',
    '+',
    '-',
    '*',
    '/',
    '^',
    '**',
    '.',
    ',',
    ';',
    '\$',
    'if',
    'then',
    'else',
    'and',
    'or',
    'not',
    'AND',
    'a',
    'x',
    'df',
    '0',
    '5',
    '1.5',
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
String assignChainSource(int n) => '${'a:=' * n}5;\n';
String ifElseChainSource(int n) => '${'if 1 then 1 else ' * n}5;\n';
String consChainSource(int n) => 'a${'.a' * n};\n';
String powerChainSource(int n) => '1${'^1' * n};\n';

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5461),
          PbtOptions(numTests: 400, seed: BigInt.from(5461)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            try {
              parseReduce(input);
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
          BigInt.from(5462),
          PbtOptions(numTests: 400, seed: BigInt.from(5462)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final ast = tryParseReduce(input);
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
          BigInt.from(5463),
          PbtOptions(numTests: 200, seed: BigInt.from(5463)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final a = tryParseReduce(input);
            final b = tryParseReduce(input);
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
          BigInt.from(5464),
          PbtOptions(numTests: 400, seed: BigInt.from(5464)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final ast = tryParseReduce(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('all five shape boundaries match the reference', () {
      check(
        'depth-boundaries',
        checkProperty<int>(
          BigInt.from(5465),
          PbtOptions(numTests: 60, seed: BigInt.from(5465)),
          (rng) => smallInt(0, 4).sample(rng),
          (shape) {
            // Trip points re-measured on this engine via scratch
            // binary-search probes, all identical to the reference:
            // parens 9, assign/ifelse/power 57, cons 113.
            switch (shape) {
              case 0:
                return tryParseReduce(nestedParenSource(8)) != null &&
                    tryParseReduce(nestedParenSource(9)) == null;
              case 1:
                return tryParseReduce(assignChainSource(56)) != null &&
                    tryParseReduce(assignChainSource(57)) == null;
              case 2:
                return tryParseReduce(ifElseChainSource(56)) != null &&
                    tryParseReduce(ifElseChainSource(57)) == null;
              case 3:
                return tryParseReduce(consChainSource(112)) != null &&
                    tryParseReduce(consChainSource(113)) == null;
              default:
                return tryParseReduce(powerChainSource(56)) != null &&
                    tryParseReduce(powerChainSource(57)) == null;
            }
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5466),
          PbtOptions(numTests: 100, seed: BigInt.from(5466)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '1;\n',
              'x := 5;\n',
              'df(x, z);\n',
              '{a, b, c};\n',
              'a . {b, c};\n',
              '2 + 3 * 4 ^ 2;\n',
              'a and b or not c;\n',
              'if a=b then c else d;\n',
              '<< a := 1; a + 1 >>;\n',
              'x := 1; y := 2; x + y;\n',
            ];
            return tryParseReduce(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5467),
          PbtOptions(numTests: 400, seed: BigInt.from(5467)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            try {
              parseReduce(input);
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
          BigInt.from(5468),
          PbtOptions(numTests: 400, seed: BigInt.from(5468)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final soft = tryParseReduce(input);
            try {
              final hard = parseReduce(input);
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
