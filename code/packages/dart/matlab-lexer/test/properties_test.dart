/// Property tests for `coding_adventures_matlab_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// block-comment strip and continuation splice as direct hook
/// properties, the paren-newline hook's depth rule as an independent
/// mirror, operator atomicity, and API agreement — all with fixed
/// seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_matlab_lexer/matlab_lexer.dart';
import 'package:coding_adventures_matlab_lexer/src/_grammar.dart';
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

/// MATLAB-dense hostile text: quotes, comments, continuations, every
/// MATLAB sigil, whitespace, and out-of-alphabet noise.
Gen<String> hostileMatlab(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    "'",
    '"',
    '`',
    '%',
    '%{',
    '%}',
    '...',
    '..',
    '.',
    '.*',
    './',
    '.^',
    r'.\',
    ".'",
    '**',
    '==',
    '~=',
    '<',
    '<=',
    '>',
    '>=',
    '&&',
    '||',
    '&',
    '|',
    '~',
    '=',
    '+',
    '-',
    '*',
    '/',
    r'\',
    '^',
    ',',
    ';',
    ':',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '@',
    '#',
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
          BigInt.from(4801),
          PbtOptions(numTests: 400, seed: BigInt.from(4801)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            try {
              tokenizeMatlab(input);
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
          BigInt.from(4802),
          PbtOptions(numTests: 400, seed: BigInt.from(4802)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMatlab(input);
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
          BigInt.from(4803),
          PbtOptions(numTests: 200, seed: BigInt.from(4803)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeMatlab(input);
            final b = tryTokenizeMatlab(input);
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
          BigInt.from(4804),
          PbtOptions(numTests: 400, seed: BigInt.from(4804)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMatlab(input);
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

    test('hook matches its depth mirror on quote-free soup', () {
      check(
        'hook-mirror',
        checkProperty<String>(
          BigInt.from(4805),
          PbtOptions(numTests: 300, seed: BigInt.from(4805)),
          (rng) => Gen.list(1, 20, smallInt(0, 9))
              .map((idx) => idx
                  .map((i) =>
                      ['(', ')', '[', ']', '{', '}', '\n', 'a', '1', ';'][i])
                  .join())
              .sample(rng),
          (input) {
            // Quote-free, comment-free, continuation-free soup: both
            // pre-hooks are the identity here (no quotes, no `%`, no
            // dots), so the raw grammar run and the hooked run differ
            // ONLY by the paren-newline rule the mirror replays.
            final hooked = tryTokenizeMatlab(input);
            if (hooked == null) return false;
            List<Token> raw;
            try {
              raw = GrammarLexer(input, tokenGrammar).tokenize();
            } on LexerError {
              return false;
            }
            // Independent inline mirror: parens alone count, and an
            // unbalanced close saturates at zero instead of going
            // negative (which would wrongly keep a later newline).
            final predicted = <Token>[];
            var depth = 0;
            for (final token in raw) {
              if (!(token.type == 'NEWLINE' && depth > 0)) {
                predicted.add(token);
              }
              if (token.type == 'LPAREN') {
                depth += 1;
              } else if (token.type == 'RPAREN') {
                if (depth > 0) depth -= 1;
              }
            }
            if (predicted.length != hooked.length) return false;
            for (var i = 0; i < predicted.length; i++) {
              if (predicted[i] != hooked[i]) return false;
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
          BigInt.from(4806),
          PbtOptions(numTests: 100, seed: BigInt.from(4806)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const ops = [
              ('==', 'EQ_EQ'),
              ('~=', 'NE'),
              ('<=', 'LE'),
              ('>=', 'GE'),
              ('&&', 'AND_AND'),
              ('||', 'OR_OR'),
              ('.*', 'ELEM_MUL'),
              ('./', 'ELEM_RDIV'),
              ('.^', 'ELEM_POW'),
              (".'", 'ELEM_TRANSPOSE'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenizeMatlab(text);
            return tokens != null &&
                tokens.length == 2 && // operator + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('block comments vanish line-count intact', () {
      check(
        'block-strip',
        checkProperty<String>(
          BigInt.from(4807),
          PbtOptions(numTests: 200, seed: BigInt.from(4807)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            // Alone-on-line markers blank to newlines: the stripped
            // text keeps every newline, so later lines keep their
            // numbers.
            final stripped =
                stripBlockComments('x = 1\n%{\n$body\n%}\ny = 2\n');
            if (stripped != 'x = 1\n\n\n\ny = 2\n') return false;
            // But inline markers are NOT comments (MATLAB requires
            // alone-on-line): the line survives to the grammar, which
            // skips `%` to end of line.
            final tokens = tryTokenizeMatlab('x = 1 %{ not alone\n');
            if (tokens == null) return false;
            return tokens
                    .where((t) => t.type != 'EOF' && t.type != 'NEWLINE')
                    .map((t) => t.type)
                    .join(',') ==
                'NAME,EQ,NUMBER';
          },
        ),
      );
    });

    test('continuation joins lines', () {
      check(
        'continuation-splice',
        checkProperty<String>(
          BigInt.from(4808),
          PbtOptions(numTests: 200, seed: BigInt.from(4808)),
          (rng) => wordGen(0, 10).sample(rng),
          (tail) {
            // `...` plus the rest of the line and its newline vanish:
            // `1 + ...TAIL\n2` lexes as the single expression `1 + 2`.
            final tokens = tryTokenizeMatlab('1 + ...$tail\n2\n');
            if (tokens == null) return false;
            return tokens
                    .where((t) => t.type != 'EOF' && t.type != 'NEWLINE')
                    .map((t) => '${t.type}=${t.value}')
                    .join('|') ==
                'NUMBER=1|PLUS=+|NUMBER=2';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4809),
          PbtOptions(numTests: 400, seed: BigInt.from(4809)),
          (rng) => hostileMatlab(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeMatlab(input);
            try {
              final hard = tokenizeMatlab(input);
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
