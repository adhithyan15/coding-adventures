/// Property tests for `coding_adventures_fsharp_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// root shape, determinism, positions, the exact paren-depth
/// boundary, valid-program parsing, API agreement, unknown-version
/// rejection, and version identity — all with fixed seeds.
library;

import 'package:coding_adventures_fsharp_lexer/fsharp_lexer.dart';
import 'package:coding_adventures_fsharp_parser/fsharp_parser.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
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
    'rec',
    'mutable',
    'for',
    'do',
    'return',
    '|',
    '->',
    '<-',
    '=',
    '+',
    '-',
    '*',
    '/',
    '%',
    '<',
    '>',
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
    '#',
    '.',
    ',',
    ';',
    ':',
    '_',
    'x',
    'value',
    '0',
    '1',
    '3.14',
    '\n',
    ' ',
    '\t',
    'é',
    '🎉',
    r'$',
    '@',
    '?',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

String nestedParenSource(int n) => 'let x = ${'(' * n}1${')' * n}';

void main() {
  group('properties', () {
    test('parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(6601),
          PbtOptions(numTests: 400, seed: BigInt.from(6601)),
          (rng) => hostileFsharp(0, 40).sample(rng),
          (input) {
            for (final version in supportedFsharpParserVersions) {
              try {
                parseFsharp(input, version);
              } on LexerError {
                // Expected failure vocabulary.
              } on GrammarParseError {
                // Expected failure vocabulary.
              }
            }
            return true;
          },
        ),
      );
    });

    test('successful parses root at compilation_unit', () {
      check(
        'root-shape',
        checkProperty<String>(
          BigInt.from(6602),
          PbtOptions(numTests: 400, seed: BigInt.from(6602)),
          (rng) => hostileFsharp(0, 40).sample(rng),
          (input) {
            for (final version in supportedFsharpParserVersions) {
              final ast = tryParseFsharp(input, version);
              if (ast == null) continue;
              if (ast.ruleName != 'compilation_unit') return false;
            }
            return true;
          },
        ),
      );
    });

    test('parse is deterministic per version', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(6603),
          PbtOptions(numTests: 200, seed: BigInt.from(6603)),
          (rng) => hostileFsharp(0, 40).sample(rng),
          (input) {
            for (final version in supportedFsharpParserVersions) {
              final a = tryParseFsharp(input, version);
              final b = tryParseFsharp(input, version);
              if (a == null || b == null) {
                if (!(a == null && b == null)) return false;
              } else if (a != b) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('paren boundary is exactly twenty-seven levels', () {
      check(
        'depth-boundary',
        checkProperty<int>(
          BigInt.from(6604),
          PbtOptions(numTests: 60, seed: BigInt.from(6604)),
          (rng) => smallInt(0, 60).sample(rng),
          (depth) {
            return (tryParseFsharp(nestedParenSource(depth)) != null) ==
                (depth <= 27);
          },
        ),
      );
    });

    test('valid programs parse in every version', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(6605),
          PbtOptions(numTests: 150, seed: BigInt.from(6605)),
          (rng) => smallInt(0, 7).sample(rng),
          (index) {
            const programs = [
              'let value = 1',
              'let f x = x + 1',
              'let pi = 3.14',
              'let name = "Alice"',
              'let t = (1, 2)',
              'let l = [1; 2; 3]',
              'let x = 1 + 2 * 3',
              'module M\nlet x = 255',
            ];
            for (final version in supportedFsharpParserVersions) {
              if (tryParseFsharp(programs[index], version) == null) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('nullable API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(6606),
          PbtOptions(numTests: 400, seed: BigInt.from(6606)),
          (rng) => hostileFsharp(0, 40).sample(rng),
          (input) {
            for (final version in supportedFsharpParserVersions) {
              final soft = tryParseFsharp(input, version);
              try {
                final hard = parseFsharp(input, version);
                if (soft == null || soft != hard) return false;
              } on LexerError {
                if (soft != null) return false;
              } on GrammarParseError {
                if (soft != null) return false;
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
          BigInt.from(6607),
          PbtOptions(numTests: 200, seed: BigInt.from(6607)),
          (rng) => hostileFsharp(1, 12).sample(rng),
          (version) {
            if (supportedFsharpParserVersions.contains(version) ||
                version.isEmpty) {
              return true;
            }
            try {
              parseFsharp('let value = 1', version);
            } on ArgumentError catch (e) {
              return e.message.contains(version);
            } on LexerError {
              return false;
            } on GrammarParseError {
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
        checkProperty<int>(
          BigInt.from(6608),
          PbtOptions(numTests: 150, seed: BigInt.from(6608)),
          (rng) =>
              smallInt(0, supportedFsharpParserVersions.length - 1).sample(rng),
          (index) {
            final version = supportedFsharpParserVersions[index];
            return resolveFsharpParserVersion(version) == version;
          },
        ),
      );
    });

    test('parser versions match the lexer versions', () {
      check(
        'version-parity',
        checkProperty<int>(
          BigInt.from(6609),
          PbtOptions(numTests: 150, seed: BigInt.from(6609)),
          (rng) =>
              smallInt(0, supportedFsharpParserVersions.length - 1).sample(rng),
          (index) =>
              supportedFsharpParserVersions[index] ==
              supportedFsharpVersions[index],
        ),
      );
    });
  });
}
