/// Property tests for `coding_adventures_dartmouth_basic_lexer`,
/// using the in-lane `pbt` package: the engine contract over hostile
/// text, the two post-hook rules (line-number relabeling, REM
/// suppression) as stream invariants, keyword case behavior,
/// operator atomicity, and factory agreement — all with fixed seeds.
///
/// The reference offers no `Result` twin (callers use factory +
/// `tokenize()`), so this lane has no nullable API either; [soft]
/// is a test-local helper, not a product API.
library;

import 'package:coding_adventures_dartmouth_basic_lexer/dartmouth_basic_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

/// Test-local nullable twin: `null` on [LexerError].
List<Token>? soft(String input) {
  try {
    return tokenizeDartmouthBasic(input);
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

/// BASIC-dense hostile text: line numbers, keywords, names, numbers,
/// strings, REM, operators, whitespace, and out-of-alphabet noise.
Gen<String> hostileBasic(int minLen, int maxLen) {
  const pieces = [
    '10',
    '20',
    '0',
    '9999',
    'LET',
    'PRINT',
    'GOTO',
    'REM',
    'IF',
    'THEN',
    'FOR',
    'NEXT',
    'END',
    'SIN',
    'FNA',
    'X',
    'A1',
    'A\$',
    '42',
    '3.14',
    '.5',
    '1E10',
    '"',
    '"HI"',
    '=',
    '<=',
    '>=',
    '<>',
    '<',
    '>',
    '+',
    '-',
    '*',
    '/',
    '^',
    '(',
    ')',
    ',',
    ';',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'z',
    '@',
    '#',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Random line-numbered programs from fixed statement templates.
Gen<String> basicProgram(int minLines, int maxLines) {
  const lines = [
    '10 LET X = 1',
    '20 GOTO 10',
    '30 REM A COMMENT 123',
    '40 PRINT X, Y; Z',
    '50 IF X <= 2 THEN 10',
    '60 FOR I = 1 TO 9 STEP 2',
    '70 NEXT I',
    '80 END',
    '90 LET S = "HI"',
  ];
  return Gen.list(minLines, maxLines, smallInt(0, lines.length - 1))
      .map((idx) => '${idx.map((i) => lines[i]).join('\n')}\n');
}

int countType(List<Token> tokens, String type) =>
    tokens.where((t) => t.type == type).length;

/// Whether [tokens] has the reference's exact EOF shape: exactly one
/// trailing EOF — unless an unclosed REM swallowed it, i.e. a REM
/// keyword stands after the last NEWLINE (proven against the Rust
/// reference, which drops EOF there too).
bool eofShapeOk(List<Token> tokens) {
  if (tokens.isNotEmpty &&
      tokens.last.type == 'EOF' &&
      countType(tokens, 'EOF') == 1) {
    return true;
  }
  if (tokens.any((t) => t.type == 'EOF')) return false;
  var lastRem = -1;
  var lastNl = -1;
  for (var i = 0; i < tokens.length; i++) {
    if (tokens[i].type == 'KEYWORD' && tokens[i].value == 'REM') {
      lastRem = i;
    }
    if (tokens[i].type == 'NEWLINE') lastNl = i;
  }
  return lastRem != -1 && lastRem > lastNl;
}

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(4901),
          PbtOptions(numTests: 400, seed: BigInt.from(4901)),
          (rng) => hostileBasic(0, 60).sample(rng),
          (input) {
            try {
              tokenizeDartmouthBasic(input);
            } on LexerError {
              return true;
            }
            return true;
          },
        ),
      );
    });

    test('successful runs keep the reference eof shape', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(4902),
          PbtOptions(numTests: 400, seed: BigInt.from(4902)),
          (rng) => hostileBasic(0, 60).sample(rng),
          (input) {
            final tokens = soft(input);
            if (tokens == null) return true;
            return eofShapeOk(tokens);
          },
        ),
      );
    });

    test('tokenize is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(4903),
          PbtOptions(numTests: 200, seed: BigInt.from(4903)),
          (rng) => hostileBasic(0, 60).sample(rng),
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
          BigInt.from(4904),
          PbtOptions(numTests: 400, seed: BigInt.from(4904)),
          (rng) => hostileBasic(0, 60).sample(rng),
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

    test('line numbers lead lines, values trail', () {
      check(
        'relabel-rule',
        checkProperty<String>(
          BigInt.from(4905),
          PbtOptions(numTests: 300, seed: BigInt.from(4905)),
          (rng) => basicProgram(1, 6).sample(rng),
          (input) {
            final tokens = soft(input);
            if (tokens == null) return false;
            // Replay the hook's own rule independently: the first
            // NUMBER on each line must read LINE_NUM, and every
            // LINE_NUM must be first on its line.
            var atLineStart = true;
            for (final t in tokens) {
              if (t.type == 'EOF') break;
              if (atLineStart) {
                if (t.type == 'NUMBER') return false;
              } else {
                if (t.type == 'LINE_NUM') return false;
              }
              atLineStart = t.type == 'NEWLINE';
            }
            return true;
          },
        ),
      );
    });

    test('rem swallows to end of line', () {
      check(
        'rem-suppression',
        checkProperty<String>(
          BigInt.from(4906),
          PbtOptions(numTests: 300, seed: BigInt.from(4906)),
          (rng) => basicProgram(1, 6).sample(rng),
          (input) {
            final tokens = soft(input);
            if (tokens == null) return false;
            // After a REM keyword, the only token allowed on its line
            // is the terminating NEWLINE.
            for (var i = 0; i < tokens.length; i++) {
              final t = tokens[i];
              if (t.type == 'KEYWORD' && t.value == 'REM') {
                for (var j = i + 1; j < tokens.length; j++) {
                  final u = tokens[j];
                  if (u.line != t.line) break;
                  if (u.type != 'NEWLINE') return false;
                }
              }
            }
            return true;
          },
        ),
      );
    });

    test('lowercase promotes uppercased, names keep case', () {
      check(
        'keyword-case',
        checkProperty<int>(
          BigInt.from(4907),
          PbtOptions(numTests: 100, seed: BigInt.from(4907)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const words = [
              ('print', 'KEYWORD', 'PRINT'),
              ('let', 'KEYWORD', 'LET'),
              ('goto', 'KEYWORD', 'GOTO'),
              ('x', 'NAME', 'x'),
              ('X', 'NAME', 'X'),
              ('a1', 'NAME', 'a1'),
            ];
            final (text, type, value) = words[index];
            final tokens = soft('10 $text\n');
            if (tokens == null || tokens.length < 2) return false;
            return tokens[1].type == type && tokens[1].value == value;
          },
        ),
      );
    });

    test('two-char operators never split', () {
      check(
        'operator-atomic',
        checkProperty<int>(
          BigInt.from(4908),
          PbtOptions(numTests: 100, seed: BigInt.from(4908)),
          (rng) => smallInt(0, 2).sample(rng),
          (index) {
            const ops = [
              ('<=', 'LE'),
              ('>=', 'GE'),
              ('<>', 'NE'),
            ];
            final (text, type) = ops[index];
            final tokens = soft('10 IF X $text Y THEN 50\n');
            if (tokens == null) return false;
            return tokens.any((t) => t.type == type && t.value == text) &&
                !tokens.any((t) =>
                    (t.type == 'LT' || t.type == 'GT' || t.type == 'EQ') &&
                    t.value.length == 1 &&
                    text.contains(t.value));
          },
        ),
      );
    });

    test('factory agrees with the one-shot', () {
      check(
        'factory-agreement',
        checkProperty<String>(
          BigInt.from(4909),
          PbtOptions(numTests: 400, seed: BigInt.from(4909)),
          (rng) => hostileBasic(0, 60).sample(rng),
          (input) {
            final oneShot = soft(input);
            List<Token> viaFactory;
            try {
              viaFactory = createDartmouthBasicLexer(input).tokenize();
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
