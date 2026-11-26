/// Property tests for `coding_adventures_nib_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// keyword promotion, and operator atomicity — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
import 'package:coding_adventures_nib_lexer/src/_grammar.dart';
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
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789_';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Markup-dense hostile text: every Nib sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileNib(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '{',
    '}',
    ':',
    '=',
    ';',
    ',',
    '+',
    '-',
    '*',
    '/',
    '%',
    '<',
    '>',
    '!',
    '&',
    '|',
    '^',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '_',
    'fn',
    'let',
    '@',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeNib(source);
  } on LexerError {
    return null;
  }
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(3401),
          PbtOptions(numTests: 400, seed: BigInt.from(3401)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            try {
              tokenizeNib(input);
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
          BigInt.from(3402),
          PbtOptions(numTests: 400, seed: BigInt.from(3402)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenize(input);
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
          BigInt.from(3403),
          PbtOptions(numTests: 200, seed: BigInt.from(3403)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final a = tryTokenize(input);
            final b = tryTokenize(input);
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
          BigInt.from(3404),
          PbtOptions(numTests: 400, seed: BigInt.from(3404)),
          (rng) => hostileNib(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenize(input);
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

    test('every keyword promotes to its own value', () {
      check(
        'keyword-promotion',
        checkProperty<int>(
          BigInt.from(3405),
          PbtOptions(numTests: 100, seed: BigInt.from(3405)),
          (rng) => smallInt(0, tokenGrammar.keywords.length - 1).sample(rng),
          (index) {
            final kw = tokenGrammar.keywords[index];
            final tokens = tryTokenize(kw);
            return tokens != null &&
                tokens.length == 2 && // keyword + EOF
                tokens[0].type == kw &&
                tokens[0].value == kw;
          },
        ),
      );
    });

    test('shift operators never split', () {
      check(
        'shift-atomic',
        checkProperty<bool>(
          BigInt.from(3406),
          PbtOptions(numTests: 100, seed: BigInt.from(3406)),
          (rng) => Gen.boolean().sample(rng),
          (left) {
            final tokens = tryTokenize(left ? '<<' : '>>');
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == (left ? 'SHL' : 'SHR');
          },
        ),
      );
    });

    test('integers lex whole', () {
      check(
        'integer-whole',
        checkProperty<String>(
          BigInt.from(3407),
          PbtOptions(numTests: 200, seed: BigInt.from(3407)),
          (rng) => smallInt(0, 99999).map((i) => i.toString()).sample(rng),
          (literal) {
            final tokens = tryTokenize(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'INT_LIT' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('names lex whole', () {
      check(
        'name-whole',
        checkProperty<String>(
          BigInt.from(3408),
          PbtOptions(numTests: 200, seed: BigInt.from(3408)),
          // `q`-led, so no sample can be a keyword.
          (rng) => Gen.tuple2(wordGen(0, 6), wordGen(0, 0))
              .map((t) => 'q${t.$1}')
              .sample(rng),
          (name) {
            final tokens = tryTokenize(name);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NAME' &&
                tokens[0].value == name;
          },
        ),
      );
    });

    test('declarations keep their shape', () {
      check(
        'decl-shape',
        checkProperty<(String, String)>(
          BigInt.from(3409),
          PbtOptions(numTests: 200, seed: BigInt.from(3409)),
          (rng) {
            final name = wordGen(1, 5).sample(rng);
            final value = smallInt(0, 99).sample(rng).toString();
            return (name, value);
          },
          (input) {
            final (name, value) = input;
            final tokens = tryTokenize('let q$name: u4 = $value;');
            if (tokens == null || tokens.length != 8) return false;
            return tokens[0].type == 'let' &&
                tokens[1].type == 'NAME' &&
                tokens[2].type == 'COLON' &&
                tokens[3].type == 'NAME' &&
                tokens[4].type == 'EQ' &&
                tokens[5].type == 'INT_LIT' &&
                tokens[5].value == value &&
                tokens[6].type == 'SEMICOLON';
          },
        ),
      );
    });
  });
}
