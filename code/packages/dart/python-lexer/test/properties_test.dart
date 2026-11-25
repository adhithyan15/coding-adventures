/// Property tests for `coding_adventures_python_lexer`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// EOF shape, determinism, positions, default agreement, the valid
/// corpus, release-table loading, unknown-version rejection, table
/// separation, and class stability — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_python_lexer/python_lexer.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Python-dense hostile text: Python keywords and sigils plus
/// whitespace (including indentation) and out-of-alphabet noise.
Gen<String> hostilePython(int minLen, int maxLen) {
  const pieces = [
    'def',
    'return',
    'if',
    'else',
    'elif',
    'for',
    'while',
    'import',
    'from',
    'as',
    'with',
    'class',
    'lambda',
    'pass',
    'match',
    'case',
    'True',
    'False',
    'None',
    'and',
    'or',
    'not',
    'is',
    'in',
    'print',
    'x',
    'value',
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '=',
    '==',
    '!=',
    '<>',
    '+',
    '-',
    '*',
    '/',
    '//',
    '%',
    '**',
    '<',
    '>',
    '<=',
    '>=',
    ':',
    ',',
    '.',
    '@',
    '@=',
    ':=',
    '#',
    '"',
    "'",
    '_',
    '0',
    '1',
    '42',
    '3.14',
    '0xFF',
    '0o17',
    '0b101',
    '1L',
    '\n',
    '\r',
    ' ',
    '    ',
    '\t',
    'é',
    '🎉',
    r'$',
    '?',
    ';',
    '\\',
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
          BigInt.from(6001),
          PbtOptions(numTests: 400, seed: BigInt.from(6001)),
          (rng) => hostilePython(0, 60).sample(rng),
          (input) {
            for (final version in supportedPythonVersions) {
              try {
                tokenizePython(input, version);
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
          BigInt.from(6002),
          PbtOptions(numTests: 400, seed: BigInt.from(6002)),
          (rng) => hostilePython(0, 60).sample(rng),
          (input) {
            for (final version in supportedPythonVersions) {
              try {
                final tokens = tokenizePython(input, version);
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
          BigInt.from(6003),
          PbtOptions(numTests: 200, seed: BigInt.from(6003)),
          (rng) => hostilePython(0, 60).sample(rng),
          (input) {
            for (final version in supportedPythonVersions) {
              List<Token>? a;
              List<Token>? b;
              try {
                a = tokenizePython(input, version);
              } on LexerError {
                a = null;
              }
              try {
                b = tokenizePython(input, version);
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
          BigInt.from(6004),
          PbtOptions(numTests: 400, seed: BigInt.from(6004)),
          (rng) => hostilePython(0, 60).sample(rng),
          (input) {
            for (final version in supportedPythonVersions) {
              try {
                for (final t in tokenizePython(input, version)) {
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
          BigInt.from(6005),
          PbtOptions(numTests: 400, seed: BigInt.from(6005)),
          (rng) => hostilePython(0, 60).sample(rng),
          (input) {
            List<Token>? a;
            List<Token>? b;
            try {
              a = tokenizePython(input, '');
            } on LexerError {
              a = null;
            }
            try {
              b = tokenizePython(input, defaultPythonVersion);
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
          BigInt.from(6006),
          PbtOptions(numTests: 150, seed: BigInt.from(6006)),
          (rng) => smallInt(0, 9).sample(rng),
          (index) {
            const snippets = [
              'x = 1\n',
              'def f():\n    return 1\n',
              'if x:\n    y = 2\n',
              'for i in [1, 2]:\n    print(i)\n',
              'import os\nfrom sys import argv\n',
              'class A:\n    def m(self):\n        pass\n',
              'x = 1 + 2 * 3\n',
              's = "hello"\n',
              '# just a comment\n',
              'while True:\n    break\n',
            ];
            for (final version in supportedPythonVersions) {
              try {
                tokenizePython(snippets[index], version);
              } on LexerError {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('release tables lex the probe in every release', () {
      check(
        'release-loading',
        checkProperty<int>(
          BigInt.from(6007),
          PbtOptions(numTests: 150, seed: BigInt.from(6007)),
          (rng) => smallInt(0, supportedPythonReleases.length - 1).sample(rng),
          (index) {
            final release = supportedPythonReleases[index];
            final grammar = tokenGrammarForRelease(release);
            if (grammar == null) return false;
            try {
              final tokens = grammarTokenize('print(1 // 2)\n', grammar);
              return tokens.isNotEmpty &&
                  tokens[0].value == 'print' &&
                  tokens.last.type == 'EOF';
            } on LexerError {
              return false;
            }
          },
        ),
      );
    });

    test('unknown versions always throw ArgumentError', () {
      check(
        'unknown-version',
        checkProperty<String>(
          BigInt.from(6008),
          PbtOptions(numTests: 200, seed: BigInt.from(6008)),
          (rng) => hostilePython(1, 12).sample(rng),
          (version) {
            if (supportedPythonVersions.contains(version) || version.isEmpty) {
              return true;
            }
            try {
              tokenizePython('x = 1\n', version);
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

    test('releases and minor lines never cross', () {
      check(
        'never-cross',
        checkProperty<String>(
          BigInt.from(6009),
          PbtOptions(numTests: 200, seed: BigInt.from(6009)),
          (rng) => hostilePython(1, 12).sample(rng),
          (id) {
            final isRelease = supportedPythonReleases.contains(id);
            final isVersion =
                supportedPythonVersions.contains(id) || id.isEmpty;
            // A release ID must fail legacy resolution (unless it is
            // also a legacy version, which none is) and a legacy
            // version must have no release grammar.
            if (isRelease && !isVersion) {
              try {
                tokenizePython('x\n', id);
              } on ArgumentError {
                return tokenGrammarForRelease(id) != null;
              }
              return false;
            }
            if (isVersion && !isRelease && id.isNotEmpty) {
              return tokenGrammarForRelease(id) == null;
            }
            return true;
          },
        ),
      );
    });

    test('every release has a class and classes are stable', () {
      check(
        'class-stability',
        checkProperty<int>(
          BigInt.from(6010),
          PbtOptions(numTests: 150, seed: BigInt.from(6010)),
          (rng) => smallInt(0, supportedPythonReleases.length - 1).sample(rng),
          (index) {
            final release = supportedPythonReleases[index];
            final cls = pythonGrammarClass(release);
            if (cls == null || !cls.startsWith('g_')) return false;
            // Same-input determinism: the class never wobbles.
            return pythonGrammarClass(release) == cls;
          },
        ),
      );
    });
  });
}
