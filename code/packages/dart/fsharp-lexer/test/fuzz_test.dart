/// Hostile-input fuzz campaign for `coding_adventures_fsharp_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [tokenizeFsharp] under every supported version. Only [LexerError]
/// may escape; every successful run must end with `EOF` and re-lex
/// deterministically. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fsharp_lexer/fsharp_lexer.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
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

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  for (final version in supportedFsharpVersions) {
    late final List<Token> tokens;
    try {
      tokens = tokenizeFsharp(text, version);
    } on LexerError catch (e) {
      cov.recordEdge(100 + (e.line % 8));
      if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
        return const OutcomeCrash('model-divergence', ['error-shape']);
      }
      continue;
    }
    cov.recordEdge(200 + (tokens.length % 16));
    if (tokens.isEmpty || tokens.last.type != 'EOF') {
      return const OutcomeCrash('model-divergence', ['eof-shape']);
    }
    final again = tokenizeFsharp(text, version);
    if (!tokensEqual(again, tokens)) {
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
      Uint8List.fromList(
          utf8.encode('match x with\n| 0 -> "zero"\n| _ -> "n"\n')),
      Uint8List.fromList(utf8.encode('let s = "a\\"b"\nlet c = \'z\'\n')),
      Uint8List.fromList(utf8.encode('(((|||>>>---\n"""\'\'\'\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(5910)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
