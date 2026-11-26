/// Hostile-input fuzz campaign for `coding_adventures_wolfram_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [tokenizeWolfram]. Only [LexerError] may escape; every successful
/// run must end in exactly one EOF with sane monotonic positions,
/// agree with [tryTokenizeWolfram], keep the hook's depth invariant,
/// and re-parse deterministically. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_wolfram_lexer/wolfram_lexer.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final List<Token> tokens;
  try {
    tokens = tokenizeWolfram(text);
  } on LexerError catch (e) {
    cov.recordEdge(100 + (e.line % 8));
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryTokenizeWolfram(text) != null) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
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
  // Hook invariant, re-checked independently: no surviving NEWLINE
  // may sit at bracket depth (braces count, `[[` is +2).
  var depth = 0;
  for (final t in tokens) {
    if (t.type == 'NEWLINE' && depth > 0) {
      return const OutcomeCrash('model-divergence', ['hook-leak']);
    }
    if (t.type == 'LDBRACKET') {
      depth += 2;
    } else if (t.type == 'LPAREN' ||
        t.type == 'LBRACKET' ||
        t.type == 'LBRACE') {
      depth += 1;
    } else if (t.type == 'RPAREN' ||
        t.type == 'RBRACKET' ||
        t.type == 'RBRACE') {
      depth -= 1;
    }
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
  final again = tokenizeWolfram(text);
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
      Uint8List.fromList(utf8.encode('Sin[x]\n(#1 + #2) &')),
      Uint8List.fromList(utf8.encode('x[[\n 1\n]]\nf[\n a\n]\n')),
      Uint8List.fromList(utf8.encode('e //. r p /; t a /. b f /@ x')),
      Uint8List.fromList(utf8.encode('1 (* c *) + "s" _?Q ~ ` @')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(4110)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
