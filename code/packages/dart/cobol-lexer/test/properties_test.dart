/// Property tests for `coding_adventures_cobol_lexer`, using the
/// in-lane `pbt` package: the engine contract over hostile carded
/// text, the column-strip rules (comment invisibility, continuation
/// splicing) as direct hook properties, picture atomicity, keyword
/// case behavior, and API agreement — all with fixed seeds.
library;

import 'package:coding_adventures_cobol_lexer/cobol_lexer.dart';
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

/// Hostile carded text: sequence digits, every indicator, code words,
/// comment bodies, CRLF joints, and out-of-alphabet noise.
Gen<String> hostileCobol(int minLen, int maxLen) {
  const pieces = [
    '000100',
    '000200',
    ' ',
    '*',
    '/',
    '-',
    'MOVE',
    'PIC',
    'PICTURE',
    'X(20)',
    '9(3)',
    'VALUE',
    'ZERO',
    '.',
    '"',
    "'",
    ',',
    ';',
    '(',
    ')',
    '\n',
    '\r',
    '\r\n',
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
          BigInt.from(4701),
          PbtOptions(numTests: 400, seed: BigInt.from(4701)),
          (rng) => hostileCobol(0, 60).sample(rng),
          (input) {
            try {
              tokenizeCobol(input);
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
          BigInt.from(4702),
          PbtOptions(numTests: 400, seed: BigInt.from(4702)),
          (rng) => hostileCobol(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeCobol(input);
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
          BigInt.from(4703),
          PbtOptions(numTests: 200, seed: BigInt.from(4703)),
          (rng) => hostileCobol(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeCobol(input);
            final b = tryTokenizeCobol(input);
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
          BigInt.from(4704),
          PbtOptions(numTests: 400, seed: BigInt.from(4704)),
          (rng) => hostileCobol(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeCobol(input);
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

    test('comment lines vanish without a trace', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(4705),
          PbtOptions(numTests: 200, seed: BigInt.from(4705)),
          (rng) => Gen.tuple2(wordGen(0, 10), smallInt(0, 1))
              .map((t) => '${t.$1}|${t.$2}')
              .sample(rng),
          (key) {
            final parts = key.split('|');
            final body = parts[0];
            final indicator = parts[1] == '0' ? '*' : '/';
            // A comment line between two code lines leaves the exact
            // same (type, value) stream as no line at all (positions
            // necessarily shift past the vanished line).
            final withComment = tryTokenizeCobol(
                '000100 MOVE X.\n000200$indicator$body\n000300 DISPLAY Y.\n');
            final without =
                tryTokenizeCobol('000100 MOVE X.\n000300 DISPLAY Y.\n');
            if (withComment == null || without == null) return false;
            if (withComment.length != without.length) return false;
            for (var i = 0; i < withComment.length; i++) {
              if (withComment[i].type != without[i].type ||
                  withComment[i].value != without[i].value) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('continuation splices at any mid-word split point', () {
      check(
        'continuation-splice',
        checkProperty<int>(
          BigInt.from(4706),
          PbtOptions(numTests: 100, seed: BigInt.from(4706)),
          // Mid-word splits only: at split 0 the joint space itself is
          // (by design, like the reference) trimmed away.
          (rng) => smallInt(1, 4).sample(rng),
          (split) {
            const word = 'FIRST';
            final first = word.substring(0, split);
            final second = word.substring(split);
            return stripCobolColumns(
                    '000100 MOVE $first\n000200-$second TO X.') ==
                'MOVE FIRST TO X.';
          },
        ),
      );
    });

    test('picture clauses never split', () {
      check(
        'picture-atomic',
        checkProperty<int>(
          BigInt.from(4707),
          PbtOptions(numTests: 100, seed: BigInt.from(4707)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const clauses = [
              ('PICTURE', 'X(20)'),
              ('PIC', '9(3)'),
              ('PIC', '9(3)V99'),
              ('PIC', 'S9(4)V99'),
              ('PICTURE', 'XXX'),
              ('PIC', '999'),
            ];
            final (keyword, picture) = clauses[index];
            final tokens = tryTokenizeCobol('000000 $keyword $picture');
            return tokens != null &&
                tokens.length == 3 && // keyword + picture + EOF
                tokens[0].type == 'KEYWORD' &&
                tokens[0].value == keyword &&
                tokens[1].type == 'PIC_STRING' &&
                tokens[1].value == picture;
          },
        ),
      );
    });

    test('uppercase promotes, lowercase stays a name', () {
      check(
        'keyword-case',
        checkProperty<int>(
          BigInt.from(4708),
          PbtOptions(numTests: 100, seed: BigInt.from(4708)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            // Both engines match patterns case-insensitively but look
            // keywords up verbatim (the grammar's `case_insensitive`
            // magic is off in both lanes — a doc line resets it), so
            // only exact-case reserved words promote, in both lanes.
            const words = [
              ('MOVE', 'KEYWORD'),
              ('PIC', 'KEYWORD'),
              ('VALUE', 'KEYWORD'),
              ('move', 'NAME'),
              ('pic', 'NAME'),
              ('value', 'NAME'),
            ];
            final (text, type) = words[index];
            final tokens = tryTokenizeCobol('000000 $text');
            return tokens != null &&
                tokens.length == 2 && // word + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4709),
          PbtOptions(numTests: 400, seed: BigInt.from(4709)),
          (rng) => hostileCobol(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeCobol(input);
            try {
              final hard = tokenizeCobol(input);
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
