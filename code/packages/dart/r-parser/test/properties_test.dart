/// Property tests for `coding_adventures_r_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–30 range, positions, valid-program parsing, assignment
/// shapes, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_r_parser/r_parser.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// R-dense hostile text: every R sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileR(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '[[',
    ']]',
    '{',
    '}',
    '<-',
    '<<-',
    '->',
    '->>',
    '=',
    '==',
    '...',
    ':',
    '|>',
    '%in%',
    r'$',
    '@',
    '~',
    '!',
    '&',
    '&&',
    '|',
    '||',
    '+',
    '-',
    '*',
    '/',
    '^',
    ',',
    ';',
    '\n',
    '\r',
    ' ',
    '\t',
    'NA',
    'NULL',
    'TRUE',
    'x',
    'f',
    '0',
    '9',
    '.',
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
          BigInt.from(5101),
          PbtOptions(numTests: 400, seed: BigInt.from(5101)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            try {
              parseR(input);
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
          BigInt.from(5102),
          PbtOptions(numTests: 400, seed: BigInt.from(5102)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final ast = tryParseR(input);
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
          BigInt.from(5103),
          PbtOptions(numTests: 200, seed: BigInt.from(5103)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final a = tryParseR(input);
            final b = tryParseR(input);
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
          BigInt.from(5104),
          PbtOptions(numTests: 400, seed: BigInt.from(5104)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final ast = tryParseR(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('depth boundary is exactly fourteen levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5105),
          PbtOptions(numTests: 100, seed: BigInt.from(5105)),
          (rng) => smallInt(0, 30).sample(rng),
          (depth) {
            final src = '${'(' * depth}1${')' * depth}\n';
            return (tryParseR(src) != null) == (depth <= 14);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5106),
          PbtOptions(numTests: 100, seed: BigInt.from(5106)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '1',
              'x <- 1',
              'mean(x)',
              'x[1]',
              'df\$col',
              'x <- 1:3',
              'if (x > 0) 1 else -1',
              'for (i in 1:3) print(i)',
              'x |> f()',
              'function(v) v * v',
            ];
            return tryParseR('${programs[index]}\n') != null;
          },
        ),
      );
    });

    test('each assignment spelling surfaces assignment', () {
      check(
        'assignment-shape',
        checkProperty<int>(
          BigInt.from(5107),
          PbtOptions(numTests: 100, seed: BigInt.from(5107)),
          (rng) => smallInt(0, 4).sample(rng),
          (index) {
            const spells = [
              'x <- 1',
              'x = 1',
              'x <<- 1',
              '1 -> x',
              '1 ->> x',
            ];
            final ast = tryParseR('${spells[index]}\n');
            return ast != null && containsRule(ast, 'assignment');
          },
        ),
      );
    });

    test('typed na atoms parse as atoms', () {
      check(
        'na-shape',
        checkProperty<int>(
          BigInt.from(5108),
          PbtOptions(numTests: 100, seed: BigInt.from(5108)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const atoms = ['NA_integer_', 'NA_real_', 'NA_character_'];
            return tryParseR('${atoms[index]}\n') != null;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5109),
          PbtOptions(numTests: 400, seed: BigInt.from(5109)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final soft = tryParseR(input);
            try {
              final hard = parseR(input);
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
