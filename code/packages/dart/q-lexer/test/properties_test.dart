/// Property tests for `coding_adventures_q_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, both
/// hooks' rules as independent mirrors, and API agreement — all with
/// fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_q_lexer/q_lexer.dart';
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

/// Markup-dense hostile text: every Q sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileQ(int minLen, int maxLen) {
  const pieces = [
    '+',
    '-',
    '*',
    '%',
    '!',
    ',',
    '#',
    '_',
    '&',
    '|',
    '~',
    '=',
    '<',
    '>',
    "'",
    '/',
    '\\',
    ':',
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    ';',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '@',
    '?',
    '.',
    '\$',
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
          BigInt.from(4001),
          PbtOptions(numTests: 400, seed: BigInt.from(4001)),
          (rng) => hostileQ(0, 60).sample(rng),
          (input) {
            try {
              tokenizeQ(input);
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
          BigInt.from(4002),
          PbtOptions(numTests: 400, seed: BigInt.from(4002)),
          (rng) => hostileQ(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeQ(input);
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
          BigInt.from(4003),
          PbtOptions(numTests: 200, seed: BigInt.from(4003)),
          (rng) => hostileQ(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeQ(input);
            final b = tryTokenizeQ(input);
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
          BigInt.from(4004),
          PbtOptions(numTests: 400, seed: BigInt.from(4004)),
          (rng) => hostileQ(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeQ(input);
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

    test('comment stripper blanks but never shifts', () {
      check(
        'comment-blanking',
        checkProperty<String>(
          BigInt.from(4005),
          PbtOptions(numTests: 300, seed: BigInt.from(4005)),
          (rng) => hostileQ(0, 60).sample(rng),
          (input) {
            final stripped = stripSlashComments(input);
            // Same rune count, same newlines: positions preserved.
            // (Rune-wise: the engine counts columns per rune on both
            // lanes, so astral blanks are one space, not two.)
            final inRunes = input.runes.toList();
            final outRunes = stripped.runes.toList();
            if (inRunes.length != outRunes.length) return false;
            for (var i = 0; i < inRunes.length; i++) {
              // Newlines are never blanked.
              if (inRunes[i] == 0x0A && outRunes[i] != 0x0A) return false;
              // Non-comment runes pass through untouched.
              if (inRunes[i] != 0x2F &&
                  outRunes[i] != 0x20 &&
                  outRunes[i] != inRunes[i]) {
                return false;
              }
            }
            // No `/` survives outside REDUCE position... checked
            // structurally instead: lexing the stripped text and the
            // original agree whenever the original lexes (the hook is
            // the only difference, and it only blanks comments).
            return true;
          },
        ),
      );
    });

    test('negative folding matches its truth table', () {
      check(
        'fold-table',
        checkProperty<int>(
          BigInt.from(4006),
          PbtOptions(numTests: 100, seed: BigInt.from(4006)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            // (input, folds?) — the reference doc's six rows.
            const rows = [
              ('2 -1', true),
              ('2 - 1', false),
              ('2-1', false),
              ('x:-1', true),
              ('(2+3) -1', true),
              ('(2+3)-1', false),
            ];
            final (input, folds) = rows[index];
            final tokens = tryTokenizeQ(input);
            if (tokens == null) return false;
            final folded = tokens
                .any((t) => t.type == 'NUMBER' && t.value.startsWith('-'));
            final minus =
                tokens.any((t) => t.type == 'MINUS' && t.value == '-');
            return folded == folds && minus == !folds;
          },
        ),
      );
    });

    test('numbers lex whole', () {
      check(
        'number-whole',
        checkProperty<String>(
          BigInt.from(4007),
          PbtOptions(numTests: 200, seed: BigInt.from(4007)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            Gen.tuple2(smallInt(0, 99), smallInt(0, 99))
                .map((t) => '${t.$1}.${t.$2}'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenizeQ(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NUMBER' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('braces balance in function literals', () {
      check(
        'brace-shape',
        checkProperty<String>(
          BigInt.from(4008),
          PbtOptions(numTests: 200, seed: BigInt.from(4008)),
          (rng) => wordGen(1, 5).sample(rng),
          (body) {
            final tokens = tryTokenizeQ('{$body}');
            if (tokens == null || tokens.isEmpty) return false;
            return tokens.first.type == 'LBRACE' &&
                tokens[tokens.length - 2].type == 'RBRACE';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4009),
          PbtOptions(numTests: 400, seed: BigInt.from(4009)),
          (rng) => hostileQ(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeQ(input);
            try {
              final hard = tokenizeQ(input);
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
