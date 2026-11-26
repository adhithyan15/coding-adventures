/// Hostile-input fuzz campaign for
/// `coding_adventures_dartmouth_basic_lexer`, using the in-lane `fuzz`
/// package: raw bytes decode (malformed sequences become U+FFFD, like
/// any caller-supplied string) and drive [tokenizeDartmouthBasic].
/// Only [LexerError] may escape; every successful run must end in
/// exactly one EOF with sane monotonic positions, keep both hook
/// invariants, agree with the factory path, and re-parse
/// deterministically. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_dartmouth_basic_lexer/dartmouth_basic_lexer.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

/// Whether [tokens] has the reference's exact EOF shape: exactly
/// one trailing EOF — unless an unclosed REM swallowed it, i.e. a REM
/// keyword stands after the last NEWLINE (proven against the Rust
/// reference, which drops EOF there too).
bool eofShapeOk(List<Token> tokens) {
  final eofCount = tokens.where((t) => t.type == 'EOF').length;
  if (tokens.isNotEmpty && tokens.last.type == 'EOF' && eofCount == 1) {
    return true;
  }
  if (eofCount != 0) return false;
  var lastRem = -1;
  var lastNl = -1;
  for (var i = 0; i < tokens.length; i++) {
    if (tokens[i].type == 'KEYWORD' && tokens[i].value == 'REM') {
      lastRem = i;
    }
    if (tokens[i].type == 'NEWLINE') lastNl = i;
  }
  return lastRem != -1 && lastRem > lastNl;
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final List<Token> tokens;
  try {
    tokens = tokenizeDartmouthBasic(text);
  } on LexerError catch (e) {
    cov.recordEdge(100 + (e.line % 8));
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    var factoryThrew = false;
    try {
      createDartmouthBasicLexer(text).tokenize();
    } on LexerError {
      factoryThrew = true;
    }
    if (!factoryThrew) {
      return const OutcomeCrash('model-divergence', ['factory-disagree']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (tokens.length % 16));
  // EOF shape, exactly like the reference: one trailing EOF — unless
  // an unclosed REM swallowed it (a REM after the last NEWLINE).
  if (!eofShapeOk(tokens)) {
    return const OutcomeCrash('model-divergence', ['eof-shape']);
  }
  // Hook invariants, re-checked independently: LINE_NUM only leads
  // lines (no bare NUMBER there), and nothing but NEWLINE follows a
  // REM on its line.
  var atLineStart = true;
  for (final t in tokens) {
    if (t.type == 'EOF') break;
    if (atLineStart && t.type == 'NUMBER') {
      return const OutcomeCrash('model-divergence', ['relabel-leak']);
    }
    if (!atLineStart && t.type == 'LINE_NUM') {
      return const OutcomeCrash('model-divergence', ['relabel-leak']);
    }
    atLineStart = t.type == 'NEWLINE';
  }
  for (var i = 0; i < tokens.length; i++) {
    final t = tokens[i];
    if (t.type == 'KEYWORD' && t.value == 'REM') {
      for (var j = i + 1; j < tokens.length; j++) {
        final u = tokens[j];
        if (u.line != t.line) break;
        if (u.type != 'NEWLINE') {
          return const OutcomeCrash('model-divergence', ['rem-leak']);
        }
      }
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
  final again = tokenizeDartmouthBasic(text);
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
      Uint8List.fromList(utf8.encode('10 LET X = 1\n20 PRINT X\n30 END\n')),
      Uint8List.fromList(
          utf8.encode('10 REM hi 123\n20 GOTO 10\n30 IF X<>Y THEN 40\n')),
      Uint8List.fromList(
          utf8.encode('10 LET S = "A""B"\n20 PRINT A\$ Z9 1.5E-3\n')),
      Uint8List.fromList(utf8.encode('let x=1\nprint sin(fna(y))\n@ # `')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(4910)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
