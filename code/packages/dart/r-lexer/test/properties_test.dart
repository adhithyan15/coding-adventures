/// Property tests for `coding_adventures_r_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// hook's depth rule as a mirror model, number shapes, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_r_lexer/r_lexer.dart';
import 'package:coding_adventures_r_lexer/src/_grammar.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

Gen<String> wordGen(int minLen, int maxLen) {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789_';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Markup-dense hostile text: every R sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileR(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '<-',
    '<<-',
    '->',
    '->>',
    '=',
    '+',
    '-',
    '*',
    '/',
    '^',
    ':',
    '\$',
    '@',
    '|>',
    '\\',
    ',',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '_',
    '.',
    'if',
    'NA',
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
          BigInt.from(3801),
          PbtOptions(numTests: 400, seed: BigInt.from(3801)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            try {
              tokenizeR(input);
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
          BigInt.from(3802),
          PbtOptions(numTests: 400, seed: BigInt.from(3802)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeR(input);
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
          BigInt.from(3803),
          PbtOptions(numTests: 200, seed: BigInt.from(3803)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeR(input);
            final b = tryTokenizeR(input);
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
          BigInt.from(3804),
          PbtOptions(numTests: 400, seed: BigInt.from(3804)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeR(input);
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
          BigInt.from(3805),
          PbtOptions(numTests: 300, seed: BigInt.from(3805)),
          (rng) => Gen.list(1, 20, smallInt(0, 6))
              .map((idx) => idx
                  .map((i) => ['(', ')', '[', ']', '{', '}', '\n'][i])
                  .join())
              .sample(rng),
          (input) {
            // The factory lexer carries the hook; a bare engine lexer
            // over the same grammar shows the pre-hook stream. An
            // INDEPENDENT inline mirror (not the function under test)
            // replays the depth rule on the raw stream and must
            // predict the hooked stream exactly.
            final hooked = tryTokenizeR(input);
            if (hooked == null) return false;
            List<Token> raw;
            try {
              raw = GrammarLexer(input, tokenGrammar).tokenize();
            } on LexerError {
              return false;
            }
            final predicted = <Token>[];
            var depth = 0;
            for (final token in raw) {
              if (!(token.type == 'NEWLINE' && depth > 0)) {
                predicted.add(token);
              }
              if (token.type == 'LPAREN' || token.type == 'LBRACKET') {
                depth += 1;
              } else if (token.type == 'RPAREN' || token.type == 'RBRACKET') {
                depth -= 1;
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

    test('balanced brackets preserve token count shape', () {
      check(
        'bracket-shape',
        checkProperty<String>(
          BigInt.from(3806),
          PbtOptions(numTests: 200, seed: BigInt.from(3806)),
          (rng) => Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
              .sample(rng),
          (name) {
            final tokens = tryTokenizeR('f($name)\n');
            if (tokens == null) return false;
            // f ( name ) NEWLINE EOF
            return tokens.length == 6 &&
                tokens[0].type == 'NAME' &&
                tokens[1].type == 'LPAREN' &&
                tokens[3].type == 'RPAREN' &&
                tokens[4].type == 'NEWLINE';
          },
        ),
      );
    });

    test('numbers lex whole', () {
      check(
        'number-whole',
        checkProperty<String>(
          BigInt.from(3807),
          PbtOptions(numTests: 200, seed: BigInt.from(3807)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            Gen.tuple2(smallInt(0, 99), smallInt(0, 99))
                .map((t) => '${t.$1}.${t.$2}'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenizeR(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NUMBER' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('underscores stay inside names', () {
      check(
        'underscore-name',
        checkProperty<String>(
          BigInt.from(3808),
          PbtOptions(numTests: 200, seed: BigInt.from(3808)),
          (rng) => Gen.tuple3(smallInt(0, 25), wordGen(0, 3), wordGen(1, 3))
              .map(
                  (t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}_${t.$3}')
              .sample(rng),
          (name) {
            final tokens = tryTokenizeR(name);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NAME' &&
                tokens[0].value == name;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(3809),
          PbtOptions(numTests: 400, seed: BigInt.from(3809)),
          (rng) => hostileR(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeR(input);
            try {
              final hard = tokenizeR(input);
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
