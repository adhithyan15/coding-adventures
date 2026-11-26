/// Property tests for `coding_adventures_derive_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// bracket-newline hook's depth rule as an independent mirror,
/// operator atomicity, keyword case-sensitivity, and API agreement —
/// all with fixed seeds.
library;

import 'package:coding_adventures_derive_lexer/derive_lexer.dart';
import 'package:coding_adventures_derive_lexer/src/_grammar.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Derive-dense hostile text: every Derive sigil plus whitespace,
/// braces (errors here — Derive has none), and out-of-alphabet noise.
Gen<String> hostileDerive(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
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
    '\n',
    '\r',
    ' ',
    '\t',
    'AND',
    'OR',
    'NOT',
    'and',
    'DIF',
    'SIN',
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

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(4601),
          PbtOptions(numTests: 400, seed: BigInt.from(4601)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            try {
              tokenizeDerive(input);
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
          BigInt.from(4602),
          PbtOptions(numTests: 400, seed: BigInt.from(4602)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeDerive(input);
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
          BigInt.from(4603),
          PbtOptions(numTests: 200, seed: BigInt.from(4603)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeDerive(input);
            final b = tryTokenizeDerive(input);
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
          BigInt.from(4604),
          PbtOptions(numTests: 400, seed: BigInt.from(4604)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeDerive(input);
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

    test('hook matches its depth mirror on bracket soup', () {
      check(
        'hook-mirror',
        checkProperty<String>(
          BigInt.from(4605),
          PbtOptions(numTests: 300, seed: BigInt.from(4605)),
          (rng) => Gen.list(1, 20, smallInt(0, 7))
              .map((idx) => idx
                  .map((i) => ['(', ')', '[', ']', '\n', 'a', '1', ','][i])
                  .join())
              .sample(rng),
          (input) {
            final hooked = tryTokenizeDerive(input);
            if (hooked == null) return false;
            List<Token> raw;
            try {
              raw = GrammarLexer(input, tokenGrammar).tokenize();
            } on LexerError {
              return false;
            }
            // Independent inline mirror: parens AND brackets count
            // toward one combined depth, and an unbalanced closer
            // saturates at zero instead of going negative.
            final predicted = <Token>[];
            var depth = 0;
            for (final token in raw) {
              if (!(token.type == 'NEWLINE' && depth > 0)) {
                predicted.add(token);
              }
              if (token.type == 'LPAREN' || token.type == 'LBRACKET') {
                depth += 1;
              } else if (token.type == 'RPAREN' || token.type == 'RBRACKET') {
                if (depth > 0) depth -= 1;
              }
            }
            if (predicted.length != hooked.length) return false;
            for (var i = 0; i < predicted.length; i++) {
              if (predicted[i] != hooked[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('multi-char operators never split', () {
      check(
        'operator-atomic',
        checkProperty<int>(
          BigInt.from(4606),
          PbtOptions(numTests: 100, seed: BigInt.from(4606)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const ops = [
              (':=', 'ASSIGN'),
              ('<=', 'LE'),
              ('>=', 'GE'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenizeDerive(text);
            return tokens != null &&
                tokens.length == 2 && // operator + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('boolean keywords are exactly the uppercase trio', () {
      check(
        'keyword-case',
        checkProperty<int>(
          BigInt.from(4607),
          PbtOptions(numTests: 100, seed: BigInt.from(4607)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const words = [
              ('AND', 'KEYWORD'),
              ('OR', 'KEYWORD'),
              ('NOT', 'KEYWORD'),
              ('and', 'NAME'),
              ('or', 'NAME'),
              ('not', 'NAME'),
            ];
            final (text, type) = words[index];
            final tokens = tryTokenizeDerive(text);
            return tokens != null &&
                tokens.length == 2 && // word + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('braces never lex: Derive has none', () {
      check(
        'no-braces',
        checkProperty<int>(
          BigInt.from(4608),
          PbtOptions(numTests: 100, seed: BigInt.from(4608)),
          (rng) => smallInt(0, 1).sample(rng),
          (index) {
            final text = ['{', '}'][index];
            return tryTokenizeDerive(text) == null;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4609),
          PbtOptions(numTests: 400, seed: BigInt.from(4609)),
          (rng) => hostileDerive(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeDerive(input);
            try {
              final hard = tokenizeDerive(input);
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
