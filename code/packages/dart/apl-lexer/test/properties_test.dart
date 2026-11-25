/// Property tests for `coding_adventures_apl_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// glyph atomicity, number shapes, and comment invisibility — all
/// with fixed seeds.
library;

import 'package:coding_adventures_apl_lexer/apl_lexer.dart';
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

Gen<String> wordGen(int minLen, int maxLen) {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Markup-dense hostile text: every APL sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileApl(int minLen, int maxLen) {
  const pieces = [
    '+',
    '-',
    '×',
    '÷',
    '⌈',
    '⌊',
    '⍴',
    '⍳',
    ',',
    '=',
    '≠',
    '<',
    '≤',
    '≥',
    '>',
    '/',
    '\\',
    '∘.',
    '←',
    '(',
    ')',
    '¯',
    '⍝',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '.',
    '@',
    '_',
    '\$',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeApl(source);
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
          BigInt.from(2801),
          PbtOptions(numTests: 400, seed: BigInt.from(2801)),
          (rng) => hostileApl(0, 60).sample(rng),
          (input) {
            try {
              tokenizeApl(input);
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
          BigInt.from(2802),
          PbtOptions(numTests: 400, seed: BigInt.from(2802)),
          (rng) => hostileApl(0, 60).sample(rng),
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
          BigInt.from(2803),
          PbtOptions(numTests: 200, seed: BigInt.from(2803)),
          (rng) => hostileApl(0, 60).sample(rng),
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
          BigInt.from(2804),
          PbtOptions(numTests: 400, seed: BigInt.from(2804)),
          (rng) => hostileApl(0, 60).sample(rng),
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

    test('every glyph lexes atomic with its value', () {
      check(
        'glyph-atomic',
        checkProperty<int>(
          BigInt.from(2805),
          PbtOptions(numTests: 100, seed: BigInt.from(2805)),
          (rng) => smallInt(0, 21).sample(rng),
          (index) {
            const glyphs = [
              ('+', 'PLUS'),
              ('-', 'MINUS'),
              ('×', 'TIMES'),
              ('÷', 'DIVIDE'),
              ('⌈', 'CEILING'),
              ('⌊', 'FLOOR'),
              ('⍴', 'RHO'),
              ('⍳', 'IOTA'),
              (',', 'RAVEL'),
              ('=', 'EQ'),
              ('≠', 'NE'),
              ('<', 'LT'),
              ('≤', 'LE'),
              ('≥', 'GE'),
              ('>', 'GT'),
              ('/', 'REDUCE'),
              ('\\', 'SCAN'),
              ('∘.', 'OUTER'),
              ('←', 'ARROW'),
              ('(', 'LPAREN'),
              (')', 'RPAREN'),
              ('⍝x\n', 'NEWLINE'),
            ];
            final (text, type) = glyphs[index];
            final tokens = tryTokenize(text);
            return tokens != null &&
                tokens.length == 2 && // token + EOF
                tokens[0].type == type &&
                (type == 'NEWLINE' || tokens[0].value == text);
          },
        ),
      );
    });

    test('numbers lex whole with high-minus forms', () {
      check(
        'number-whole',
        checkProperty<String>(
          BigInt.from(2806),
          PbtOptions(numTests: 200, seed: BigInt.from(2806)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            smallInt(1, 999).map((i) => '¯$i'),
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

    test('names lex whole', () {
      check(
        'name-whole',
        checkProperty<String>(
          BigInt.from(2807),
          PbtOptions(numTests: 200, seed: BigInt.from(2807)),
          (rng) => Gen.tuple2(smallInt(0, 25), wordGen(0, 6))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
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

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(2808),
          PbtOptions(numTests: 200, seed: BigInt.from(2808)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenize('A←1 ⍝$body\nB←2');
            if (tokens == null) return false;
            final kinds = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => t.type)
                .join('|');
            return kinds == 'NAME|ARROW|NUMBER|NEWLINE|NAME|ARROW|NUMBER';
          },
        ),
      );
    });

    test('reduce chains keep operator order', () {
      check(
        'reduce-shape',
        checkProperty<String>(
          BigInt.from(2809),
          PbtOptions(numTests: 200, seed: BigInt.from(2809)),
          (rng) => Gen.tuple2(smallInt(0, 25), smallInt(0, 25))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}+/'
                  '${'abcdefghijklmnopqrstuvwxyz'[t.$2]}')
              .sample(rng),
          (input) {
            final tokens = tryTokenize(input);
            if (tokens == null || tokens.length != 5) return false;
            return tokens[0].type == 'NAME' &&
                tokens[1].type == 'PLUS' &&
                tokens[2].type == 'REDUCE' &&
                tokens[3].type == 'NAME';
          },
        ),
      );
    });
  });
}
