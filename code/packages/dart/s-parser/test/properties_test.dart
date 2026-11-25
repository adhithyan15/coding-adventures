/// Property tests for `coding_adventures_s_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–30 range, positions, valid-program parsing, assignment
/// spellings, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_s_parser/s_parser.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// S-dense hostile text: every S sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileS(int minLen, int maxLen) {
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
    '_',
    '=',
    '==',
    '...',
    ':',
    '%in%',
    '%%',
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
          BigInt.from(5201),
          PbtOptions(numTests: 400, seed: BigInt.from(5201)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            try {
              parseS(input);
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
          BigInt.from(5202),
          PbtOptions(numTests: 400, seed: BigInt.from(5202)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final ast = tryParseS(input);
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
          BigInt.from(5203),
          PbtOptions(numTests: 200, seed: BigInt.from(5203)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final a = tryParseS(input);
            final b = tryParseS(input);
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
          BigInt.from(5204),
          PbtOptions(numTests: 400, seed: BigInt.from(5204)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final ast = tryParseS(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('depth boundary is exactly fifteen levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5205),
          PbtOptions(numTests: 100, seed: BigInt.from(5205)),
          (rng) => smallInt(0, 30).sample(rng),
          (depth) {
            final src = '${'(' * depth}1${')' * depth}\n';
            return (tryParseS(src) != null) == (depth <= 15);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5206),
          PbtOptions(numTests: 100, seed: BigInt.from(5206)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '1',
              'x <- 1',
              'x _ 1',
              'mean(x)',
              'x[1]',
              'df\$x',
              'x <- 1:3',
              'if (x > 0) 1 else -1',
              'for (i in 1:3) print(i)',
              'function(v) v * v',
            ];
            return tryParseS('${programs[index]}\n') != null;
          },
        ),
      );
    });

    test('each assignment spelling surfaces assignment', () {
      check(
        'assignment-shape',
        checkProperty<int>(
          BigInt.from(5207),
          PbtOptions(numTests: 100, seed: BigInt.from(5207)),
          (rng) => smallInt(0, 4).sample(rng),
          (index) {
            const spells = [
              'x <- 1',
              'x _ 1',
              '1 -> x',
              'x <<- 1',
              'a <- b <- 3',
            ];
            final ast = tryParseS('${spells[index]}\n');
            return ast != null && containsRule(ast, 'assignment');
          },
        ),
      );
    });

    test('calls surface call suffixes', () {
      check(
        'call-shape',
        checkProperty<int>(
          BigInt.from(5208),
          PbtOptions(numTests: 100, seed: BigInt.from(5208)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const calls = [
              ('c(1, 2, 3)', 'call_suffix'),
              ('x[1]', 'index_suffix'),
              ('df\$x', 'dollar_suffix'),
            ];
            final (src, rule) = calls[index];
            final ast = tryParseS('$src\n');
            return ast != null && containsRule(ast, rule);
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5209),
          PbtOptions(numTests: 400, seed: BigInt.from(5209)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final soft = tryParseS(input);
            try {
              final hard = parseS(input);
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
