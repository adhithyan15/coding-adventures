/// Property tests for `coding_adventures_c_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile text, the
/// keyword predicate over the grammar's own tables, multi-char
/// operator priority, and comment invisibility — all with fixed seeds.
library;

import 'package:coding_adventures_c_lexer/c_lexer.dart';
import 'package:coding_adventures_c_lexer/src/_grammar.dart';
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

/// Markup-dense hostile text: every C sigil plus whitespace, quotes,
/// and a little Unicode.
Gen<String> hostileC(int minLen, int maxLen) {
  const pieces = [
    '=',
    '==',
    '+',
    '-',
    '*',
    '/',
    '%',
    '<',
    '>',
    '!',
    '&',
    '|',
    '^',
    '~',
    '?',
    ':',
    ';',
    ',',
    '.',
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '#',
    '##',
    "'",
    '"',
    '\\',
    ' ',
    '\n',
    '\r',
    '\t',
    'a',
    'Z',
    '0',
    '_',
    'int',
    '0x',
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
          BigInt.from(2601),
          PbtOptions(numTests: 400, seed: BigInt.from(2601)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            try {
              tokenizeC(input);
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
          BigInt.from(2602),
          PbtOptions(numTests: 400, seed: BigInt.from(2602)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeC(input);
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
          BigInt.from(2603),
          PbtOptions(numTests: 200, seed: BigInt.from(2603)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeC(input);
            final b = tryTokenizeC(input);
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
          BigInt.from(2604),
          PbtOptions(numTests: 400, seed: BigInt.from(2604)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeC(input);
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

    test('keyword predicate matches the grammar tables', () {
      check(
        'keyword-agreement',
        checkProperty<int>(
          BigInt.from(2605),
          PbtOptions(numTests: 200, seed: BigInt.from(2605)),
          (rng) => smallInt(
                  0,
                  tokenGrammar.keywords.length +
                      tokenGrammar.reservedKeywords.length -
                      1)
              .sample(rng),
          (index) {
            final table = [
              ...tokenGrammar.keywords,
              ...tokenGrammar.reservedKeywords
            ];
            final word = table[index];
            if (!isCKeyword(word)) return false;
            // Keywords lex as KEYWORD; reserved words either promote
            // or fail closed — never as NAME.
            final tokens = tryTokenizeC(word);
            if (tokens == null) return true;
            return tokens.length == 2 && tokens[0].type != 'NAME';
          },
        ),
      );
    });

    test('multi-char operators never split', () {
      check(
        'operator-atomic',
        checkProperty<int>(
          BigInt.from(2606),
          PbtOptions(numTests: 100, seed: BigInt.from(2606)),
          (rng) => smallInt(0, 8).sample(rng),
          (index) {
            // Note: no `->` here — this cut has no ARROW token, so it
            // lexes as MINUS + GT (pinned in unit tests).
            const ops = [
              '==',
              '<<',
              '>>',
              '&&',
              '||',
              '##',
              '<=',
              '>=',
              '!=',
            ];
            final op = ops[index];
            final tokens = tryTokenizeC(op);
            return tokens != null &&
                tokens.length == 2 && // operator + EOF
                tokens[0].value == op;
          },
        ),
      );
    });

    test('integers lex whole with values intact', () {
      check(
        'integer-whole',
        checkProperty<String>(
          BigInt.from(2607),
          PbtOptions(numTests: 200, seed: BigInt.from(2607)),
          (rng) => Gen.oneOf([
            smallInt(0, 99999).map((i) => i.toString()),
            smallInt(0, 65535)
                .map((i) => '0x${i.toRadixString(16).toUpperCase()}'),
          ]).sample(rng),
          (literal) {
            final tokens = tryTokenizeC(literal);
            return tokens != null &&
                tokens.length == 2 &&
                tokens[0].type == 'INT_LIT' &&
                tokens[0].value == literal;
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(2608),
          PbtOptions(numTests: 200, seed: BigInt.from(2608)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            String signature(List<Token> tokens) => tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            final bare = tryTokenizeC('int x;')!;
            final line = tryTokenizeC('int x; //$body\n');
            final block = tryTokenizeC('int /*$body*/ x;');
            return line != null &&
                block != null &&
                signature(line) == signature(bare) &&
                signature(block) == signature(bare);
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(2609),
          PbtOptions(numTests: 400, seed: BigInt.from(2609)),
          (rng) => hostileC(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeC(input);
            try {
              final hard = tokenizeC(input);
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
