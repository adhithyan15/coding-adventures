/// Property tests for `coding_adventures_axiom_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, all four recursion-shape depth
/// boundaries, positions, valid-program parsing, error shapes, and
/// API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_axiom_parser/axiom_parser.dart';
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

/// Axiom-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileAxiom(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    ':=',
    '==',
    '=',
    '~=',
    '<',
    '>',
    '<=',
    '>=',
    ':',
    '::',
    'has',
    '+',
    '-',
    '*',
    '/',
    '^',
    '**',
    ',',
    ';',
    'if',
    'then',
    'else',
    'and',
    'or',
    'not',
    'true',
    'false',
    'Integer',
    'Float',
    'Ring',
    'f',
    'x',
    'factorial',
    '0',
    '7',
    '1.5',
    '"',
    '--',
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

String nestedParenSource(int n) => '${'(' * n}5${')' * n}';
String nestedCallSource(int n) => '${'f(' * n}5${')' * n}';
String unaryPrefixChainSource(int n) => '${'- ' * n}5';
String powerChainSource(int n) => '1${'^1' * n}';

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5481),
          PbtOptions(numTests: 400, seed: BigInt.from(5481)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            try {
              parseAxiom(input);
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
          BigInt.from(5482),
          PbtOptions(numTests: 400, seed: BigInt.from(5482)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final ast = tryParseAxiom(input);
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
          BigInt.from(5483),
          PbtOptions(numTests: 200, seed: BigInt.from(5483)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final a = tryParseAxiom(input);
            final b = tryParseAxiom(input);
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
          BigInt.from(5484),
          PbtOptions(numTests: 400, seed: BigInt.from(5484)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final ast = tryParseAxiom(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('all four shape boundaries match the reference', () {
      check(
        'depth-boundaries',
        checkProperty<int>(
          BigInt.from(5485),
          PbtOptions(numTests: 60, seed: BigInt.from(5485)),
          (rng) => smallInt(0, 3).sample(rng),
          (shape) {
            // Trip points re-measured on this engine via scratch
            // binary-search probes, all identical to the reference:
            // parens 14, nested calls 14, unary chain 131, power 66.
            switch (shape) {
              case 0:
                return tryParseAxiom(nestedParenSource(13)) != null &&
                    tryParseAxiom(nestedParenSource(14)) == null;
              case 1:
                return tryParseAxiom(nestedCallSource(13)) != null &&
                    tryParseAxiom(nestedCallSource(14)) == null;
              case 2:
                return tryParseAxiom(unaryPrefixChainSource(130)) != null &&
                    tryParseAxiom(unaryPrefixChainSource(131)) == null;
              default:
                return tryParseAxiom(powerChainSource(65)) != null &&
                    tryParseAxiom(powerChainSource(66)) == null;
            }
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5486),
          PbtOptions(numTests: 100, seed: BigInt.from(5486)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '1',
              'factorial 7',
              'f(a, b)',
              '[a, b, c]',
              '2 + 3 * 4 ^ 2',
              'x := 5',
              'f x == x * x',
              'if a > 0 then 1 else -1',
              'Polynomial(Integer) has Ring',
              'a : Fraction(Integer)',
            ];
            return tryParseAxiom(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5487),
          PbtOptions(numTests: 400, seed: BigInt.from(5487)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            try {
              parseAxiom(input);
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
          BigInt.from(5488),
          PbtOptions(numTests: 400, seed: BigInt.from(5488)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final soft = tryParseAxiom(input);
            try {
              final hard = parseAxiom(input);
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
