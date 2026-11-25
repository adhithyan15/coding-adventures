/// Property tests for `coding_adventures_macrooct_lexer`, using
/// the in-lane `pbt` package: the engine contract over hostile text,
/// promotion completeness (no `KEYWORD` survives the one-shots),
/// directive atomicity, agreement with the `oct` lane on shared
/// input, comment invisibility, and API agreement — all with fixed
/// seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macrooct_lexer/macrooct_lexer.dart';
import 'package:coding_adventures_oct_lexer/oct_lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

Gen<String> wordGen(int minLen, int maxLen) {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// MacroOct-dense hostile text: directives, strings, every Oct sigil,
/// comment openers, whitespace, and out-of-alphabet noise.
Gen<String> hostileMacrooct(int minLen, int maxLen) {
  const pieces = [
    '@include',
    '@define',
    '@if',
    '@else',
    '@end',
    '@',
    '"',
    '"s"',
    '==',
    '!=',
    '<=',
    '>=',
    '&&',
    '||',
    '->',
    '<<',
    '>>',
    '+',
    '-',
    '*',
    '/',
    '%',
    '&',
    '|',
    '^',
    '~',
    '!',
    '<',
    '>',
    '=',
    '{',
    '}',
    '(',
    ')',
    ':',
    ';',
    ',',
    '0b',
    '0x',
    'fn',
    'let',
    'if',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '_',
    '\$',
    '#',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Shared-surface soup: no `@`, no quotes — input on which the
/// MacroOct-only rules can never fire, so both lanes must agree
/// exactly (the grammars are otherwise identical, proven by the
/// drift-guard unit test).
Gen<String> sharedSoup(int minLen, int maxLen) {
  const pieces = [
    '==',
    '!=',
    '<=',
    '>=',
    '&&',
    '||',
    '->',
    '<<',
    '>>',
    '+',
    '-',
    '*',
    '/',
    '%',
    '&',
    '|',
    '^',
    '~',
    '!',
    '<',
    '>',
    '=',
    '{',
    '}',
    '(',
    ')',
    ':',
    ';',
    ',',
    '0b101',
    '0x1F',
    'fn',
    'let',
    'static',
    'if',
    'else',
    'while',
    'true',
    '\n',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '_',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(4401),
          PbtOptions(numTests: 400, seed: BigInt.from(4401)),
          (rng) => hostileMacrooct(0, 60).sample(rng),
          (input) {
            try {
              tokenizeMacrooct(input);
            } on LexerError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('successful runs end in exactly one EOF', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(4402),
          PbtOptions(numTests: 400, seed: BigInt.from(4402)),
          (rng) => hostileMacrooct(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMacrooct(input);
            if (tokens == null) return true;
            return tokens.isNotEmpty &&
                tokens.last.type == 'EOF' &&
                countType(tokens, 'EOF') == 1;
          },
        ),
      );
    });

    test('tokenize is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(4403),
          PbtOptions(numTests: 200, seed: BigInt.from(4403)),
          (rng) => hostileMacrooct(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeMacrooct(input);
            final b = tryTokenizeMacrooct(input);
            if (a == null || b == null) return a == null && b == null;
            if (a.length != b.length) return false;
            for (var i = 0; i < a.length; i++) {
              if (a[i] != b[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('positions are sane and monotonic', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(4404),
          PbtOptions(numTests: 400, seed: BigInt.from(4404)),
          (rng) => hostileMacrooct(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMacrooct(input);
            if (tokens == null) return true;
            var line = 1;
            var column = 1;
            for (final t in tokens) {
              if (t.line < 1 || t.column < 1) return false;
              if (t.line < line) return false;
              if (t.line == line && t.column < column) return false;
              line = t.line;
              column = t.column;
            }
            return true;
          },
        ),
      );
    });

    test('no KEYWORD type survives the one-shots', () {
      check(
        'promotion-complete',
        checkProperty<String>(
          BigInt.from(4405),
          PbtOptions(numTests: 400, seed: BigInt.from(4405)),
          (rng) => hostileMacrooct(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMacrooct(input);
            if (tokens == null) return true;
            return countType(tokens, 'KEYWORD') == 0;
          },
        ),
      );
    });

    test('directives and strings never split', () {
      check(
        'directive-atomic',
        checkProperty<int>(
          BigInt.from(4406),
          PbtOptions(numTests: 100, seed: BigInt.from(4406)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const only = [
              ('@include', 'AT_INCLUDE'),
              ('@define', 'AT_DEFINE'),
              ('@if', 'AT_IF'),
              ('@else', 'AT_ELSE'),
              ('@end', 'AT_END'),
              ('"abc"', 'STR_LIT'),
            ];
            final (text, type) = only[index];
            final tokens = tryTokenizeMacrooct(text);
            return tokens != null &&
                tokens.length == 2 && // token + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('oct lane agrees on the shared surface', () {
      check(
        'oct-agreement',
        checkProperty<String>(
          BigInt.from(4407),
          PbtOptions(numTests: 400, seed: BigInt.from(4407)),
          (rng) => sharedSoup(0, 60).sample(rng),
          (input) {
            final actual = tryTokenizeMacrooct(input);
            List<Token> reference;
            try {
              reference = tokenizeOct(input);
            } on LexerError {
              return actual == null;
            }
            if (actual == null) return false;
            if (actual.length != reference.length) return false;
            for (var i = 0; i < actual.length; i++) {
              if (actual[i] != reference[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(4408),
          PbtOptions(numTests: 200, seed: BigInt.from(4408)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenizeMacrooct('1 // $body\n+ 2');
            if (tokens == null) return false;
            final sig = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return sig == 'INT_LIT=1|PLUS=+|INT_LIT=2';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4409),
          PbtOptions(numTests: 400, seed: BigInt.from(4409)),
          (rng) => hostileMacrooct(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeMacrooct(input);
            try {
              final hard = tokenizeMacrooct(input);
              if (soft == null) return false;
              if (soft.length != hard.length) return false;
              for (var i = 0; i < hard.length; i++) {
                if (soft[i] != hard[i]) return false;
              }
              return true;
            } on LexerError {
              return soft == null;
            }
          },
        ),
      );
    });
  });
}
