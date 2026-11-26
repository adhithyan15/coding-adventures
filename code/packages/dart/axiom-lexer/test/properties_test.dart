/// Property tests for `coding_adventures_axiom_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text,
/// longest-match priority, number shapes, keyword exactness, and
/// comment invisibility — all with fixed seeds.
library;

import 'package:coding_adventures_axiom_lexer/axiom_lexer.dart';
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

/// Markup-dense hostile text: every Axiom sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileAxiom(int minLen, int maxLen) {
  const pieces = [
    ':',
    ':=',
    '::',
    '==',
    '=',
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
    '~=',
    '<>',
    '(',
    ')',
    '[',
    ']',
    ',',
    ';',
    '"',
    '--',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '.',
    'if',
    'has',
    '@',
    '\$',
    '!',
    '?',
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
          BigInt.from(3001),
          PbtOptions(numTests: 400, seed: BigInt.from(3001)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            try {
              tokenizeAxiom(input);
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
          BigInt.from(3002),
          PbtOptions(numTests: 400, seed: BigInt.from(3002)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeAxiom(input);
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
          BigInt.from(3003),
          PbtOptions(numTests: 200, seed: BigInt.from(3003)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeAxiom(input);
            final b = tryTokenizeAxiom(input);
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
          BigInt.from(3004),
          PbtOptions(numTests: 400, seed: BigInt.from(3004)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeAxiom(input);
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
          BigInt.from(3005),
          PbtOptions(numTests: 100, seed: BigInt.from(3005)),
          (rng) => smallInt(0, 7).sample(rng),
          (index) {
            const ops = [
              (':=', 'ASSIGN'),
              ('::', 'COERCE'),
              ('==', 'DEFINE'),
              ('**', 'POW'),
              ('<=', 'LE'),
              ('>=', 'GE'),
              ('~=', 'NE'),
              ('^', 'CARET'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenizeAxiom(text);
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
          BigInt.from(3006),
          PbtOptions(numTests: 200, seed: BigInt.from(3006)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            Gen.tuple2(smallInt(0, 99), smallInt(0, 99))
                .map((t) => '${t.$1}.${t.$2}'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenizeAxiom(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NUMBER' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('keywords match exactly and case-sensitively', () {
      check(
        'keyword-exact',
        checkProperty<int>(
          BigInt.from(3007),
          PbtOptions(numTests: 100, seed: BigInt.from(3007)),
          (rng) => smallInt(0, 3).sample(rng),
          (index) {
            const keywords = ['if', 'then', 'else', 'has'];
            final kw = keywords[index];
            final hit = tryTokenizeAxiom(kw);
            final miss =
                tryTokenizeAxiom('${kw[0].toUpperCase()}${kw.substring(1)}');
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

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(3008),
          PbtOptions(numTests: 200, seed: BigInt.from(3008)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenizeAxiom('a := 1 --$body\nb := 2');
            if (tokens == null) return false;
            final kinds = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => t.type)
                .join('|');
            return kinds == 'NAME|ASSIGN|NUMBER|NAME|ASSIGN|NUMBER';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(3009),
          PbtOptions(numTests: 400, seed: BigInt.from(3009)),
          (rng) => hostileAxiom(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeAxiom(input);
            try {
              final hard = tokenizeAxiom(input);
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
