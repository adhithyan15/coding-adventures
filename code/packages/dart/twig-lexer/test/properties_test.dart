/// Property tests for `coding_adventures_twig_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// keyword promotion, integer shapes, and comment invisibility — all
/// with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_twig_lexer/twig_lexer.dart';
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

/// Markup-dense hostile text: every Twig sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileTwig(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    "'",
    '#',
    '+',
    '-',
    '*',
    '/',
    '=',
    '<',
    '>',
    '?',
    ':',
    '->',
    ';',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    'define',
    'if',
    '@',
    '~',
    '€',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeTwig(source);
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
          BigInt.from(3101),
          PbtOptions(numTests: 400, seed: BigInt.from(3101)),
          (rng) => hostileTwig(0, 60).sample(rng),
          (input) {
            try {
              tokenizeTwig(input);
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
          BigInt.from(3102),
          PbtOptions(numTests: 400, seed: BigInt.from(3102)),
          (rng) => hostileTwig(0, 60).sample(rng),
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
          BigInt.from(3103),
          PbtOptions(numTests: 200, seed: BigInt.from(3103)),
          (rng) => hostileTwig(0, 60).sample(rng),
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
          BigInt.from(3104),
          PbtOptions(numTests: 400, seed: BigInt.from(3104)),
          (rng) => hostileTwig(0, 60).sample(rng),
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

    test('keywords promote, near-misses stay names', () {
      check(
        'keyword-promotion',
        checkProperty<int>(
          BigInt.from(3105),
          PbtOptions(numTests: 100, seed: BigInt.from(3105)),
          (rng) => smallInt(0, 6).sample(rng),
          (index) {
            const keywords = [
              'define',
              'lambda',
              'let',
              'if',
              'begin',
              'quote',
              'nil'
            ];
            final kw = keywords[index];
            final hit = tryTokenize(kw);
            final miss = tryTokenize('${kw}x');
            return hit != null &&
                hit.length == 2 &&
                hit[0].type == 'KEYWORD' &&
                hit[0].value == kw &&
                miss != null &&
                miss.length == 2 &&
                miss[0].type == 'NAME';
          },
        ),
      );
    });

    test('integers lex whole with sign attached', () {
      check(
        'integer-whole',
        checkProperty<String>(
          BigInt.from(3106),
          PbtOptions(numTests: 200, seed: BigInt.from(3106)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            smallInt(1, 999).map((i) => '-$i'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenize(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'INTEGER' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('booleans are atomic and distinct', () {
      check(
        'boolean-atomic',
        checkProperty<bool>(
          BigInt.from(3107),
          PbtOptions(numTests: 100, seed: BigInt.from(3107)),
          (rng) => Gen.boolean().sample(rng),
          (value) {
            final tokens = tryTokenize(value ? '#t' : '#f');
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == (value ? 'BOOL_TRUE' : 'BOOL_FALSE');
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(3108),
          PbtOptions(numTests: 200, seed: BigInt.from(3108)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenize('42 ;$body\n43');
            if (tokens == null) return false;
            final kinds = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return kinds == 'INTEGER=42|INTEGER=43';
          },
        ),
      );
    });

    test('balanced forms keep their brackets', () {
      check(
        'form-shape',
        checkProperty<String>(
          BigInt.from(3109),
          PbtOptions(numTests: 200, seed: BigInt.from(3109)),
          // `q`-prefixed so the operator can never be a keyword.
          (rng) => Gen.tuple2(wordGen(0, 4), smallInt(0, 99))
              .map((t) => '(q${t.$1} ${t.$2})')
              .sample(rng),
          (input) {
            final tokens = tryTokenize(input);
            if (tokens == null || tokens.length != 5) return false;
            return tokens[0].type == 'LPAREN' &&
                tokens[1].type == 'NAME' &&
                tokens[2].type == 'INTEGER' &&
                tokens[3].type == 'RPAREN';
          },
        ),
      );
    });
  });
}
