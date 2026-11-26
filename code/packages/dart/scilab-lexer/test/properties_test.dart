/// Property tests for `coding_adventures_scilab_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// paren-newline hook's depth rule as an independent mirror, operator
/// and percent-constant atomicity, comment invisibility, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_scilab_lexer/scilab_lexer.dart';
import 'package:coding_adventures_scilab_lexer/src/_grammar.dart';
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

/// Scilab-dense hostile text: every Scilab sigil plus whitespace,
/// quotes, comment openers, and a little out-of-alphabet noise.
Gen<String> hostileScilab(int minLen, int maxLen) {
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
    '//',
    '/*',
    '*/',
    '%',
    '%pi',
    '%foo',
    r'$',
    '.',
    '.*',
    './',
    '.^',
    r'.\',
    ".'",
    '**',
    '==',
    '~=',
    '<>',
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
          BigInt.from(4201),
          PbtOptions(numTests: 400, seed: BigInt.from(4201)),
          (rng) => hostileScilab(0, 60).sample(rng),
          (input) {
            try {
              tokenizeScilab(input);
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
          BigInt.from(4202),
          PbtOptions(numTests: 400, seed: BigInt.from(4202)),
          (rng) => hostileScilab(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeScilab(input);
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
          BigInt.from(4203),
          PbtOptions(numTests: 200, seed: BigInt.from(4203)),
          (rng) => hostileScilab(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeScilab(input);
            final b = tryTokenizeScilab(input);
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
          BigInt.from(4204),
          PbtOptions(numTests: 400, seed: BigInt.from(4204)),
          (rng) => hostileScilab(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeScilab(input);
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
          BigInt.from(4205),
          PbtOptions(numTests: 300, seed: BigInt.from(4205)),
          (rng) => Gen.list(1, 20, smallInt(0, 9))
              .map((idx) => idx
                  .map((i) =>
                      ['(', ')', '[', ']', '{', '}', '\n', 'a', '1', ';'][i])
                  .join())
              .sample(rng),
          (input) {
            // Quote-free soup: the quote-protection pre-hook is the
            // identity here, so the raw grammar run and the hooked run
            // differ ONLY by the paren-newline rule the mirror replays.
            final hooked = tryTokenizeScilab(input);
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
          BigInt.from(4206),
          PbtOptions(numTests: 100, seed: BigInt.from(4206)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const ops = [
              ('==', 'EQ_EQ'),
              ('~=', 'NE'),
              ('<>', 'NE_ALT'),
              ('<=', 'LE'),
              ('>=', 'GE'),
              ('&&', 'AND_AND'),
              ('||', 'OR_OR'),
              ('.*', 'ELEM_MUL'),
              ('./', 'ELEM_RDIV'),
              ('.^', 'ELEM_POW'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenizeScilab(text);
            return tokens != null &&
                tokens.length == 2 && // operator + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('percent constants are atomic single tokens', () {
      check(
        'percent-atomic',
        checkProperty<int>(
          BigInt.from(4207),
          PbtOptions(numTests: 100, seed: BigInt.from(4207)),
          (rng) => smallInt(0, 7).sample(rng),
          (index) {
            const consts = [
              '%pi',
              '%e',
              '%i',
              '%inf',
              '%nan',
              '%eps',
              '%t',
              '%f',
            ];
            final text = consts[index];
            final tokens = tryTokenizeScilab(text);
            return tokens != null &&
                tokens.length == 2 && // constant + EOF
                tokens[0].type == 'PERCENT_CONST' &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(4208),
          PbtOptions(numTests: 200, seed: BigInt.from(4208)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenizeScilab('1 /* $body */ + 2');
            if (tokens == null) return false;
            final sig = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return sig == 'NUMBER=1|PLUS=+|NUMBER=2';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4209),
          PbtOptions(numTests: 400, seed: BigInt.from(4209)),
          (rng) => hostileScilab(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeScilab(input);
            try {
              final hard = tokenizeScilab(input);
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
