/// Hostile-input fuzz campaign for `coding_adventures_python_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [tokenizePython] under every legacy version plus every release
/// grammar. Only [LexerError] may escape; every successful run must
/// end with `EOF` and re-lex deterministically. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_python_lexer/python_lexer.dart';
import 'package:test/test.dart';

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

FuzzOutcome checkRun(List<Token> Function() lex, List<Token> Function() relex) {
  late final List<Token> tokens;
  try {
    tokens = lex();
  } on LexerError catch (e) {
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    return const OutcomePass();
  }
  if (tokens.isEmpty || tokens.last.type != 'EOF') {
    return const OutcomeCrash('model-divergence', ['eof-shape']);
  }
  if (!tokensEqual(relex(), tokens)) {
    return const OutcomeCrash('model-divergence', ['nondeterministic']);
  }
  return const OutcomePass();
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  for (final version in supportedPythonVersions) {
    final outcome = checkRun(
      () => tokenizePython(text, version),
      () => tokenizePython(text, version),
    );
    if (outcome is! OutcomePass) return outcome;
  }
  for (final release in supportedPythonReleases) {
    final grammar = tokenGrammarForRelease(release)!;
    final outcome = checkRun(
      () => grammarTokenize(text, grammar),
      () => grammarTokenize(text, grammar),
    );
    if (outcome is! OutcomePass) return outcome;
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('x = 1\ndef f():\n    return 1\n')),
      Uint8List.fromList(utf8.encode('print(1 // 2)\nprint 7 / 2 <> 3\n')),
      Uint8List.fromList(utf8.encode('if x:\n\ty = 1\n')),
      Uint8List.fromList(utf8.encode('(((\n"""\n\x00\xff\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6011)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
