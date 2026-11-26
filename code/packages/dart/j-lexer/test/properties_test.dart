/// Property tests for `coding_adventures_j_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// digraph priority, number shapes, and comment invisibility — all
/// with fixed seeds.
library;

import 'package:coding_adventures_j_lexer/j_lexer.dart';
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

/// Markup-dense hostile text: every J sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileJ(int minLen, int maxLen) {
  const pieces = [
    '+',
    '-',
    '*',
    '%',
    '^',
    '<',
    '>',
    '~',
    '=',
    ':',
    '.',
    '\$.',
    '\$',
    'i.',
    ',',
    '#',
    '/',
    '\\',
    '@',
    '(',
    ')',
    '_',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    'NB.',
    '`',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeJ(source);
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
          BigInt.from(2901),
          PbtOptions(numTests: 400, seed: BigInt.from(2901)),
          (rng) => hostileJ(0, 60).sample(rng),
          (input) {
            try {
              tokenizeJ(input);
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
          BigInt.from(2902),
          PbtOptions(numTests: 400, seed: BigInt.from(2902)),
          (rng) => hostileJ(0, 60).sample(rng),
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
          BigInt.from(2903),
          PbtOptions(numTests: 200, seed: BigInt.from(2903)),
          (rng) => hostileJ(0, 60).sample(rng),
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
          BigInt.from(2904),
          PbtOptions(numTests: 400, seed: BigInt.from(2904)),
          (rng) => hostileJ(0, 60).sample(rng),
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

    test('digraphs never split', () {
      check(
        'digraph-atomic',
        checkProperty<int>(
          BigInt.from(2905),
          PbtOptions(numTests: 100, seed: BigInt.from(2905)),
          (rng) => smallInt(0, 8).sample(rng),
          (index) {
            const digraphs = [
              ('<.', 'FLOOR'),
              ('>.', 'CEILING'),
              ('<:', 'LE'),
              ('>:', 'GE'),
              ('~:', 'NE'),
              ('=.', 'ASSIGN_LOCAL'),
              ('=:', 'ASSIGN_GLOBAL'),
              ('i.', 'IDOT'),
              ('+/', 'PLUS_PLUS'),
            ];
            final (text, _) = digraphs[index];
            final tokens = tryTokenize(text);
            if (index == 8) {
              // `+/` is two verbs, not a digraph — the control case.
              return tokens != null &&
                  tokens.length == 3 &&
                  tokens[0].type == 'PLUS' &&
                  tokens[1].type == 'REDUCE';
            }
            return tokens != null &&
                tokens.length == 2 && // digraph + EOF
                tokens[0].type == digraphs[index].$2 &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('numbers lex whole with underscore forms', () {
      check(
        'number-whole',
        checkProperty<String>(
          BigInt.from(2906),
          PbtOptions(numTests: 200, seed: BigInt.from(2906)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            smallInt(1, 999).map((i) => '_$i'),
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
          BigInt.from(2907),
          PbtOptions(numTests: 200, seed: BigInt.from(2907)),
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
          BigInt.from(2908),
          PbtOptions(numTests: 200, seed: BigInt.from(2908)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenize('A=.1 NB.$body\nB=.2');
            if (tokens == null) return false;
            final kinds = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => t.type)
                .join('|');
            return kinds ==
                'NAME|ASSIGN_LOCAL|NUMBER|NEWLINE|NAME|ASSIGN_LOCAL|NUMBER';
          },
        ),
      );
    });

    test('assignments keep target, operator, and value', () {
      check(
        'assign-shape',
        checkProperty<(String, String)>(
          BigInt.from(2909),
          PbtOptions(numTests: 200, seed: BigInt.from(2909)),
          (rng) {
            final target = Gen.tuple2(smallInt(0, 25), wordGen(0, 4))
                .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
                .sample(rng);
            final value = smallInt(0, 999).sample(rng).toString();
            return (target, value);
          },
          (input) {
            final (target, value) = input;
            for (final op in ['=.', '=:']) {
              final tokens = tryTokenize('$target$op$value');
              if (tokens == null || tokens.length != 4) return false;
              if (tokens[0].type != 'NAME' ||
                  tokens[2].type != 'NUMBER' ||
                  tokens[2].value != value) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });
  });
}
