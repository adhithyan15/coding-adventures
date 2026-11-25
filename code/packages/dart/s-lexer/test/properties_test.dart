/// Property tests for `coding_adventures_s_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// hook's depth rule as a mirror model, underscore splitting, and API
/// agreement — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_s_lexer/s_lexer.dart';
import 'package:coding_adventures_s_lexer/src/_grammar.dart';
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

/// Markup-dense hostile text: every S sigil plus whitespace and a
/// little out-of-alphabet noise.
Gen<String> hostileS(int minLen, int maxLen) {
  const pieces = [
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '<-',
    '->',
    '=',
    '==',
    '+',
    '-',
    '*',
    '/',
    '^',
    ':',
    ',',
    '_',
    '.',
    '#',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    'if',
    'TRUE',
    '@',
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
          BigInt.from(3901),
          PbtOptions(numTests: 400, seed: BigInt.from(3901)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            try {
              tokenizeS(input);
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
          BigInt.from(3902),
          PbtOptions(numTests: 400, seed: BigInt.from(3902)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeS(input);
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
          BigInt.from(3903),
          PbtOptions(numTests: 200, seed: BigInt.from(3903)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeS(input);
            final b = tryTokenizeS(input);
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
          BigInt.from(3904),
          PbtOptions(numTests: 400, seed: BigInt.from(3904)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeS(input);
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
          BigInt.from(3905),
          PbtOptions(numTests: 300, seed: BigInt.from(3905)),
          (rng) => Gen.list(1, 20, smallInt(0, 6))
              .map((idx) => idx
                  .map((i) => ['(', ')', '[', ']', '{', '}', '\n'][i])
                  .join())
              .sample(rng),
          (input) {
            final hooked = tryTokenizeS(input);
            if (hooked == null) return false;
            List<Token> raw;
            try {
              raw = GrammarLexer(input, tokenGrammar).tokenize();
            } on LexerError {
              return false;
            }
            final predicted = <Token>[];
            var depth = 0;
            for (final token in raw) {
              if (!(token.type == 'NEWLINE' && depth > 0)) {
                predicted.add(token);
              }
              if (token.type == 'LPAREN' || token.type == 'LBRACKET') {
                depth += 1;
              } else if (token.type == 'RPAREN' || token.type == 'RBRACKET') {
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

    test('underscores always split names', () {
      check(
        'underscore-split',
        checkProperty<(String, String)>(
          BigInt.from(3906),
          PbtOptions(numTests: 200, seed: BigInt.from(3906)),
          (rng) {
            String ident() => Gen.tuple2(smallInt(0, 25), wordGen(0, 4))
                .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
                .sample(rng);
            return (ident(), ident());
          },
          (input) {
            final (a, b) = input;
            final tokens = tryTokenizeS('${a}_$b');
            if (tokens == null || tokens.length != 4) return false;
            return tokens[0].type == 'NAME' &&
                tokens[0].value == a &&
                tokens[1].type == 'UNDERSCORE' &&
                tokens[2].type == 'NAME' &&
                tokens[2].value == b;
          },
        ),
      );
    });

    test('dotted names lex whole', () {
      check(
        'dotted-whole',
        checkProperty<(String, String)>(
          BigInt.from(3907),
          PbtOptions(numTests: 200, seed: BigInt.from(3907)),
          (rng) {
            String ident() => Gen.tuple2(smallInt(0, 25), wordGen(0, 4))
                .map((t) => '${'abcdefghijklmnopqrstuvwxyz'[t.$1]}${t.$2}')
                .sample(rng);
            return (ident(), ident());
          },
          (input) {
            final (a, b) = input;
            final tokens = tryTokenizeS('$a.$b');
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'NAME' &&
                tokens[0].value == '$a.$b';
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(3908),
          PbtOptions(numTests: 200, seed: BigInt.from(3908)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenizeS('x <- 1 #$body\ny <- 2');
            if (tokens == null) return false;
            final sig = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return sig ==
                'NAME=x|ASSIGN=<-|NUMBER=1|NEWLINE=\\n|'
                    'NAME=y|ASSIGN=<-|NUMBER=2';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(3909),
          PbtOptions(numTests: 400, seed: BigInt.from(3909)),
          (rng) => hostileS(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeS(input);
            try {
              final hard = tokenizeS(input);
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
