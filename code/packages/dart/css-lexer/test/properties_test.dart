/// Property tests for `coding_adventures_css_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text plus
/// CSS priority and trivia-invisibility checks over generated rules —
/// all with fixed seeds.
library;

import 'package:coding_adventures_css_lexer/css_lexer.dart';
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

Gen<String> identGen(int maxLen) {
  const letters = 'abcdefghijklmnopqrstuvwxyz';
  return Gen.tuple2(smallInt(0, letters.length - 1), wordGen(0, maxLen - 1))
      .map((t) => '${letters[t.$1]}${t.$2}');
}

/// Markup-dense hostile text: every CSS sigil plus whitespace, quotes,
/// and a little Unicode.
Gen<String> hostileCss(int minLen, int maxLen) {
  const pieces = [
    '{',
    '}',
    '(',
    ')',
    '[',
    ']',
    ':',
    ';',
    ',',
    '.',
    '+',
    '>',
    '~',
    '*',
    '|',
    '!',
    '/',
    '=',
    '&',
    '-',
    '#',
    '@',
    '%',
    '"',
    "'",
    '\\',
    ' ',
    '\n',
    '\r',
    '\t',
    'a',
    'Z',
    '0',
    '::',
    '<!--',
    '-->',
    '/*',
    '*/',
    'url(',
    'U+',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

List<Token>? tryTokenize(String source) {
  try {
    return tokenizeCss(source);
  } on LexerError {
    return null;
  }
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

/// Generates `tag { prop: value; }` rules with identifier values.
Gen<String> ruleGen() => Gen.tuple3(identGen(5), identGen(5), identGen(5))
    .map((t) => '${t.$1} { ${t.$2}: ${t.$3}; }');

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(2401),
          PbtOptions(numTests: 400, seed: BigInt.from(2401)),
          (rng) => hostileCss(0, 60).sample(rng),
          (input) {
            try {
              tokenizeCss(input);
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
          BigInt.from(2402),
          PbtOptions(numTests: 400, seed: BigInt.from(2402)),
          (rng) => hostileCss(0, 60).sample(rng),
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
          BigInt.from(2403),
          PbtOptions(numTests: 200, seed: BigInt.from(2403)),
          (rng) => hostileCss(0, 60).sample(rng),
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
          BigInt.from(2404),
          PbtOptions(numTests: 400, seed: BigInt.from(2404)),
          (rng) => hostileCss(0, 60).sample(rng),
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

    test('generated rules balance structurally', () {
      check(
        'rule-balance',
        checkProperty<String>(
          BigInt.from(2405),
          PbtOptions(numTests: 300, seed: BigInt.from(2405)),
          (rng) => ruleGen().sample(rng),
          (input) {
            final tokens = tryTokenize(input);
            if (tokens == null) return false;
            return countType(tokens, 'LBRACE') == 1 &&
                countType(tokens, 'RBRACE') == 1 &&
                countType(tokens, 'COLON') == 1 &&
                countType(tokens, 'SEMICOLON') == 1 &&
                countType(tokens, 'IDENT') == 3;
          },
        ),
      );
    });

    test('priority tokens never split', () {
      check(
        'priority-atomic',
        checkProperty<(String, String)>(
          BigInt.from(2406),
          PbtOptions(numTests: 200, seed: BigInt.from(2406)),
          (rng) {
            final num = smallInt(1, 999).sample(rng).toString();
            // Units are letters-only and never start with `e` (that
            // hits the documented scientific-notation quirk).
            const letters = 'abcdfghijklmnopqrstuvwxyz';
            final unit = Gen.list(1, 3, smallInt(0, letters.length - 1))
                .map((idx) => idx.map((i) => letters[i]).join())
                .sample(rng);
            return (num, unit);
          },
          (input) {
            final (num, unit) = input;
            bool single(String src, String type, String value) {
              final tokens = tryTokenize(src);
              return tokens != null &&
                  tokens.length == 2 && // token + EOF
                  tokens[0].type == type &&
                  tokens[0].value == value;
            }

            if (!single('$num$unit', 'DIMENSION', '$num$unit')) {
              return false;
            }
            if (!single('$num%', 'PERCENTAGE', '$num%')) return false;
            if (!single('--$unit', 'CUSTOM_PROPERTY', '--$unit')) {
              return false;
            }
            if (!single('::', 'COLON_COLON', '::')) return false;
            // `::` still splits from a following identifier.
            final split = tryTokenize('::$unit');
            return split != null &&
                split.length == 3 &&
                split[0].type == 'COLON_COLON' &&
                split[1].type == 'IDENT' &&
                split[1].value == unit;
          },
        ),
      );
    });

    test('quoted strings lex whole with quotes stripped', () {
      check(
        'string-whole',
        checkProperty<String>(
          BigInt.from(2407),
          PbtOptions(numTests: 200, seed: BigInt.from(2407)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            for (final quote in ['"', "'"]) {
              final tokens = tryTokenize('$quote$body$quote');
              if (tokens == null || tokens.length != 2) return false;
              if (tokens[0].type != 'STRING') return false;
              if (tokens[0].value != body) return false;
            }
            return true;
          },
        ),
      );
    });

    test('trivia never surfaces as tokens', () {
      check(
        'trivia-invisible',
        checkProperty<String>(
          BigInt.from(2408),
          PbtOptions(numTests: 200, seed: BigInt.from(2408)),
          (rng) => ruleGen().sample(rng),
          (input) {
            final plain = tryTokenize(input)!;
            final padded = tryTokenize('  /* c */ $input /* c */  \n');
            if (padded == null) return false;
            String signature(List<Token> tokens) => tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return signature(plain) == signature(padded);
          },
        ),
      );
    });
  });
}
