/// Hostile-input fuzz campaign for `coding_adventures_s_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [parseS]. Only [LexerError] and [GrammarParseError] may escape;
/// every successful run must root at `program`, agree with
/// [tryParseS], and re-parse deterministically. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_s_parser/s_parser.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final ASTNode ast;
  try {
    ast = parseS(text);
  } on LexerError catch (e) {
    cov.recordEdge(100 + (e.line % 8));
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryParseS(text) != null) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
    }
    return const OutcomePass();
  } on GrammarParseError catch (e) {
    cov.recordEdge(120 + (e.message.length % 8));
    if (e.message.isEmpty) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    final token = e.token;
    if (token != null && (token.line < 1 || token.column < 1)) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryParseS(text) != null) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (ast.ruleName.length % 16));
  if (ast.ruleName != 'program') {
    return const OutcomeCrash('model-divergence', ['root-shape']);
  }
  final again = parseS(text);
  if (again != ast) {
    return const OutcomeCrash('model-divergence', ['nondeterministic']);
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(
          utf8.encode('x <- c(1, 2, 3)\nmean(x)\nx * 10 + c(1, 2)\n')),
      Uint8List.fromList(utf8.encode('x _ 1\n1 -> x\ndf[["x"]]\nx %% 3\n')),
      Uint8List.fromList(utf8.encode('switch("a", a = , b = "hit")\n1:3+1\n')),
      Uint8List.fromList(utf8.encode('((((1))))\n` ~ \$ @ # é\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(5210)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
