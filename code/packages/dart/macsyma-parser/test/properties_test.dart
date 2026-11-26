/// Property tests for `coding_adventures_macsyma_parser`, using
/// the in-lane `pbt` package: the failure vocabulary over hostile
/// text, the root shape, determinism, the exact depth boundary
/// across the whole 0–40 range, positions, valid-program parsing,
/// error shapes, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macsyma_parser/macsyma_parser.dart';
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

/// MACSYMA-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileMacsyma(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '+',
    '-',
    '*',
    '/',
    '^',
    '**',
    ':',
    ':=',
    '=',
    '#',
    '<',
    '>',
    '<=',
    '>=',
    ',',
    ';',
    '\$',
    '.',
    '"',
    "'",
    'if',
    'then',
    'elseif',
    'else',
    'for',
    'in',
    'thru',
    'step',
    'while',
    'unless',
    'do',
    'block',
    'return',
    'and',
    'or',
    'not',
    'true',
    'false',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    'x',
    '0',
    '9',
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

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5421),
          PbtOptions(numTests: 400, seed: BigInt.from(5421)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            try {
              parseMacsyma(input);
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
          BigInt.from(5422),
          PbtOptions(numTests: 400, seed: BigInt.from(5422)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            final ast = tryParseMacsyma(input);
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
          BigInt.from(5423),
          PbtOptions(numTests: 200, seed: BigInt.from(5423)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            final a = tryParseMacsyma(input);
            final b = tryParseMacsyma(input);
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
          BigInt.from(5424),
          PbtOptions(numTests: 400, seed: BigInt.from(5424)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            final ast = tryParseMacsyma(input);
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
          BigInt.from(5425),
          PbtOptions(numTests: 100, seed: BigInt.from(5425)),
          (rng) => smallInt(0, 40).sample(rng),
          (depth) {
            // Measured identical in both engines via scratch probes:
            // 14 parses, 15 trips the 200 cap.
            final src = '${'(' * depth}0${')' * depth}\$\n';
            return (tryParseMacsyma(src) != null) == (depth <= 14);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5426),
          PbtOptions(numTests: 100, seed: BigInt.from(5426)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '1+2;',
              'x : 3\$',
              'if a then b else c;',
              'for i in [1] do i;',
              'while a do b;',
              'block([x], x+1)\$',
              'f(x, y);',
              'a^b^c;',
              'not a and b or c;',
              'for i: 1 thru 10 do i;',
            ];
            return tryParseMacsyma(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5427),
          PbtOptions(numTests: 400, seed: BigInt.from(5427)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            try {
              parseMacsyma(input);
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
          BigInt.from(5428),
          PbtOptions(numTests: 400, seed: BigInt.from(5428)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            final soft = tryParseMacsyma(input);
            try {
              final hard = parseMacsyma(input);
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
