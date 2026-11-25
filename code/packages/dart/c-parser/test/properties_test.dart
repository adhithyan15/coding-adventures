/// Property tests for `coding_adventures_c_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// the root shape, determinism, the exact depth boundary across the
/// whole 0–40 range, positions, valid-program parsing, the directive
/// strip, the token budget, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_c_lexer/c_lexer.dart';
import 'package:coding_adventures_c_parser/c_parser.dart';
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

/// C-dense hostile text: every C sigil plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileC(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '#',
    '#include',
    'int',
    'void',
    'return',
    'if',
    'else',
    'for',
    'while',
    '=',
    '==',
    '<',
    '<=',
    '>',
    '>=',
    '&&',
    '||',
    '!',
    '+',
    '-',
    '*',
    '/',
    '%',
    '^',
    '&',
    '|',
    ',',
    ';',
    ':',
    '.',
    '"',
    "'",
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '_',
    '@',
    '`',
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
          BigInt.from(5401),
          PbtOptions(numTests: 400, seed: BigInt.from(5401)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            try {
              parseC(input);
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

    test('successful parses root at translation unit', () {
      check(
        'root-shape',
        checkProperty<String>(
          BigInt.from(5402),
          PbtOptions(numTests: 400, seed: BigInt.from(5402)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final ast = tryParseC(input);
            if (ast == null) return true;
            return ast.ruleName == 'translation_unit';
          },
        ),
      );
    });

    test('parse is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5403),
          PbtOptions(numTests: 200, seed: BigInt.from(5403)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final a = tryParseC(input);
            final b = tryParseC(input);
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
          BigInt.from(5404),
          PbtOptions(numTests: 400, seed: BigInt.from(5404)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final ast = tryParseC(input);
            if (ast == null) return true;
            for (final t in treeTokens(ast)) {
              if (t.line < 1 || t.column < 1) return false;
            }
            return true;
          },
        ),
      );
    });

    test('depth boundary is exactly twenty-one levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(5405),
          PbtOptions(numTests: 100, seed: BigInt.from(5405)),
          (rng) => smallInt(0, 40).sample(rng),
          (depth) {
            // Measured identical in both engines via scratch binaries:
            // 21 parses, 22 trips the 128 cap.
            final src =
                'int main(void) { return ${'(' * depth}1${')' * depth}; }';
            return (tryParseC(src) != null) == (depth <= 21);
          },
        ),
      );
    });

    test('valid programs always parse', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5406),
          PbtOptions(numTests: 100, seed: BigInt.from(5406)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              'int main(void) { }',
              'int main(void) { int x = 5; return x; }',
              'int f(int x) { return x; }',
              'int main(void) { int a[3] = {1, 2, 3}; return a[0]; }',
              'void f(void) { for (;;) break; }',
              'int g(void) { if (1) return 1; else return 0; }',
              'int h(void) { while (0) { } return 0; }',
              'double area(double r) { return r; }',
              'unsigned long g(unsigned int a) { return a; }',
              '#include <x>\nint main(void) { return 0; }',
            ];
            return tryParseC(programs[index]) != null;
          },
        ),
      );
    });

    test('directive lines vanish from the token stream', () {
      check(
        'directive-strip',
        checkProperty<String>(
          BigInt.from(5407),
          PbtOptions(numTests: 200, seed: BigInt.from(5407)),
          (rng) => smallInt(0, 2)
              .map((i) =>
                  ['#include <stdint.h>', '#define N 10', '#pragma once'][i])
              .sample(rng),
          (directive) {
            // The stripped stream carries no HASH token and parses
            // exactly like the directive-free source.
            final withDirective = tryTokenizeC('$directive\nint x;\n');
            if (withDirective == null) return false;
            final stripped = stripLegacyDirectiveLines(withDirective);
            if (stripped.any((t) => t.type == 'HASH')) return false;
            final plain = tryTokenizeC('int x;\n');
            if (plain == null) return false;
            final sig = (List<Token> toks) => toks
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return sig(stripped) == sig(plain);
          },
        ),
      );
    });

    test('token budget refuses each dimension', () {
      check(
        'budget-shape',
        checkProperty<int>(
          BigInt.from(5408),
          PbtOptions(numTests: 100, seed: BigInt.from(5408)),
          (rng) => smallInt(0, 3).sample(rng),
          (index) {
            final base = tryTokenizeC('int main(void) { return 1; }')!;
            String? refusal;
            switch (index) {
              case 0:
                refusal = checkTokenBudget([]);
              case 1:
                refusal = checkTokenBudget(base, maxTokens: base.length - 1);
              case 2:
                refusal = checkTokenBudget(base, maxFieldBytes: 5);
              case 3:
                refusal = checkTokenBudget(base, maxTextBytes: 10);
            }
            return refusal != null && tryParseCTokens(base) != null;
          },
        ),
      );
    });

    test('nullable APIs agree with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(5409),
          PbtOptions(numTests: 400, seed: BigInt.from(5409)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final soft = tryParseC(input);
            try {
              final hard = parseC(input);
              if (soft == null) return false;
              if (soft != hard) return false;
            } on LexerError {
              if (soft != null) return false;
            } on GrammarParseError {
              if (soft != null) return false;
            }
            // Token-input twin agrees on directive-free streams.
            final tokens = tryTokenizeC(input);
            if (tokens == null) return soft == null;
            if (tokens.any((t) => t.type == 'HASH')) return true;
            final viaTokens = tryParseCTokens(tokens);
            if (soft == null || viaTokens == null) {
              return soft == null && viaTokens == null;
            }
            return soft == viaTokens;
          },
        ),
      );
    });
  });
}
