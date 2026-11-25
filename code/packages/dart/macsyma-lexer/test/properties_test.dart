/// Property tests for `coding_adventures_macsyma_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// longest-match priority, number shapes, and keyword exactness — all
/// with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macsyma_lexer/macsyma_lexer.dart';
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

/// Markup-dense hostile text: every MACSYMA sigil plus whitespace
/// and a little out-of-alphabet noise.
Gen<String> hostileMacsyma(int minLen, int maxLen) {
  const pieces = [
    ':=',
    '=',
    '**',
    '*',
    '<=',
    '>=',
    '<',
    '>',
    '->',
    '(',
    ')',
    ',',
    ';',
    '\$',
    '+',
    '-',
    '/',
    '^',
    '%',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '.',
    'and',
    'or',
    '@',
    '~',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeMacsyma(source);
  } on LexerError {
    return null;
  }
}

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(3601),
          PbtOptions(numTests: 400, seed: BigInt.from(3601)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
          (input) {
            try {
              tokenizeMacsyma(input);
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
          BigInt.from(3602),
          PbtOptions(numTests: 400, seed: BigInt.from(3602)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
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
          BigInt.from(3603),
          PbtOptions(numTests: 200, seed: BigInt.from(3603)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
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
          BigInt.from(3604),
          PbtOptions(numTests: 400, seed: BigInt.from(3604)),
          (rng) => hostileMacsyma(0, 60).sample(rng),
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

    test('multi-char operators never split', () {
      check(
        'operator-atomic',
        checkProperty<int>(
          BigInt.from(3605),
          PbtOptions(numTests: 100, seed: BigInt.from(3605)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const ops = [
              (':=', 'COLONEQ'),
              ('**', 'STAREQ'),
              ('<=', 'LEQ'),
              ('>=', 'GEQ'),
              ('->', 'ARROW'),
              ('^', 'CARET'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenize(text);
            return tokens != null &&
                tokens.length == 2 && // operator + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('numbers lex whole', () {
      check(
        'number-whole',
        checkProperty<String>(
          BigInt.from(3606),
          PbtOptions(numTests: 200, seed: BigInt.from(3606)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            Gen.tuple2(smallInt(0, 99), smallInt(0, 99))
                .map((t) => '${t.$1}.${t.$2}'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenize(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NUMBER' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('keywords match exactly and lowercase-only', () {
      check(
        'keyword-exact',
        checkProperty<int>(
          BigInt.from(3607),
          PbtOptions(numTests: 100, seed: BigInt.from(3607)),
          (rng) => smallInt(0, 3).sample(rng),
          (index) {
            const keywords = ['and', 'or', 'not', 'false'];
            final kw = keywords[index];
            final hit = tryTokenize(kw);
            final miss =
                tryTokenize('${kw[0].toUpperCase()}${kw.substring(1)}');
            return hit != null &&
                hit.length == 2 &&
                hit[0].type == 'KEYWORD' &&
                miss != null &&
                miss.length == 2 &&
                miss[0].type == 'NAME';
          },
        ),
      );
    });

    test('percent names lex whole', () {
      check(
        'percent-whole',
        checkProperty<String>(
          BigInt.from(3608),
          PbtOptions(numTests: 200, seed: BigInt.from(3608)),
          // Letter-led or empty: `%` needs a name-start after it.
          (rng) => Gen.oneOf([
            Gen.constant(''),
            Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
                .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}'),
          ]).sample(rng),
          (rest) {
            final tokens = tryTokenize('%$rest');
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NAME' &&
                tokens[0].value == '%$rest';
          },
        ),
      );
    });

    test('terminators stay distinct', () {
      check(
        'terminator-distinct',
        checkProperty<String>(
          BigInt.from(3609),
          PbtOptions(numTests: 200, seed: BigInt.from(3609)),
          (rng) => Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
              .sample(rng),
          (name) {
            final semi = tryTokenize('$name;');
            final dollar = tryTokenize('$name\$');
            return semi != null &&
                semi.length == 3 &&
                semi[1].type == 'SEMI' &&
                dollar != null &&
                dollar.length == 3 &&
                dollar[1].type == 'DOLLAR';
          },
        ),
      );
    });
  });
}
