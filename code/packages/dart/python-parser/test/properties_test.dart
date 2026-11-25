/// Property tests for `coding_adventures_python_parser`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// root shapes, determinism, exact depth boundaries (legacy 9/10,
/// every release 21/22), valid-program parsing, API agreement,
/// unknown-version rejection, release-resolution laws, budget laws,
/// and error-message hygiene — all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:coding_adventures_python_parser/python_parser.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Echo-stability oracle for the unknown-version error contract:
/// true exactly when the implementation echoes [version] verbatim
/// (short UTF-8 length, nothing the diagnostic sanitizer would
/// escape), mirroring the private renderer without importing it.
bool echoStableVersion(String version) {
  var bytes = 0;
  for (final rune in version.runes) {
    if (rune <= 0x1F ||
        (rune >= 0x7F && rune <= 0x9F) ||
        rune == 0x22 ||
        rune == 0x27 ||
        rune == 0x5C ||
        (rune >= 0xD800 && rune <= 0xDFFF)) {
      return false;
    }
    if (rune < 0x80) {
      bytes += 1;
    } else if (rune < 0x800) {
      bytes += 2;
    } else if (rune < 0x10000) {
      bytes += 3;
    } else {
      bytes += 4;
    }
  }
  return bytes <= maxVersionEchoBytes;
}

/// Python-dense hostile text: Python keywords and sigils plus
/// whitespace (including indentation) and out-of-alphabet noise.
Gen<String> hostilePython(int minLen, int maxLen) {
  const pieces = [
    'def',
    'return',
    'if',
    'else',
    'for',
    'while',
    'import',
    'class',
    'lambda',
    'pass',
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
    '(',
    ')',
    '[',
    ']',
    '{',
    '}',
    '=',
    '==',
    '!=',
    '+',
    '-',
    '*',
    '/',
    '//',
    '%',
    '<',
    '>',
    ':',
    ',',
    '.',
    '@',
    '#',
    '"',
    "'",
    '0',
    '1',
    '42',
    '3.14',
    '0xFF',
    '\n',
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

String nestedParenSource(int n) => 'x = ${'(' * n}1${')' * n}\n';

String nestedPrintSource(int n) => 'print(${'(' * n}1${')' * n})';

void main() {
  group('properties', () {
    test('legacy parse fails closed with engine errors only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(6701),
          PbtOptions(numTests: 400, seed: BigInt.from(6701)),
          (rng) => hostilePython(0, 40).sample(rng),
          (input) {
            for (final version in supportedPythonParserVersions) {
              try {
                parsePython(input, version);
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

    test('legacy root shapes hold in every version', () {
      check(
        'root-shape',
        checkProperty<String>(
          BigInt.from(6702),
          PbtOptions(numTests: 400, seed: BigInt.from(6702)),
          (rng) => hostilePython(0, 40).sample(rng),
          (input) {
            for (final version in supportedPythonParserVersions) {
              final ast = tryParsePython(input, version);
              if (ast == null) continue;
              final expected = version == '3.8' ? 'file_input' : 'file';
              if (ast.ruleName != expected) return false;
            }
            return true;
          },
        ),
      );
    });

    test('legacy parse is deterministic per version', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(6703),
          PbtOptions(numTests: 200, seed: BigInt.from(6703)),
          (rng) => hostilePython(0, 40).sample(rng),
          (input) {
            for (final version in supportedPythonParserVersions) {
              final a = tryParsePython(input, version);
              final b = tryParsePython(input, version);
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

    test('legacy boundary is exactly nine levels', () {
      check(
        'depth-boundary-legacy',
        checkProperty<int>(
          BigInt.from(6704),
          PbtOptions(numTests: 60, seed: BigInt.from(6704)),
          (rng) => smallInt(0, 30).sample(rng),
          (depth) {
            return (tryParsePython(nestedParenSource(depth), '3.12') != null) ==
                (depth <= 9);
          },
        ),
      );
    });

    test('release boundary is exactly twenty-one levels', () {
      check(
        'depth-boundary-release',
        checkProperty<List<int>>(
          BigInt.from(6705),
          PbtOptions(numTests: 120, seed: BigInt.from(6705)),
          (rng) => Gen.list(2, 2, smallInt(0, 59)).sample(rng),
          (pair) {
            final release = supportedPythonParserReleases[
                pair[0] % supportedPythonParserReleases.length];
            final depth = pair[1];
            var parsed = false;
            try {
              parsePythonAt(
                  nestedPrintSource(depth), release, const FrontendBudgets());
              parsed = true;
            } on PythonFrontendError {
              parsed = false;
            }
            return parsed == (depth <= 21);
          },
        ),
      );
    });

    test('valid legacy programs parse in every version', () {
      check(
        'valid-corpus-legacy',
        checkProperty<int>(
          BigInt.from(6706),
          PbtOptions(numTests: 150, seed: BigInt.from(6706)),
          (rng) => smallInt(0, 7).sample(rng),
          (index) {
            // Every entry parses under all six versions (the 3.8
            // grammar covers no assignments, if/for, or subscripts —
            // verified against the reference).
            const programs = [
              'print(1)\n',
              'def f():\n    return 1\n',
              'print(1 + 2 * 3)\n',
              'import os\n',
              'while x:\n    y\n',
              '# just a comment\n',
              'f(1, 2)\n',
              'print("hi")\n',
            ];
            for (final version in supportedPythonParserVersions) {
              if (tryParsePython(programs[index], version) == null) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('valid release programs parse in every release', () {
      check(
        'valid-corpus-release',
        checkProperty<int>(
          BigInt.from(6712),
          PbtOptions(numTests: 150, seed: BigInt.from(6712)),
          (rng) => smallInt(0, 7).sample(rng),
          (index) {
            // The release grammars cover the print/int/float subset.
            const programs = [
              'print(1 + 2 * 3)\n',
              'print(abs(1))\n',
              'print(-7 // 2)\n',
              'print((2))\n',
              'print(1)\n',
              '1 + 2\n',
              '# leading comment\nprint(1)\n',
              'print(1);\n',
            ];
            for (final release in supportedPythonParserReleases) {
              try {
                parsePythonAt(
                    programs[index], release, const FrontendBudgets());
              } on PythonFrontendError {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('nullable legacy API agrees with the throwing API', () {
      check(
        'api-agreement',
        checkProperty<String>(
          BigInt.from(6707),
          PbtOptions(numTests: 400, seed: BigInt.from(6707)),
          (rng) => hostilePython(0, 40).sample(rng),
          (input) {
            for (final version in supportedPythonParserVersions) {
              final soft = tryParsePython(input, version);
              try {
                final hard = parsePython(input, version);
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
          BigInt.from(6708),
          PbtOptions(numTests: 200, seed: BigInt.from(6708)),
          (rng) => hostilePython(1, 12).sample(rng),
          (version) {
            if (supportedPythonParserVersions.contains(version) ||
                version.isEmpty) {
              return true;
            }
            try {
              parsePython('x = 1\n', version);
            } on ArgumentError catch (e) {
              final message = e.message as String;
              if (!message.startsWith("Unsupported Python version '")) {
                return false;
              }
              // Security contract: no control character ever reaches
              // the diagnostic, whatever the caller supplied.
              for (final rune in message.runes) {
                if (rune <= 0x1F ||
                    (rune >= 0x7F && rune <= 0x9F) ||
                    (rune >= 0xD800 && rune <= 0xDFFF)) {
                  return false;
                }
              }
              // Fidelity contract: clean versions echo verbatim.
              if (echoStableVersion(version)) {
                return message.contains(version);
              }
              return true;
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

    test('release resolution is exact', () {
      check(
        'release-resolution',
        checkProperty<int>(
          BigInt.from(6709),
          PbtOptions(numTests: 200, seed: BigInt.from(6709)),
          (rng) =>
              smallInt(0, supportedPythonParserReleases.length - 1).sample(rng),
          (index) {
            final release = supportedPythonParserReleases[index];
            if (resolvePythonRelease(release) != release) return false;
            if (resolvePythonRelease(null) != defaultPythonRelease) {
              return false;
            }
            return releaseParserGrammar(release) != null &&
                pythonParserGrammarClass(release) != null;
          },
        ),
      );
    });

    test('budgets reject oversized input with clamped limits', () {
      check(
        'budget-laws',
        checkProperty<int>(
          BigInt.from(6710),
          PbtOptions(numTests: 60, seed: BigInt.from(6710)),
          (rng) => smallInt(65530, 65545).sample(rng),
          (length) {
            final source = 'x${' ' * (length - 1)}';
            try {
              parsePythonAt(source, null, const FrontendBudgets());
            } on PythonBudgetExceeded catch (e) {
              return length > 65536 &&
                  e.which == 'source bytes' &&
                  e.limit == 65536;
            } on PythonFrontendError {
              return length <= 65536;
            }
            return length <= 65536;
          },
        ),
      );
    });

    test('error messages never carry control characters', () {
      check(
        'error-hygiene',
        checkProperty<String>(
          BigInt.from(6711),
          PbtOptions(numTests: 400, seed: BigInt.from(6711)),
          (rng) => hostilePython(1, 24).sample(rng),
          (id) {
            if (supportedPythonParserReleases.contains(id)) return true;
            try {
              resolvePythonRelease(id);
            } on PythonUnknownRelease catch (e) {
              return !_hasControl(e.toString());
            } on PythonReleaseNotAvailable catch (e) {
              return !_hasControl(e.toString());
            }
            return false;
          },
        ),
      );
    });
  });
}

bool _hasControl(String text) {
  for (final unit in text.codeUnits) {
    if ((unit >= 0x00 && unit <= 0x1F) || unit == 0x7F) {
      return true;
    }
  }
  return false;
}
