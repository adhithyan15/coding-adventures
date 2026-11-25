/// Property tests for `coding_adventures_fsharp_lexer`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text
/// in every version, EOF shape, determinism, positions,
/// empty-version agreement, the valid corpus across all versions,
/// factory agreement, and unknown-version rejection — all with fixed
/// seeds.
library;

import 'package:coding_adventures_fsharp_lexer/fsharp_lexer.dart';
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

/// F#-dense hostile text: F# keywords and sigils plus whitespace and
/// out-of-alphabet noise.
Gen<String> hostileFsharp(int minLen, int maxLen) {
  const pieces = [
    'let',
    'match',
    'with',
    'type',
    'module',
    'open',
    'if',
    'then',
    'else',
    'fun',
    'function',
    'rec',
    'mutable',
    'for',
    'while',
    'do',
    'return',
    'yield',
    '|',
    '->',
    '<-',
    '=>',
    '=',
    '+',
    '-',
    '*',
    '/',
    '%',
    '<',
    '>',
    '<=',
    '>=',
    '<>',
    '&&',
    '||',
    '!',
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '"',
    "'",
    '`',
    '#',
    '.',
    ',',
    ';',
    ':',
    '::',
    '_',
    'x',
    'value',
    '0',
    '1',
    '3.14',
    '0xFF',
    '\n',
    '\r',
    ' ',
    '\t',
    'é',
    '🎉',
    r'$',
    '@',
    '?',
    '~',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

Gen<String> anyVersion() => smallInt(0, supportedFsharpVersions.length - 1)
    .map((i) => supportedFsharpVersions[i]);

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
          BigInt.from(5901),
          PbtOptions(numTests: 400, seed: BigInt.from(5901)),
          (rng) => hostileFsharp(0, 60).sample(rng),
          (input) {
            for (final version in supportedFsharpVersions) {
              try {
                tokenizeFsharp(input, version);
              } on LexerError {
                // Expected failure vocabulary.
              }
            }
            return true;
          },
        ),
      );
    });

    test('successful runs end with EOF in every version', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(5902),
          PbtOptions(numTests: 400, seed: BigInt.from(5902)),
          (rng) => hostileFsharp(0, 60).sample(rng),
          (input) {
            for (final version in supportedFsharpVersions) {
              try {
                final tokens = tokenizeFsharp(input, version);
                if (tokens.isEmpty || tokens.last.type != 'EOF') {
                  return false;
                }
              } on LexerError {
                // Failures carry no shape obligation.
              }
            }
            return true;
          },
        ),
      );
    });

    test('tokenize is deterministic per version', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(5903),
          PbtOptions(numTests: 200, seed: BigInt.from(5903)),
          (rng) => hostileFsharp(0, 60).sample(rng),
          (input) {
            for (final version in supportedFsharpVersions) {
              List<Token>? a;
              List<Token>? b;
              try {
                a = tokenizeFsharp(input, version);
              } on LexerError {
                a = null;
              }
              try {
                b = tokenizeFsharp(input, version);
              } on LexerError {
                b = null;
              }
              if (a == null || b == null) {
                if (!(a == null && b == null)) return false;
              } else if (!tokensEqual(a, b)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('tokens keep sane positions in every version', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(5904),
          PbtOptions(numTests: 400, seed: BigInt.from(5904)),
          (rng) => hostileFsharp(0, 60).sample(rng),
          (input) {
            for (final version in supportedFsharpVersions) {
              try {
                for (final t in tokenizeFsharp(input, version)) {
                  if (t.line < 1 || t.column < 1) return false;
                }
              } on LexerError {
                // No tokens, no positions to check.
              }
            }
            return true;
          },
        ),
      );
    });

    test('empty version always agrees with the default', () {
      check(
        'default-agreement',
        checkProperty<String>(
          BigInt.from(5905),
          PbtOptions(numTests: 400, seed: BigInt.from(5905)),
          (rng) => hostileFsharp(0, 60).sample(rng),
          (input) {
            List<Token>? a;
            List<Token>? b;
            try {
              a = tokenizeFsharp(input, '');
            } on LexerError {
              a = null;
            }
            try {
              b = tokenizeFsharp(input, defaultFsharpVersion);
            } on LexerError {
              b = null;
            }
            if (a == null || b == null) return a == null && b == null;
            return tokensEqual(a, b);
          },
        ),
      );
    });

    test('valid snippets tokenize in every version', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(5906),
          PbtOptions(numTests: 150, seed: BigInt.from(5906)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const snippets = [
              'let value = 1',
              'let f x = x + 1',
              'let pi = 3.14',
              'let name = "Alice"',
              "let c = 'z'",
              'let t = (1, 2)',
              'let l = [1; 2; 3]',
              'let b = true && false',
              'module M\nlet x = 0xFF',
              'let _ = 1 + 2 * 3',
            ];
            for (final version in supportedFsharpVersions) {
              try {
                tokenizeFsharp(snippets[index], version);
              } on LexerError {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('factory agrees with the one-shot in every version', () {
      check(
        'factory-agreement',
        checkProperty<String>(
          BigInt.from(5907),
          PbtOptions(numTests: 200, seed: BigInt.from(5907)),
          (rng) => hostileFsharp(0, 60).sample(rng),
          (input) {
            for (final version in supportedFsharpVersions) {
              List<Token>? a;
              List<Token>? b;
              try {
                a = createFsharpLexer(input, version).tokenize();
              } on LexerError {
                a = null;
              }
              try {
                b = tokenizeFsharp(input, version);
              } on LexerError {
                b = null;
              }
              if (a == null || b == null) {
                if (!(a == null && b == null)) return false;
              } else if (!tokensEqual(a, b)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('unknown versions always throw ArgumentError', () {
      check(
        'unknown-version',
        checkProperty<String>(
          BigInt.from(5908),
          PbtOptions(numTests: 200, seed: BigInt.from(5908)),
          (rng) => hostileFsharp(1, 12).sample(rng),
          (version) {
            if (supportedFsharpVersions.contains(version) || version.isEmpty) {
              return true;
            }
            try {
              tokenizeFsharp('let value = 1', version);
            } on ArgumentError catch (e) {
              return e.message.contains(version);
            } on LexerError {
              return false;
            }
            return false;
          },
        ),
      );
    });

    test('sampled versions resolve to themselves', () {
      check(
        'version-identity',
        checkProperty<String>(
          BigInt.from(5909),
          PbtOptions(numTests: 150, seed: BigInt.from(5909)),
          anyVersion().sample,
          (version) => resolveFsharpVersion(version) == version,
        ),
      );
    });
  });
}
