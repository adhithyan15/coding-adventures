/// Property tests for `coding_adventures_idl_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// keyword folding over the grammar's own keyword table, name case
/// preservation, string wholeness, comment invisibility, and nullable
/// API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_idl_lexer/idl_lexer.dart';
import 'package:coding_adventures_idl_lexer/src/_grammar.dart';
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
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789_';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Markup-dense hostile text: every IDL sigil plus whitespace, quotes,
/// and a little Unicode.
Gen<String> hostileIdl(int minLen, int maxLen) {
  const pieces = [
    '=',
    '+',
    '-',
    '*',
    '/',
    '^',
    '#',
    '##',
    '&',
    ';',
    ',',
    ':',
    '[',
    ']',
    '(',
    ')',
    '\$',
    "'",
    '"',
    ' ',
    '\n',
    '\r',
    '\t',
    'a',
    'Z',
    '0',
    '_',
    '.',
    'IF',
    'if',
    'EQ',
    'END',
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
          BigInt.from(2501),
          PbtOptions(numTests: 400, seed: BigInt.from(2501)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            try {
              tokenizeIdl(input);
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
          BigInt.from(2502),
          PbtOptions(numTests: 400, seed: BigInt.from(2502)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeIdl(input);
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
          BigInt.from(2503),
          PbtOptions(numTests: 200, seed: BigInt.from(2503)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeIdl(input);
            final b = tryTokenizeIdl(input);
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
          BigInt.from(2504),
          PbtOptions(numTests: 400, seed: BigInt.from(2504)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeIdl(input);
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

    test('every keyword folds to uppercase in any case', () {
      check(
        'keyword-fold',
        checkProperty<(int, int)>(
          BigInt.from(2505),
          PbtOptions(numTests: 300, seed: BigInt.from(2505)),
          (rng) {
            final word =
                smallInt(0, tokenGrammar.keywords.length - 1).sample(rng);
            final casing = smallInt(0, 2).sample(rng);
            return (word, casing);
          },
          (input) {
            final (word, casing) = input;
            final keyword = tokenGrammar.keywords[word];
            final spelled = switch (casing) {
              0 => keyword.toLowerCase(),
              1 => keyword
                  .split('')
                  .asMap()
                  .entries
                  .map((e) => e.key.isEven
                      ? e.value.toLowerCase()
                      : e.value.toUpperCase())
                  .join(),
              _ => keyword,
            };
            final tokens = tryTokenizeIdl(spelled);
            return tokens != null &&
                tokens.length == 2 && // keyword + EOF
                tokens[0].type == 'KEYWORD' &&
                tokens[0].value == keyword;
          },
        ),
      );
    });

    test('non-keyword names keep exact case', () {
      check(
        'name-case',
        checkProperty<String>(
          BigInt.from(2506),
          PbtOptions(numTests: 200, seed: BigInt.from(2506)),
          // Letter + digits can never be a keyword (all are pure
          // letters), so every sample is a NAME by construction.
          (rng) => Gen.tuple2(smallInt(0, 25), wordGen(0, 5))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}9')
              .sample(rng),
          (name) {
            final tokens = tryTokenizeIdl(name);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NAME' &&
                tokens[0].value == name;
          },
        ),
      );
    });

    test('quoted strings lex whole with quotes stripped', () {
      check(
        'string-whole',
        checkProperty<String>(
          BigInt.from(2507),
          PbtOptions(numTests: 200, seed: BigInt.from(2507)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            for (final quote in ["'", '"']) {
              final tokens = tryTokenizeIdl('$quote$body$quote');
              if (tokens == null || tokens.length != 2) return false;
              if (tokens[0].type != 'STRING') return false;
              if (tokens[0].value != body) return false;
            }
            return true;
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(2508),
          PbtOptions(numTests: 200, seed: BigInt.from(2508)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final bare = tryTokenizeIdl('x = 1\n');
            final commented = tryTokenizeIdl('x = 1 ;$body\n');
            if (bare == null || commented == null) return false;
            String signature(List<Token> tokens) => tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return signature(bare) == signature(commented);
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(2509),
          PbtOptions(numTests: 400, seed: BigInt.from(2509)),
          (rng) => hostileIdl(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeIdl(input);
            try {
              final hard = tokenizeIdl(input);
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
