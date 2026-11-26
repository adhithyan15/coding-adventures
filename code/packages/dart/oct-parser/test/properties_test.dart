/// Property tests for `coding_adventures_oct_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–40 range, positions, valid-program parsing, error shapes,
/// and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_oct_lexer/oct_lexer.dart';
import 'package:coding_adventures_oct_parser/oct_parser.dart';
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

/// Oct-dense hostile text: every grammar sigil and keyword plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileOct(int minLen, int maxLen) {
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
    '->',
    'fn',
    'let',
    'const',
    'static',
    'if',
    'else',
    'while',
    'loop',
    'break',
    'return',
    'true',
    'false',
    'out',
    'u8',
    'main',
    'x',
    'forty_two',
    '0',
    '9',
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
          BigInt.from(5441),
          PbtOptions(numTests: 400, seed: BigInt.from(5441)),
          (rng) => hostileOct(0, 60).sample(rng),
          (input) {
            try {
              parseOct(input);
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
          BigInt.from(5442),
          PbtOptions(numTests: 400, seed: BigInt.from(5442)),
          (rng) => hostileOct(0, 60).sample(rng),
          (input) {
            final ast = tryParseOct(input);
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
          BigInt.from(5443),
          PbtOptions(numTests: 200, seed: BigInt.from(5443)),
          (rng) => hostileOct(0, 60).sample(rng),
          (input) {
            final a = tryParseOct(input);
            final b = tryParseOct(input);
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
          BigInt.from(5444),
          PbtOptions(numTests: 400, seed: BigInt.from(5444)),
          (rng) => hostileOct(0, 60).sample(rng),
          (input) {
            final ast = tryParseOct(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('depth boundary is exactly twenty levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5445),
          PbtOptions(numTests: 100, seed: BigInt.from(5445)),
          (rng) => smallInt(0, 40).sample(rng),
          (depth) {
            // Measured identical in both engines via scratch probes:
            // 20 parses, 21 trips the 200 cap.
            final src =
                'fn main() { let x: u8 = ${'(' * depth}1${')' * depth}; }';
            return (tryParseOct(src) != null) == (depth <= 20);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5446),
          PbtOptions(numTests: 100, seed: BigInt.from(5446)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              '',
              'fn main() { }',
              'fn main() { let x: u8 = 5; }',
              'fn add(a: u8, b: u8) -> u8 { return a + b; }',
              'fn t() { if 1 == 1 { } else { } }',
              'fn t() { while 1 == 1 { } }',
              'fn t() { loop { break; } }',
              'fn t() { out(1, 0); }',
              'static counter: u8 = 0;\nfn main() { }',
              'fn t() { if 1 + 2 == 3 { } }',
            ];
            return tryParseOct(programs[index]) != null;
          },
        ),
      );
    });

    test('failures carry positioned, non-empty errors', () {
      check(
        'error-shape',
        checkProperty<String>(
          BigInt.from(5447),
          PbtOptions(numTests: 400, seed: BigInt.from(5447)),
          (rng) => hostileOct(0, 60).sample(rng),
          (input) {
            try {
              parseOct(input);
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
          BigInt.from(5448),
          PbtOptions(numTests: 400, seed: BigInt.from(5448)),
          (rng) => hostileOct(0, 60).sample(rng),
          (input) {
            final soft = tryParseOct(input);
            try {
              final hard = parseOct(input);
              if (soft == null) return false;
              if (soft != hard) return false;
            } on LexerError {
              if (soft != null) return false;
            } on GrammarParseError {
              if (soft != null) return false;
            }
            // Token factory agrees whenever the source lexes.
            List<Token> tokens;
            try {
              tokens = tokenizeOct(input);
            } on LexerError {
              return soft == null;
            }
            try {
              final viaTokens = createOctParserFromTokens(tokens).parse();
              if (soft == null) return false;
              return soft == viaTokens;
            } on GrammarParseError {
              return soft == null;
            }
          },
        ),
      );
    });
  });
}
