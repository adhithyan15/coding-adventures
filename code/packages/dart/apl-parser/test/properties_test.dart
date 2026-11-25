/// Property tests for `coding_adventures_apl_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, both exact depth boundaries across
/// wide ranges, positions, valid-program parsing, and API agreement —
/// all with fixed seeds.
library;

import 'package:coding_adventures_apl_parser/apl_parser.dart';
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

/// APL-dense hostile text: every APL sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileApl(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '←',
    '+',
    '-',
    '×',
    '÷',
    '⌈',
    '⌊',
    '⍴',
    '⍳',
    ',',
    '/',
    r'\',
    '∘.',
    '=',
    '≠',
    '<',
    '≤',
    '≥',
    '>',
    '⍝',
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
    '@',
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

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5601),
          PbtOptions(numTests: 400, seed: BigInt.from(5601)),
          (rng) => hostileApl(0, 60).sample(rng),
          (input) {
            try {
              parseApl(input);
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
          BigInt.from(5602),
          PbtOptions(numTests: 400, seed: BigInt.from(5602)),
          (rng) => hostileApl(0, 60).sample(rng),
          (input) {
            final ast = tryParseApl(input);
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
          BigInt.from(5603),
          PbtOptions(numTests: 200, seed: BigInt.from(5603)),
          (rng) => hostileApl(0, 60).sample(rng),
          (input) {
            final a = tryParseApl(input);
            final b = tryParseApl(input);
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
          BigInt.from(5604),
          PbtOptions(numTests: 400, seed: BigInt.from(5604)),
          (rng) => hostileApl(0, 60).sample(rng),
          (input) {
            final ast = tryParseApl(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('paren boundary is exactly forty-seven levels', () {
      check(
        'depth-boundary-parens',
        checkProperty<int>(
          BigInt.from(5605),
          PbtOptions(numTests: 100, seed: BigInt.from(5605)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            final src = '${'(' * depth}5${')' * depth}\n';
            return (tryParseApl(src) != null) == (depth <= 47);
          },
        ),
      );
    });

    test('chain boundary is exactly ninety-four terms', () {
      check(
        'depth-boundary-chain',
        checkProperty<int>(
          BigInt.from(5606),
          PbtOptions(numTests: 60, seed: BigInt.from(5606)),
          (rng) => smallInt(0, 120).sample(rng),
          (terms) {
            final src = '1${'+1' * terms}\n';
            return (tryParseApl(src) != null) == (terms <= 94);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5607),
          PbtOptions(numTests: 100, seed: BigInt.from(5607)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '5',
              '1 2 3',
              'A←5',
              '⍴A',
              '2×3+4',
              '+/A',
              '(A+B)×C',
              'A∘.×B',
              'A←1\nB←2\nA+B',
              '⍝ comment',
            ];
            return tryParseApl('${programs[index]}\n') != null;
          },
        ),
      );
    });

    test('multi-line programs keep every line', () {
      check(
        'line-count',
        checkProperty<int>(
          BigInt.from(5608),
          PbtOptions(numTests: 100, seed: BigInt.from(5608)),
          (rng) => smallInt(1, 5).sample(rng),
          (count) {
            final src = '${List.filled(count, 'A←1').join('\n')}\n';
            final ast = tryParseApl(src);
            if (ast == null) return false;
            final lines = [
              for (final c in ast.children)
                if (c is ASTNode) c.ruleName,
            ].where((n) => n == 'line').length;
            return lines == count;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5609),
          PbtOptions(numTests: 400, seed: BigInt.from(5609)),
          (rng) => hostileApl(0, 60).sample(rng),
          (input) {
            final soft = tryParseApl(input);
            try {
              final hard = parseApl(input);
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
