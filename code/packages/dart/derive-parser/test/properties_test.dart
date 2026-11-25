/// Property tests for `coding_adventures_derive_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–30 range, positions, valid-expression parsing, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_derive_parser/derive_parser.dart';
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

/// Derive-dense hostile text: every Derive sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileDerive(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    ':=',
    '=',
    '<=',
    '>=',
    '<',
    '>',
    '^',
    '*',
    '/',
    '+',
    '-',
    ',',
    ';',
    ':',
    'AND',
    'OR',
    'NOT',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '.',
    '~',
    '@',
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
          BigInt.from(5001),
          PbtOptions(numTests: 400, seed: BigInt.from(5001)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            try {
              parseDerive(input);
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
          BigInt.from(5002),
          PbtOptions(numTests: 400, seed: BigInt.from(5002)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final ast = tryParseDerive(input);
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
          BigInt.from(5003),
          PbtOptions(numTests: 200, seed: BigInt.from(5003)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final a = tryParseDerive(input);
            final b = tryParseDerive(input);
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
          BigInt.from(5004),
          PbtOptions(numTests: 400, seed: BigInt.from(5004)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final ast = tryParseDerive(input);
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
          BigInt.from(5005),
          PbtOptions(numTests: 100, seed: BigInt.from(5005)),
          (rng) => smallInt(0, 30).sample(rng),
          (depth) {
            final src = '${'(' * depth}1${')' * depth}\n';
            // The measured boundary holds across the whole range, not
            // just at 14/15: the cap counts rule frames identically in
            // both engines.
            return (tryParseDerive(src) != null) == (depth <= 14);
          },
        ),
      );
    });

    test('valid expressions always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5006),
          PbtOptions(numTests: 100, seed: BigInt.from(5006)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const exprs = [
              '1',
              'x',
              'a + b',
              'f(x)',
              '[1, 2]',
              'x := 5',
              'a AND b',
              '-x^2',
              '(1 + 2) * 3',
              'DIF(SIN(x), x)',
            ];
            return tryParseDerive('${exprs[index]}\n') != null;
          },
        ),
      );
    });

    test('assignments surface the assignment rule', () {
      check(
        'assignment-shape',
        checkProperty<int>(
          BigInt.from(5007),
          PbtOptions(numTests: 100, seed: BigInt.from(5007)),
          (rng) => smallInt(0, 1).sample(rng),
          (index) {
            final src = ['x := 5\n', 'F(x) := x^2 + 1\n'][index];
            final ast = tryParseDerive(src);
            return ast != null && containsRule(ast, 'assignment');
          },
        ),
      );
    });

    test('equations carry eq, never assign', () {
      check(
        'equation-shape',
        checkProperty<String>(
          BigInt.from(5008),
          PbtOptions(numTests: 100, seed: BigInt.from(5008)),
          (rng) => smallInt(0, 99).map((i) => 'x = $i\n').sample(rng),
          (src) {
            // Every expression descends through the `assignment` rule
            // structurally, so the equation/definition distinction
            // lives in the operator token: `=` vs `:=`.
            final ast = tryParseDerive(src);
            if (ast == null || !containsRule(ast, 'comparison')) {
              return false;
            }
            final tokens = treeTokens(ast);
            return tokens.any((t) => t.type == 'EQ' && t.value == '=') &&
                !tokens.any((t) => t.type == 'ASSIGN');
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5009),
          PbtOptions(numTests: 400, seed: BigInt.from(5009)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final soft = tryParseDerive(input);
            try {
              final hard = parseDerive(input);
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
