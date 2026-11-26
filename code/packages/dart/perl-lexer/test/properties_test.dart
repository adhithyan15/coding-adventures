/// Property tests for `coding_adventures_perl_lexer`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// EOF shape, determinism, positions, the exact source-cap boundary,
/// valid-snippet lexing, release-table coverage, unknown-release
/// nulls, and class determinism — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_perl_lexer/perl_lexer.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Perl-dense hostile text: Perl keywords and sigils plus whitespace
/// and out-of-alphabet noise.
Gen<String> hostilePerl(int minLen, int maxLen) {
  const pieces = [
    'print',
    'if',
    'else',
    'while',
    'for',
    'foreach',
    'my',
    'our',
    'sub',
    'return',
    'use',
    'strict',
    'r',
    r'$',
    '@',
    '%',
    '(',
    ')',
    '{',
    '}',
    '[',
    ']',
    '=',
    '==',
    '+',
    '-',
    '--',
    '*',
    '/',
    ';',
    ',',
    '"',
    "'",
    '#',
    '0',
    '7',
    '42',
    '_',
    'x',
    '\n',
    '\r',
    ' ',
    '\t',
    'é',
    '🎉',
    '?',
    ':',
    '.',
    '\\',
    '!',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

bool tokensEqual(List<Token> a, List<Token> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i].type != b[i].type ||
        a[i].value != b[i].value ||
        a[i].line != b[i].line ||
        a[i].column != b[i].column) {
      return false;
    }
  }
  return true;
}

void main() {
  group('properties', () {
    test('tokenize fails closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(6101),
          PbtOptions(numTests: 400, seed: BigInt.from(6101)),
          (rng) => hostilePerl(0, 60).sample(rng),
          (input) {
            try {
              tokenizePerl(input);
            } on LexerError {
              // Expected failure vocabulary (inputs are far under
              // the 64KiB cap, so no ArgumentError is possible).
            }
            return true;
          },
        ),
      );
    });

    test('successful runs end with EOF', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(6102),
          PbtOptions(numTests: 400, seed: BigInt.from(6102)),
          (rng) => hostilePerl(0, 60).sample(rng),
          (input) {
            try {
              final tokens = tokenizePerl(input);
              return tokens.isNotEmpty && tokens.last.type == 'EOF';
            } on LexerError {
              return true;
            }
          },
        ),
      );
    });

    test('tokenize is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(6103),
          PbtOptions(numTests: 200, seed: BigInt.from(6103)),
          (rng) => hostilePerl(0, 60).sample(rng),
          (input) {
            List<Token>? a;
            List<Token>? b;
            try {
              a = tokenizePerl(input);
            } on LexerError {
              a = null;
            }
            try {
              b = tokenizePerl(input);
            } on LexerError {
              b = null;
            }
            if (a == null || b == null) return a == null && b == null;
            return tokensEqual(a, b);
          },
        ),
      );
    });

    test('tokens keep sane positions', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(6104),
          PbtOptions(numTests: 400, seed: BigInt.from(6104)),
          (rng) => hostilePerl(0, 60).sample(rng),
          (input) {
            try {
              for (final t in tokenizePerl(input)) {
                if (t.line < 1 || t.column < 1) return false;
              }
            } on LexerError {
              // No tokens, no positions to check.
            }
            return true;
          },
        ),
      );
    });

    test('source cap boundary is exactly 65536 bytes', () {
      check(
        'cap-boundary',
        checkProperty<int>(
          BigInt.from(6105),
          PbtOptions(numTests: 60, seed: BigInt.from(6105)),
          (rng) => smallInt(65530, 65542).sample(rng),
          (length) {
            final input = 'x' * length;
            try {
              tokenizePerl(input);
            } on ArgumentError {
              return length > 65536;
            } on LexerError {
              return length <= 65536;
            }
            return length <= 65536;
          },
        ),
      );
    });

    test('valid snippets always lex', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(6106),
          PbtOptions(numTests: 100, seed: BigInt.from(6106)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const snippets = [
              'print(7 - 2 * 3);\n',
              'print(1--2);',
              'x-1',
              'print(0);',
              'abc_123',
              '1+2*3-4',
              '(a)',
              'print;',
              '',
              '   \n\t\n',
            ];
            try {
              tokenizePerl(snippets[index]);
              return true;
            } on LexerError {
              return false;
            }
          },
        ),
      );
    });

    test('every release resolves a table and a class', () {
      check(
        'release-coverage',
        checkProperty<int>(
          BigInt.from(6107),
          PbtOptions(numTests: 236, seed: BigInt.from(6107)),
          (rng) => smallInt(0, supportedPerlReleases.length - 1).sample(rng),
          (index) {
            final release = supportedPerlReleases[index];
            final table = perlReleaseTokenGrammar(release);
            final cls = perlGrammarClass(release);
            return table != null && cls != null && cls.startsWith('g_');
          },
        ),
      );
    });

    test('unknown release ids resolve to null', () {
      check(
        'unknown-release',
        checkProperty<String>(
          BigInt.from(6108),
          PbtOptions(numTests: 200, seed: BigInt.from(6108)),
          (rng) => hostilePerl(1, 12).sample(rng),
          (id) {
            if (supportedPerlReleases.contains(id)) return true;
            return perlReleaseTokenGrammar(id) == null &&
                perlGrammarClass(id) == null;
          },
        ),
      );
    });

    test('release lookups are deterministic', () {
      check(
        'lookup-determinism',
        checkProperty<int>(
          BigInt.from(6109),
          PbtOptions(numTests: 236, seed: BigInt.from(6109)),
          (rng) => smallInt(0, supportedPerlReleases.length - 1).sample(rng),
          (index) {
            final release = supportedPerlReleases[index];
            return identical(perlReleaseTokenGrammar(release),
                    perlReleaseTokenGrammar(release)) &&
                perlGrammarClass(release) == perlGrammarClass(release);
          },
        ),
      );
    });
  });
}
