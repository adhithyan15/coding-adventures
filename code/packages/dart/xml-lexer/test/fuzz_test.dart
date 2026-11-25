/// Hostile-input fuzz campaign for `coding_adventures_xml_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [tokenizeXml]. Only [LexerError] may escape; every successful run
/// must end in exactly one EOF with sane monotonic positions and
/// re-parse deterministically. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_xml_lexer/xml_lexer.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final List<Token> tokens;
  try {
    tokens = tokenizeXml(text);
  } on LexerError catch (e) {
    // The documented failure vocabulary: positioned, identified.
    cov.recordEdge(100 + (e.line % 8));
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (tokens.length % 16));
  if (tokens.isEmpty || tokens.last.type != 'EOF') {
    return const OutcomeCrash('model-divergence', ['eof-shape']);
  }
  if (tokens.where((t) => t.type == 'EOF').length != 1) {
    return const OutcomeCrash('model-divergence', ['eof-count']);
  }
  var line = 1;
  var column = 1;
  for (final t in tokens) {
    if (t.line < 1 || t.column < 1) {
      return const OutcomeCrash('model-divergence', ['position-sanity']);
    }
    if (t.line < line || (t.line == line && t.column < column)) {
      return const OutcomeCrash('model-divergence', ['position-order']);
    }
    line = t.line;
    column = t.column;
  }
  final again = tokenizeXml(text);
  if (again.length != tokens.length) {
    return const OutcomeCrash('model-divergence', ['nondeterministic']);
  }
  for (var i = 0; i < tokens.length; i++) {
    if (again[i] != tokens[i]) {
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
      Uint8List.fromList(utf8.encode('<p>text</p>')),
      Uint8List.fromList(
          utf8.encode('<?xml version="1.0"?><!--c--><a b="v">&amp;</a>')),
      Uint8List.fromList(utf8.encode('<![CDATA[x < y]]><?t a?b?>')),
      Uint8List.fromList(utf8.encode('<<//==&&;;[[{{((##')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(2309)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
