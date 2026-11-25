/// Property tests for `coding_adventures_matlab_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–30 range, positions, valid-program parsing, the end-retag
/// rule, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_matlab_parser/matlab_parser.dart';
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

/// MATLAB-dense hostile text: every MATLAB sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileMatlab(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    "'",
    '"',
    '=',
    '==',
    '~=',
    '<',
    '<=',
    '>',
    '>=',
    '&&',
    '||',
    '&',
    '|',
    '~',
    '...',
    '.',
    '.*',
    './',
    '.^',
    ".'",
    '**',
    '+',
    '-',
    '*',
    '/',
    r'\',
    '^',
    ',',
    ';',
    ':',
    '@',
    '%',
    'end',
    'if',
    'for',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '#',
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

bool containsRule(ASTNode node, String name) {
  if (node.ruleName == name) return true;
  return node.children.any((c) => c is ASTNode && containsRule(c, name));
}

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5301),
          PbtOptions(numTests: 400, seed: BigInt.from(5301)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            try {
              parseMatlab(input);
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
          BigInt.from(5302),
          PbtOptions(numTests: 400, seed: BigInt.from(5302)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final ast = tryParseMatlab(input);
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
          BigInt.from(5303),
          PbtOptions(numTests: 200, seed: BigInt.from(5303)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final a = tryParseMatlab(input);
            final b = tryParseMatlab(input);
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
          BigInt.from(5304),
          PbtOptions(numTests: 400, seed: BigInt.from(5304)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final ast = tryParseMatlab(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('depth boundary is exactly twelve levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5305),
          PbtOptions(numTests: 100, seed: BigInt.from(5305)),
          (rng) => smallInt(0, 30).sample(rng),
          (depth) {
            final src = '${'(' * depth}0${')' * depth}\n';
            return (tryParseMatlab(src) != null) == (depth <= 12);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5306),
          PbtOptions(numTests: 100, seed: BigInt.from(5306)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              'y = 2 + 3 * 4',
              'M = [1 2; 3 4]',
              'y = f(x, 2)',
              'last = A(end)',
              'sq = @(x) x .^ 2',
              'a = 1; b = 2',
              'c = A .* B',
              'r = a == b',
              "y = A'",
              'x = 1, y = 2',
            ];
            return tryParseMatlab('${programs[index]}\n') != null;
          },
        ),
      );
    });

    test('bracketed end retags to name', () {
      check(
        'end-retag',
        checkProperty<int>(
          BigInt.from(5307),
          PbtOptions(numTests: 100, seed: BigInt.from(5307)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            // The pre-parse hook rewrites bracketed `end` to NAME, so
            // these parse as index expressions; a depth-0 `end` would
            // be a block terminator instead (unparseable here).
            const srcs = [
              'last = A(end)\n',
              'tail = A(2:end)\n',
              'rev = A(end:-1:1)\n',
            ];
            return tryParseMatlab(srcs[index]) != null;
          },
        ),
      );
    });

    test('matrices surface matrix rules', () {
      check(
        'matrix-shape',
        checkProperty<int>(
          BigInt.from(5308),
          PbtOptions(numTests: 100, seed: BigInt.from(5308)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const srcs = [
              'M = [1 2; 3 4]\n',
              'r = [1, 2, 3]\n',
              'empty = []\n',
            ];
            final ast = tryParseMatlab(srcs[index]);
            return ast != null && containsRule(ast, 'matrix_literal');
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5309),
          PbtOptions(numTests: 400, seed: BigInt.from(5309)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final soft = tryParseMatlab(input);
            try {
              final hard = parseMatlab(input);
              return soft != null && soft == hard;
            } on LexerError {
              return soft == null;
            } on GrammarParseError {
              return soft == null;
            }
          },
        ),
      );
    });
  });
}
