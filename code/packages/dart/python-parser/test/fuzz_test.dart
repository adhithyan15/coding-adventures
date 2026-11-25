/// Hostile-input fuzz campaign for `coding_adventures_python_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// the legacy one-shot under every version plus the release frontend
/// under every release. Only engine errors (legacy) and
/// [PythonFrontendError] (releases) may escape; every successful run
/// must root correctly and re-parse deterministically. Fixed seed, so
/// the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_python_parser/python_parser.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  for (final version in supportedPythonParserVersions) {
    late final ASTNode ast;
    try {
      ast = parsePython(text, version);
    } on LexerError catch (e) {
      cov.recordEdge(100 + (e.line % 8));
      if (tryParsePython(text, version) != null) {
        return const OutcomeCrash('model-divergence', ['api-disagree']);
      }
      continue;
    } on GrammarParseError {
      cov.recordEdge(110 + (text.length % 8));
      if (tryParsePython(text, version) != null) {
        return const OutcomeCrash('model-divergence', ['api-disagree']);
      }
      continue;
    }
    cov.recordEdge(120 + (ast.ruleName.length % 8));
    final expected = version == '3.8' ? 'file_input' : 'file';
    if (ast.ruleName != expected) {
      return const OutcomeCrash('model-divergence', ['root-shape']);
    }
    if (parsePython(text, version) != ast) {
      return const OutcomeCrash('model-divergence', ['nondeterministic']);
    }
  }
  const budgets = FrontendBudgets();
  for (final release in supportedPythonParserReleases) {
    late final PythonParse parsed;
    try {
      parsed = parsePythonAt(text, release, budgets);
    } on PythonFrontendError catch (e) {
      cov.recordEdge(200 + (e.toString().length % 16));
      // Lane-constructed errors name the release and carry no control
      // characters; engine passthroughs (Lex/Parse) echo engine text
      // verbatim, exactly like the reference.
      if (e is PythonUnknownRelease ||
          e is PythonReleaseNotAvailable ||
          e is PythonBudgetExceeded) {
        if (!e.toString().contains(release) && e is! PythonUnknownRelease) {
          return const OutcomeCrash('model-divergence', ['error-shape']);
        }
        for (final unit in e.toString().codeUnits) {
          if ((unit >= 0x00 && unit <= 0x1F) || unit == 0x7F) {
            return const OutcomeCrash('model-divergence', ['error-hygiene']);
          }
        }
      }
      continue;
    }
    cov.recordEdge(220 + (parsed.ast.ruleName.length % 8));
    if (parsed.release != release) {
      return const OutcomeCrash('model-divergence', ['release-echo']);
    }
    if (parsed.ast.ruleName != 'file') {
      return const OutcomeCrash('model-divergence', ['root-shape']);
    }
    final again = parsePythonAt(text, release, budgets);
    if (again.release != release || again.ast != parsed.ast) {
      return const OutcomeCrash('model-divergence', ['nondeterministic']);
    }
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('x = 1\ndef f():\n    return 1\n')),
      Uint8List.fromList(utf8.encode('print(1 + 2 * 3)\nprint(1 // 2)\n')),
      Uint8List.fromList(utf8.encode('if x:\n\ty = 1\n')),
      Uint8List.fromList(utf8.encode('(((\n"""\n\x00\xff\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6713)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
