/// Hostile-input fuzz campaign for `coding_adventures_fsharp_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [parseFsharp] under every supported version. Only engine errors
/// may escape; every successful run must root at `compilation_unit`,
/// agree with [tryParseFsharp], and re-parse deterministically.
/// Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fsharp_parser/fsharp_parser.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  for (final version in supportedFsharpParserVersions) {
    late final ASTNode ast;
    try {
      ast = parseFsharp(text, version);
    } on LexerError catch (e) {
      cov.recordEdge(100 + (e.line % 8));
      if (tryParseFsharp(text, version) != null) {
        return const OutcomeCrash('model-divergence', ['api-disagree']);
      }
      continue;
    } on GrammarParseError {
      cov.recordEdge(120 + (text.length % 8));
      if (tryParseFsharp(text, version) != null) {
        return const OutcomeCrash('model-divergence', ['api-disagree']);
      }
      continue;
    }
    cov.recordEdge(200 + (ast.ruleName.length % 16));
    if (ast.ruleName != 'compilation_unit') {
      return const OutcomeCrash('model-divergence', ['root-shape']);
    }
    final soft = tryParseFsharp(text, version);
    if (soft == null || soft != ast) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
    }
    if (parseFsharp(text, version) != ast) {
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
      Uint8List.fromList(utf8.encode('let value = 1\nlet f x = x + 1\n')),
      Uint8List.fromList(utf8.encode('match x with\n| 0 -> 1\n| _ -> 2\n')),
      Uint8List.fromList(utf8.encode('((((1))))\n[[[2]]]\n')),
      Uint8List.fromList(utf8.encode('let = \n((((\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6610)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
