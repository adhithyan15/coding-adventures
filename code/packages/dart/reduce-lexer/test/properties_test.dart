/// Property tests for `coding_adventures_reduce_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// longest-match priority, number shapes, keyword exactness, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_reduce_lexer/reduce_lexer.dart';
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

/// Markup-dense hostile text: every REDUCE sigil plus whitespace and
/// a little out-of-alphabet noise.
Gen<String> hostileReduce(int minLen, int maxLen) {
  const pieces = [
    ':=',
    '=',
    '==',
    '+',
    '-',
    '*',
    '**',
    '/',
    '^',
    '<',
    '<=',
    '>',
    '>=',
    '<<',
    '>>',
    '(',
    ')',
    '{',
    '}',
    ',',
    '.',
    ';',
    '\$',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    'if',
    'and',
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
          BigInt.from(3201),
          PbtOptions(numTests: 400, seed: BigInt.from(3201)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            try {
              tokenizeReduce(input);
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
          BigInt.from(3202),
          PbtOptions(numTests: 400, seed: BigInt.from(3202)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeReduce(input);
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
          BigInt.from(3203),
          PbtOptions(numTests: 200, seed: BigInt.from(3203)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeReduce(input);
            final b = tryTokenizeReduce(input);
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
          BigInt.from(3204),
          PbtOptions(numTests: 400, seed: BigInt.from(3204)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeReduce(input);
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
          BigInt.from(3205),
          PbtOptions(numTests: 100, seed: BigInt.from(3205)),
          (rng) => smallInt(0, 6).sample(rng),
          (index) {
            // Note: no `==` here — REDUCE has no DEFINE token, so it
            // lexes as EQ EQ (pinned in unit tests).
            const ops = [
              (':=', 'ASSIGN'),
              ('**', 'POW'),
              ('<=', 'LE'),
              ('>=', 'GE'),
              ('<<', 'GROUP_OPEN'),
              ('>>', 'GROUP_CLOSE'),
              ('^', 'CARET'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenizeReduce(text);
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
          BigInt.from(3206),
          PbtOptions(numTests: 200, seed: BigInt.from(3206)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            Gen.tuple2(smallInt(0, 99), smallInt(0, 99))
                .map((t) => '${t.$1}.${t.$2}'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenizeReduce(literal);
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
          BigInt.from(3207),
          PbtOptions(numTests: 100, seed: BigInt.from(3207)),
          (rng) => smallInt(0, 6).sample(rng),
          (index) {
            const keywords = ['and', 'or', 'not', 'neq', 'if', 'then', 'else'];
            final kw = keywords[index];
            final hit = tryTokenizeReduce(kw);
            final miss =
                tryTokenizeReduce('${kw[0].toUpperCase()}${kw.substring(1)}');
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

    test('terminators stay distinct', () {
      check(
        'terminator-distinct',
        checkProperty<String>(
          BigInt.from(3208),
          PbtOptions(numTests: 200, seed: BigInt.from(3208)),
          (rng) => Gen.tuple2(smallInt(0, 25), smallInt(0, 99))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]} := ${t.$2}')
              .sample(rng),
          (stmt) {
            final semi = tryTokenizeReduce('$stmt;');
            final dollar = tryTokenizeReduce('$stmt\$');
            return semi != null &&
                semi[semi.length - 2].type == 'SEMI' &&
                dollar != null &&
                dollar[dollar.length - 2].type == 'DOLLAR';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(3209),
          PbtOptions(numTests: 400, seed: BigInt.from(3209)),
          (rng) => hostileReduce(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeReduce(input);
            try {
              final hard = tokenizeReduce(input);
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
