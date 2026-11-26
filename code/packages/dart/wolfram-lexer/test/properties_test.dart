/// Property tests for `coding_adventures_wolfram_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// hook's depth rule (braces included, double-bracket counts double)
/// as a mirror model, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_wolfram_lexer/src/_grammar.dart';
import 'package:coding_adventures_wolfram_lexer/wolfram_lexer.dart';
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

/// Markup-dense hostile text: every Wolfram sigil plus whitespace
/// and a little out-of-alphabet noise.
Gen<String> hostileWolfram(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '[[',
    '{',
    '}',
    '->',
    ':>',
    ':=',
    '/.',
    '//.',
    '/;',
    '/@',
    '@@',
    '&',
    '&&',
    '|',
    '||',
    '?',
    '_',
    '#',
    '##',
    '=',
    '==',
    '!=',
    '<',
    '<=',
    '>',
    '>=',
    '+',
    '-',
    '*',
    '/',
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
    '~',
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
          BigInt.from(4101),
          PbtOptions(numTests: 400, seed: BigInt.from(4101)),
          (rng) => hostileWolfram(0, 60).sample(rng),
          (input) {
            try {
              tokenizeWolfram(input);
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
          BigInt.from(4102),
          PbtOptions(numTests: 400, seed: BigInt.from(4102)),
          (rng) => hostileWolfram(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeWolfram(input);
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
          BigInt.from(4103),
          PbtOptions(numTests: 200, seed: BigInt.from(4103)),
          (rng) => hostileWolfram(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeWolfram(input);
            final b = tryTokenizeWolfram(input);
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
          BigInt.from(4104),
          PbtOptions(numTests: 400, seed: BigInt.from(4104)),
          (rng) => hostileWolfram(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeWolfram(input);
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

    test('hook matches its depth mirror on bracket soup', () {
      check(
        'hook-mirror',
        checkProperty<String>(
          BigInt.from(4105),
          PbtOptions(numTests: 300, seed: BigInt.from(4105)),
          (rng) => Gen.list(1, 20, smallInt(0, 7))
              .map((idx) => idx
                  .map((i) => ['(', ')', '[', ']', '[[', '{', '}', '\n'][i])
                  .join())
              .sample(rng),
          (input) {
            final hooked = tryTokenizeWolfram(input);
            if (hooked == null) return false;
            List<Token> raw;
            try {
              raw = GrammarLexer(input, tokenGrammar).tokenize();
            } on LexerError {
              return false;
            }
            // Independent inline mirror: braces count, `[[` is +2.
            final predicted = <Token>[];
            var depth = 0;
            for (final token in raw) {
              if (!(token.type == 'NEWLINE' && depth > 0)) {
                predicted.add(token);
              }
              if (token.type == 'LDBRACKET') {
                depth += 2;
              } else if (token.type == 'LPAREN' ||
                  token.type == 'LBRACKET' ||
                  token.type == 'LBRACE') {
                depth += 1;
              } else if (token.type == 'RPAREN' ||
                  token.type == 'RBRACKET' ||
                  token.type == 'RBRACE') {
                depth -= 1;
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
          BigInt.from(4106),
          PbtOptions(numTests: 100, seed: BigInt.from(4106)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const ops = [
              ('/.', 'REPLACEALL'),
              ('//.', 'REPLACEREPEATED'),
              ('/;', 'CONDITION'),
              ('/@', 'MAP'),
              ('@@', 'APPLY'),
              ('->', 'RULE'),
              (':>', 'RULEDELAYED'),
              (':=', 'SETDELAYED'),
              ('&&', 'AND'),
              ('||', 'OR'),
            ];
            final (text, type) = ops[index];
            final tokens = tryTokenizeWolfram(text);
            return tokens != null &&
                tokens.length == 2 && // operator + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('part expressions keep opener, index, closers', () {
      check(
        'part-shape',
        checkProperty<String>(
          BigInt.from(4107),
          PbtOptions(numTests: 200, seed: BigInt.from(4107)),
          (rng) => Gen.tuple2(smallInt(0, 25), wordGen(0, 4))
              .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
              .sample(rng),
          (name) {
            final tokens = tryTokenizeWolfram('x[[$name]]');
            if (tokens == null || tokens.length != 6) return false;
            return tokens[0].type == 'NAME' &&
                tokens[1].type == 'LDBRACKET' &&
                tokens[2].type == 'NAME' &&
                tokens[2].value == name &&
                tokens[3].type == 'RBRACKET' &&
                tokens[4].type == 'RBRACKET';
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(4108),
          PbtOptions(numTests: 200, seed: BigInt.from(4108)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenizeWolfram('1 (* $body *) + 2');
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
          BigInt.from(4109),
          PbtOptions(numTests: 400, seed: BigInt.from(4109)),
          (rng) => hostileWolfram(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeWolfram(input);
            try {
              final hard = tokenizeWolfram(input);
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
