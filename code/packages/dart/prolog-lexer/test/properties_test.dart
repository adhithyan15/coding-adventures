/// Property tests for `coding_adventures_prolog_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// atom/variable discrimination, number shapes, alias behavior, and
/// comment invisibility — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_prolog_lexer/prolog_lexer.dart';
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

/// Markup-dense hostile text: every Prolog sigil plus whitespace,
/// quotes, and a little Unicode.
Gen<String> hostileProlog(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    ',',
    '.',
    ':',
    ':-',
    '?-',
    '-->',
    '!',
    '|',
    '_',
    "'",
    '"',
    '%',
    ' ',
    '\n',
    '\r',
    '\t',
    'a',
    'Z',
    '0',
    '=',
    '>',
    '<',
    '+',
    '-',
    '*',
    '/',
    '\\',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeIsoProlog(source);
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
          BigInt.from(2701),
          PbtOptions(numTests: 400, seed: BigInt.from(2701)),
          (rng) => hostileProlog(0, 60).sample(rng),
          (input) {
            try {
              tokenizeIsoProlog(input);
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
          BigInt.from(2702),
          PbtOptions(numTests: 400, seed: BigInt.from(2702)),
          (rng) => hostileProlog(0, 60).sample(rng),
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
          BigInt.from(2703),
          PbtOptions(numTests: 200, seed: BigInt.from(2703)),
          (rng) => hostileProlog(0, 60).sample(rng),
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
          BigInt.from(2704),
          PbtOptions(numTests: 400, seed: BigInt.from(2704)),
          (rng) => hostileProlog(0, 60).sample(rng),
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

    test('case decides atom versus variable', () {
      check(
        'atom-variable-split',
        checkProperty<String>(
          BigInt.from(2705),
          PbtOptions(numTests: 200, seed: BigInt.from(2705)),
          (rng) => wordGen(0, 8).sample(rng),
          (rest) {
            bool single(String src, String type) {
              final tokens = tryTokenize(src);
              return tokens != null &&
                  tokens.length == 2 && // token + EOF
                  tokens[0].type == type &&
                  tokens[0].value == src;
            }

            return single('a$rest', 'ATOM') &&
                single('Z$rest', 'VARIABLE') &&
                single('_$rest', rest.isEmpty ? 'ANON_VAR' : 'VARIABLE');
          },
        ),
      );
    });

    test('integers lex whole', () {
      check(
        'integer-whole',
        checkProperty<String>(
          BigInt.from(2706),
          PbtOptions(numTests: 200, seed: BigInt.from(2706)),
          (rng) => smallInt(0, 99999).map((i) => i.toString()).sample(rng),
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

    test('arrows never split', () {
      check(
        'arrow-atomic',
        checkProperty<int>(
          BigInt.from(2707),
          PbtOptions(numTests: 100, seed: BigInt.from(2707)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const arrows = [(':-', 'RULE'), ('?-', 'QUERY'), ('-->', 'DCG')];
            final (text, type) = arrows[index];
            final tokens = tryTokenize(text);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(2708),
          PbtOptions(numTests: 200, seed: BigInt.from(2708)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenize('a. %$body\nb.');
            if (tokens == null) return false;
            final kinds = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => t.type)
                .join('|');
            // The whitespace skip swallows newlines too, so no NEWLINE
            // token appears (both lanes — the reference's own
            // whitespace test pins the empty result).
            return kinds == 'ATOM|DOT|ATOM|DOT';
          },
        ),
      );
    });

    test('facts keep functor, args, and terminator', () {
      check(
        'fact-shape',
        checkProperty<(String, String)>(
          BigInt.from(2709),
          PbtOptions(numTests: 200, seed: BigInt.from(2709)),
          (rng) {
            final functor = Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
                .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
                .sample(rng);
            final arg = Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
                .map((t) => '${'ABCDEFGHIJKLMNOPQRSTUVWXYZ'[t.$1]}${t.$2}')
                .sample(rng);
            return (functor, arg);
          },
          (input) {
            final (functor, arg) = input;
            final tokens = tryTokenize('$functor($arg).');
            if (tokens == null || tokens.length != 6) return false;
            return tokens[0].type == 'ATOM' &&
                tokens[1].type == 'LPAREN' &&
                tokens[2].type == 'VARIABLE' &&
                tokens[2].value == arg &&
                tokens[3].type == 'RPAREN' &&
                tokens[4].type == 'DOT';
          },
        ),
      );
    });
  });
}
