/// Property tests for `coding_adventures_mccarthy_lisp_lexer`,
/// using the in-lane `pbt` package: the failure vocabulary over
/// hostile text, EOF shape, determinism, positions, the uppercase
/// rule, valid-program lexing, and factory agreement — all with fixed
/// seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mccarthy_lisp_lexer/mccarthy_lisp_lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// McCarthy-dense hostile text: Lisp 1.0 shapes plus lowercase (which
/// must always fail), operator symbols, strings, and unicode noise.
Gen<String> hostileMcCarthy(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    "'",
    '.',
    'CAR',
    'CDR',
    'CONS',
    'NIL',
    'A',
    'Z',
    'LIST-OF-3',
    '0',
    '42',
    '-42',
    '-',
    'car',
    'x',
    '+',
    '<=',
    'null?',
    '"str"',
    '3.14',
    ';',
    ' ',
    '\n',
    '\t',
    '\r',
    '_',
    '?',
    '!',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

bool tokensEqual(List<Token> a, List<Token> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i].type != b[i].type ||
        a[i].value != b[i].value ||
        a[i].line != b[i].line ||
        a[i].column != b[i].column) {
      return false;
    }
  }
  return true;
}

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(6201),
          PbtOptions(numTests: 400, seed: BigInt.from(6201)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            try {
              tokenizeMcCarthy(input);
            } on LexerError {
              // Expected failure vocabulary.
            }
            return true;
          },
        ),
      );
    });

    test('successful runs end with EOF', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(6202),
          PbtOptions(numTests: 400, seed: BigInt.from(6202)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            try {
              final tokens = tokenizeMcCarthy(input);
              return tokens.isNotEmpty && tokens.last.type == 'EOF';
            } on LexerError {
              return true;
            }
          },
        ),
      );
    });

    test('tokenize is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(6203),
          PbtOptions(numTests: 200, seed: BigInt.from(6203)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            List<Token>? a;
            List<Token>? b;
            try {
              a = tokenizeMcCarthy(input);
            } on LexerError {
              a = null;
            }
            try {
              b = tokenizeMcCarthy(input);
            } on LexerError {
              b = null;
            }
            if (a == null || b == null) return a == null && b == null;
            return tokensEqual(a, b);
          },
        ),
      );
    });

    test('tokens keep sane positions', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(6204),
          PbtOptions(numTests: 400, seed: BigInt.from(6204)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            try {
              for (final t in tokenizeMcCarthy(input)) {
                if (t.line < 1 || t.column < 1) return false;
              }
            } on LexerError {
              // No tokens, no positions to check.
            }
            return true;
          },
        ),
      );
    });

    test('lowercase letters always fail', () {
      check(
        'uppercase-rule',
        checkProperty<String>(
          BigInt.from(6205),
          PbtOptions(numTests: 400, seed: BigInt.from(6205)),
          (rng) => hostileMcCarthy(1, 60).sample(rng),
          (input) {
            // Comments may hold anything; lowercase anywhere else
            // must fail.
            final code = input.replaceAll(RegExp(r';[^\n]*'), '');
            if (!RegExp('[a-z]').hasMatch(code)) return true;
            try {
              tokenizeMcCarthy(input);
            } on LexerError {
              return true;
            }
            return false;
          },
        ),
      );
    });

    test('valid programs always lex', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(6206),
          PbtOptions(numTests: 100, seed: BigInt.from(6206)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const programs = [
              "(CAR '(A B C))",
              '(CONS A (CONS B NIL))',
              '(A . -42)',
              'LIST-OF-3',
              '-42',
              '  CAR ; take the head\n  CDR',
              '',
              '   ; just a comment\n',
              '((A))',
              "'(1 2 3)",
            ];
            try {
              tokenizeMcCarthy(programs[index]);
              return true;
            } on LexerError {
              return false;
            }
          },
        ),
      );
    });

    test('factory agrees with the one-shot', () {
      check(
        'factory-agreement',
        checkProperty<String>(
          BigInt.from(6207),
          PbtOptions(numTests: 200, seed: BigInt.from(6207)),
          (rng) => hostileMcCarthy(0, 60).sample(rng),
          (input) {
            List<Token>? a;
            List<Token>? b;
            try {
              a = createMcCarthyLexer(input).tokenize();
            } on LexerError {
              a = null;
            }
            try {
              b = tokenizeMcCarthy(input);
            } on LexerError {
              b = null;
            }
            if (a == null || b == null) return a == null && b == null;
            return tokensEqual(a, b);
          },
        ),
      );
    });
  });
}
