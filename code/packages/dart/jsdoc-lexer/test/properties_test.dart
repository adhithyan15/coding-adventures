/// Property tests for `coding_adventures_jsdoc_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, tag
/// and ellipsis atomicity, whitespace insensitivity, leftover-star
/// invisibility, and factory agreement — all with fixed seeds.
///
/// The reference offers no `Result` twin (callers use factory +
/// `tokenize()`), so this lane has no nullable API either; [soft]
/// is a test-local helper, not a product API.
library;

import 'package:coding_adventures_jsdoc_lexer/jsdoc_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// Test-local nullable twin: `null` on [LexerError].
List<Token>? soft(String input) {
  try {
    return tokenizeJsdoc(input);
  } on LexerError {
    return null;
  }
}

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// JSDoc-dense hostile text: tags, type-expression sigils, comment
/// markers, whitespace, and out-of-alphabet noise.
Gen<String> hostileJsdoc(int minLen, int maxLen) {
  const pieces = [
    '@type',
    '@param',
    '@returns',
    '@',
    '{',
    '}',
    '[',
    ']',
    '(',
    ')',
    '<',
    '>',
    '|',
    '&',
    ',',
    ':',
    '=',
    '?',
    '!',
    '...',
    '.',
    '*',
    '/**',
    '*/',
    '"',
    "'",
    '\n',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '_',
    '\$',
    '`',
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
          BigInt.from(4501),
          PbtOptions(numTests: 400, seed: BigInt.from(4501)),
          (rng) => hostileJsdoc(0, 60).sample(rng),
          (input) {
            try {
              tokenizeJsdoc(input);
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
          BigInt.from(4502),
          PbtOptions(numTests: 400, seed: BigInt.from(4502)),
          (rng) => hostileJsdoc(0, 60).sample(rng),
          (input) {
            final tokens = soft(input);
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
          BigInt.from(4503),
          PbtOptions(numTests: 200, seed: BigInt.from(4503)),
          (rng) => hostileJsdoc(0, 60).sample(rng),
          (input) {
            final a = soft(input);
            final b = soft(input);
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
          BigInt.from(4504),
          PbtOptions(numTests: 400, seed: BigInt.from(4504)),
          (rng) => hostileJsdoc(0, 60).sample(rng),
          (input) {
            final tokens = soft(input);
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

    test('at tags never split', () {
      check(
        'tag-atomic',
        checkProperty<int>(
          BigInt.from(4505),
          PbtOptions(numTests: 100, seed: BigInt.from(4505)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const tags = [
              'type',
              'param',
              'returns',
              'typedef',
              'template',
              'deprecated',
            ];
            final text = '@${tags[index]}';
            final tokens = soft(text);
            return tokens != null &&
                tokens.length == 2 && // tag + EOF
                tokens[0].type == 'AT_TAG' &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('ellipsis never splits into dots', () {
      check(
        'ellipsis-atomic',
        checkProperty<String>(
          BigInt.from(4506),
          PbtOptions(numTests: 100, seed: BigInt.from(4506)),
          (rng) =>
              smallInt(0, 1).map((i) => ['...', '{...string}'][i]).sample(rng),
          (text) {
            final tokens = soft(text);
            if (tokens == null) return false;
            return tokens
                    .any((t) => t.type == 'ELLIPSIS' && t.value == '...') &&
                tokens.every((t) => t.type != 'DOT');
          },
        ),
      );
    });

    test('inter-token padding never changes the type sequence', () {
      check(
        'whitespace-insensitive',
        checkProperty<String>(
          BigInt.from(4507),
          PbtOptions(numTests: 200, seed: BigInt.from(4507)),
          (rng) {
            const pads = [' ', '  ', '\t', ' \t '];
            final p = Gen.list(4, 4, smallInt(0, pads.length - 1))
                .map((idx) => idx.map((i) => pads[i]).join('|'))
                .sample(rng);
            return p;
          },
          (padKey) {
            final pads = padKey.split('|');
            final input =
                '@type${pads[0]}{${pads[1]}number${pads[2]}}${pads[3]}\n';
            final tokens = soft(input);
            if (tokens == null) return false;
            final seq = tokens.map((t) => t.type).join(',');
            return seq == 'AT_TAG,LBRACE,NAME,RBRACE,NEWLINE,EOF';
          },
        ),
      );
    });

    test('leftover star lines vanish without a trace', () {
      check(
        'star-invisible',
        checkProperty<int>(
          BigInt.from(4508),
          PbtOptions(numTests: 100, seed: BigInt.from(4508)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const tags = ['type', 'param', 'returns'];
            final tag = tags[index];
            // The safety-net skip eats `\n * ` wholesale, so a star-only
            // line between two tags leaves the exact same stream as no
            // line at all.
            final starred = '@type {number}\n * \n@$tag {string} x\n';
            final starless = '@type {number}\n@$tag {string} x\n';
            final a = soft(starred);
            final b = soft(starless);
            if (a == null || b == null) return false;
            if (a.length != b.length) return false;
            // (type, value) only: line numbers necessarily shift past
            // the vanished star line.
            for (var i = 0; i < a.length; i++) {
              if (a[i].type != b[i].type || a[i].value != b[i].value) {
                return false;
              }
            }
            return a.every((t) => !t.type.contains('STAR'));
          },
        ),
      );
    });

    test('factory agrees with the one-shot', () {
      check(
        'factory-agreement',
        checkProperty<String>(
          BigInt.from(4509),
          PbtOptions(numTests: 400, seed: BigInt.from(4509)),
          (rng) => hostileJsdoc(0, 60).sample(rng),
          (input) {
            final oneShot = soft(input);
            List<Token> viaFactory;
            try {
              viaFactory = createJsdocLexer(input).tokenize();
            } on LexerError {
              return oneShot == null;
            }
            if (oneShot == null) return false;
            if (oneShot.length != viaFactory.length) return false;
            for (var i = 0; i < oneShot.length; i++) {
              if (oneShot[i] != viaFactory[i]) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
