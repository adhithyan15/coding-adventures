/// Property tests for `coding_adventures_nib_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–40 range, positions, valid-program parsing, error shapes,
/// and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
import 'package:coding_adventures_nib_parser/nib_parser.dart';
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

/// Nib-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileNib(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '{',
    '}',
    '[',
    ']',
    '+',
    '-',
    '*',
    '/',
    '%',
    '+%',
    '<<',
    '>>',
    ':',
    ';',
    ',',
    '=',
    '==',
    '<',
    '>',
    '<=',
    '>=',
    '!',
    '~',
    '&',
    '|',
    '^',
    '&&',
    '||',
    'fn',
    'let',
    'const',
    'static',
    'if',
    'else',
    'while',
    'for',
    'return',
    'true',
    'false',
    'u4',
    'u8',
    'main',
    'x',
    'foo',
    '0',
    '9',
    '0xFF',
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

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(5431),
          PbtOptions(numTests: 400, seed: BigInt.from(5431)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            try {
              parseNib(input);
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
          BigInt.from(5432),
          PbtOptions(numTests: 400, seed: BigInt.from(5432)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final ast = tryParseNib(input);
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
          BigInt.from(5433),
          PbtOptions(numTests: 200, seed: BigInt.from(5433)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final a = tryParseNib(input);
            final b = tryParseNib(input);
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
          BigInt.from(5434),
          PbtOptions(numTests: 400, seed: BigInt.from(5434)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final ast = tryParseNib(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('depth boundary is exactly sixteen levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5435),
          PbtOptions(numTests: 100, seed: BigInt.from(5435)),
          (rng) => smallInt(0, 40).sample(rng),
          (depth) {
            // Measured identical in both engines via scratch probes:
            // 16 parses, 17 trips the 200 cap.
            final src =
                'fn main() { let x: u4 = ${'(' * depth}1${')' * depth}; }';
            return (tryParseNib(src) != null) == (depth <= 16);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5436),
          PbtOptions(numTests: 100, seed: BigInt.from(5436)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '',
              'fn main() { }',
              'fn main() { let x: u4 = 1; }',
              'fn main() { let x: u4 = 1 +% 2; }',
              'fn main() { let x: u8 = 1 << 2 >> 1; }',
              'fn main() { let x: u4 = (1 + 2) * 3; }',
              'fn add(a: u4, b: u4) { return a + b; }',
              'fn main() { if x { return 1; } else { return 0; } }',
              'fn main() { while x { x = x + 1; } }',
              'fn main() { for i: u4 in 0..8 { f(i); } }',
            ];
            return tryParseNib(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5437),
          PbtOptions(numTests: 400, seed: BigInt.from(5437)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            try {
              parseNib(input);
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

    test('nullable APIs agree with the throwing APIs', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5438),
          PbtOptions(numTests: 400, seed: BigInt.from(5438)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final soft = tryParseNib(input);
            try {
              final hard = parseNib(input);
              if (soft == null) return false;
              if (soft != hard) return false;
            } on LexerError {
              if (soft != null) return false;
            } on GrammarParseError {
              if (soft != null) return false;
            }
            // Token-input twins agree whenever the source lexes.
            List<Token> tokens;
            try {
              tokens = tokenizeNib(input);
            } on LexerError {
              return soft == null;
            }
            final softTokens = tryParseNibTokens(tokens);
            try {
              final hardTokens = parseNibTokens(tokens);
              if (softTokens == null) return false;
              if (softTokens != hardTokens) return false;
              if (soft == null || soft != hardTokens) return false;
            } on GrammarParseError {
              if (softTokens != null) return false;
              if (soft != null) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
