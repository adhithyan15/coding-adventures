/// Property tests for `coding_adventures_j_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, all three exact depth boundaries
/// across wide ranges, positions, valid-program parsing, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_j_parser/j_parser.dart';
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

/// J-dense hostile text: every J sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileJ(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '=.`',
    '=.',
    '=:',
    '+',
    '-',
    '*',
    '%',
    '^',
    '<.',
    '>.',
    r'$',
    'i.',
    ',',
    '#',
    '=',
    '~:',
    '<',
    '>',
    '<:',
    '>:',
    '@',
    '/',
    r'\',
    'NB.',
    '\n',
    '\r',
    ' ',
    '\t',
    'A',
    'Z',
    'a',
    '0',
    '9',
    '.',
    '_',
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
          BigInt.from(5701),
          PbtOptions(numTests: 400, seed: BigInt.from(5701)),
          (rng) => hostileJ(0, 60).sample(rng),
          (input) {
            try {
              parseJ(input);
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
          BigInt.from(5702),
          PbtOptions(numTests: 400, seed: BigInt.from(5702)),
          (rng) => hostileJ(0, 60).sample(rng),
          (input) {
            final ast = tryParseJ(input);
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
          BigInt.from(5703),
          PbtOptions(numTests: 200, seed: BigInt.from(5703)),
          (rng) => hostileJ(0, 60).sample(rng),
          (input) {
            final a = tryParseJ(input);
            final b = tryParseJ(input);
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
          BigInt.from(5704),
          PbtOptions(numTests: 400, seed: BigInt.from(5704)),
          (rng) => hostileJ(0, 60).sample(rng),
          (input) {
            final ast = tryParseJ(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('paren boundary is exactly thirty-two levels', () {
      check(
        'depth-boundary-parens',
        checkProperty<int>(
          BigInt.from(5705),
          PbtOptions(numTests: 100, seed: BigInt.from(5705)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            final src = '${'(' * depth}5${')' * depth}\n';
            return (tryParseJ(src) != null) == (depth <= 32);
          },
        ),
      );
    });

    test('chain boundary is exactly sixty-three terms', () {
      check(
        'depth-boundary-chain',
        checkProperty<int>(
          BigInt.from(5706),
          PbtOptions(numTests: 60, seed: BigInt.from(5706)),
          (rng) => smallInt(0, 100).sample(rng),
          (terms) {
            final src = '1${'+1' * terms}\n';
            return (tryParseJ(src) != null) == (terms <= 63);
          },
        ),
      );
    });

    test('train boundary is exactly sixty-one teeth', () {
      check(
        'depth-boundary-train',
        checkProperty<int>(
          BigInt.from(5707),
          PbtOptions(numTests: 60, seed: BigInt.from(5707)),
          // Trains need at least 2 teeth to parse at all.
          (rng) => smallInt(2, 100).sample(rng),
          (teeth) {
            final src = '(${List.filled(teeth, '+').join(' ')})5\n';
            return (tryParseJ(src) != null) == (teeth <= 61);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5708),
          PbtOptions(numTests: 100, seed: BigInt.from(5708)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '5',
              'A=.5',
              'A+B',
              '+/A',
              '(A+B)*C',
              '(+ - *)A',
              'A(+ -)B',
              '+@-A',
              'A=.1\nB=.2\nA+B',
              'NB. just a comment',
            ];
            return tryParseJ('${programs[index]}\n') != null;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5709),
          PbtOptions(numTests: 400, seed: BigInt.from(5709)),
          (rng) => hostileJ(0, 60).sample(rng),
          (input) {
            final soft = tryParseJ(input);
            try {
              final hard = parseJ(input);
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
