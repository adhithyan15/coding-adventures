/// Hostile-input fuzz campaign for `coding_adventures_cobol_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [tokenizeCobol]. Only [LexerError] may escape; every successful
/// run must end in exactly one EOF with sane monotonic positions,
/// agree with [tryTokenizeCobol], and re-parse deterministically.
/// Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_cobol_lexer/cobol_lexer.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final List<Token> tokens;
  try {
    tokens = tokenizeCobol(text);
  } on LexerError catch (e) {
    cov.recordEdge(100 + (e.line % 8));
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryTokenizeCobol(text) != null) {
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
  final again = tokenizeCobol(text);
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
      Uint8List.fromList(utf8.encode(
          '000100 IDENTIFICATION DIVISION.\n000200 PROGRAM-ID. PAYROLL.\n')),
      Uint8List.fromList(utf8.encode(
          "000300*COMPUTE ONE EMPLOYEE'S PAY\n000600 01 EMP-NAME PIC X(20).\n")),
      Uint8List.fromList(
          utf8.encode('000100 MOVE FIRST-\n000200-PART TO X.\r\nshort\n')),
      Uint8List.fromList(
          utf8.encode('000700 77 GROSS-PAY PIC 9(6)V99 VALUE ZERO.\n@ # ` ~')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(4710)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
