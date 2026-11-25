/// Property tests for `coding_adventures_macronib_lexer`, using
/// the in-lane `pbt` package: the engine contract over hostile text,
/// promotion completeness (no `KEYWORD` survives the one-shots),
/// directive atomicity, agreement with the `nib` lane on shared
/// input, comment invisibility, and API agreement — all with fixed
/// seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macronib_lexer/macronib_lexer.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
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

/// MacroNib-dense hostile text: directives, strings, every Nib sigil,
/// comment openers, whitespace, and out-of-alphabet noise.
Gen<String> hostileMacronib(int minLen, int maxLen) {
  const pieces = [
    '.include',
    '.ifdef',
    '.endif',
    '.else',
    '.set',
    '"',
    '"s"',
    '..',
    '->',
    '==',
    '!=',
    '<=',
    '>=',
    '&&',
    '||',
    '<<',
    '>>',
    '+%',
    '+?',
    '+',
    '-',
    '*',
    '/',
    '%',
    '&',
    '|',
    '^',
    '~',
    '!',
    '<',
    '>',
    '=',
    '{',
    '}',
    '(',
    ')',
    ':',
    ';',
    ',',
    '.',
    '0x',
    'fn',
    'let',
    '\n',
    '\r',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '_',
    '@',
    '#',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Shared-surface soup: no dots, no quotes — input on which the
/// MacroNib-only rules can never fire, so both lanes must agree
/// exactly (the grammars are otherwise identical, proven by the
/// `every non-directive rule matches nib in order` unit test).
Gen<String> sharedSoup(int minLen, int maxLen) {
  const pieces = [
    '->',
    '==',
    '!=',
    '<=',
    '>=',
    '&&',
    '||',
    '<<',
    '>>',
    '+%',
    '+?',
    '+',
    '-',
    '*',
    '/',
    '%',
    '&',
    '|',
    '^',
    '~',
    '!',
    '<',
    '>',
    '=',
    '{',
    '}',
    '(',
    ')',
    ':',
    ';',
    ',',
    '0x1F',
    'fn',
    'let',
    'static',
    'if',
    'else',
    'true',
    '\n',
    ' ',
    '\t',
    'a',
    'Z',
    '0',
    '9',
    '_',
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
          BigInt.from(4301),
          PbtOptions(numTests: 400, seed: BigInt.from(4301)),
          (rng) => hostileMacronib(0, 60).sample(rng),
          (input) {
            try {
              tokenizeMacronib(input);
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
          BigInt.from(4302),
          PbtOptions(numTests: 400, seed: BigInt.from(4302)),
          (rng) => hostileMacronib(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMacronib(input);
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
          BigInt.from(4303),
          PbtOptions(numTests: 200, seed: BigInt.from(4303)),
          (rng) => hostileMacronib(0, 60).sample(rng),
          (input) {
            final a = tryTokenizeMacronib(input);
            final b = tryTokenizeMacronib(input);
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
          BigInt.from(4304),
          PbtOptions(numTests: 400, seed: BigInt.from(4304)),
          (rng) => hostileMacronib(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMacronib(input);
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

    test('no KEYWORD type survives the one-shots', () {
      check(
        'promotion-complete',
        checkProperty<String>(
          BigInt.from(4305),
          PbtOptions(numTests: 400, seed: BigInt.from(4305)),
          (rng) => hostileMacronib(0, 60).sample(rng),
          (input) {
            final tokens = tryTokenizeMacronib(input);
            if (tokens == null) return true;
            return countType(tokens, 'KEYWORD') == 0;
          },
        ),
      );
    });

    test('directives and strings never split', () {
      check(
        'directive-atomic',
        checkProperty<int>(
          BigInt.from(4306),
          PbtOptions(numTests: 100, seed: BigInt.from(4306)),
          (rng) => smallInt(0, 5).sample(rng),
          (index) {
            const only = [
              ('.include', 'DOT_INCLUDE'),
              ('.ifdef', 'DOT_IFDEF'),
              ('.endif', 'DOT_ENDIF'),
              ('.else', 'DOT_ELSE'),
              ('.set', 'DOT_SET'),
              ('"abc"', 'STR_LIT'),
            ];
            final (text, type) = only[index];
            final tokens = tryTokenizeMacronib(text);
            return tokens != null &&
                tokens.length == 2 && // token + EOF
                tokens[0].type == type &&
                tokens[0].value == text;
          },
        ),
      );
    });

    test('nib lane agrees on the shared surface', () {
      check(
        'nib-agreement',
        checkProperty<String>(
          BigInt.from(4307),
          PbtOptions(numTests: 400, seed: BigInt.from(4307)),
          (rng) => sharedSoup(0, 60).sample(rng),
          (input) {
            final actual = tryTokenizeMacronib(input);
            List<Token> reference;
            try {
              reference = tokenizeNib(input);
            } on LexerError {
              return actual == null;
            }
            if (actual == null) return false;
            if (actual.length != reference.length) return false;
            for (var i = 0; i < actual.length; i++) {
              if (actual[i] != reference[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('comments never surface as tokens', () {
      check(
        'comment-invisible',
        checkProperty<String>(
          BigInt.from(4308),
          PbtOptions(numTests: 200, seed: BigInt.from(4308)),
          (rng) => wordGen(0, 10).sample(rng),
          (body) {
            final tokens = tryTokenizeMacronib('1 // $body\n+ 2');
            if (tokens == null) return false;
            final sig = tokens
                .where((t) => t.type != 'EOF')
                .map((t) => '${t.type}=${t.value}')
                .join('|');
            return sig == 'INT_LIT=1|PLUS=+|INT_LIT=2';
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(4309),
          PbtOptions(numTests: 400, seed: BigInt.from(4309)),
          (rng) => hostileMacronib(0, 60).sample(rng),
          (input) {
            final soft = tryTokenizeMacronib(input);
            try {
              final hard = tokenizeMacronib(input);
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
