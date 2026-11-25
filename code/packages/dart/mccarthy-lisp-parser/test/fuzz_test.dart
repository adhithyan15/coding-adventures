/// Hostile-input fuzz campaign for
/// `coding_adventures_mccarthy_lisp_parser`, using the in-lane `fuzz`
/// package: raw bytes decode (malformed sequences become U+FFFD, like
/// any caller-supplied string) and drive [parseMcCarthy]. Only
/// [McCarthyParseError] may escape; every failure must carry a shaped
/// error, agree with [tryParseMcCarthy], and every success must
/// re-parse deterministically with a stable display. Fixed seed, so
/// the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_mccarthy_lisp_parser/mccarthy_lisp_parser.dart';
import 'package:test/test.dart';

bool _formsEqual(List<LispExpr> a, List<LispExpr> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final List<LispExpr> forms;
  try {
    forms = parseMcCarthy(text);
  } on McCarthyParseError catch (e) {
    cov.recordEdge(100 + (e.message.length % 8));
    if (e.message.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryParseMcCarthy(text) != null) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (forms.length % 16));
  final soft = tryParseMcCarthy(text);
  if (soft == null || !_formsEqual(soft, forms)) {
    return const OutcomeCrash('model-divergence', ['api-disagree']);
  }
  if (!_formsEqual(parseMcCarthy(text), forms)) {
    return const OutcomeCrash('model-divergence', ['nondeterministic']);
  }
  if (parseMcCarthyToCst(text).ruleName != 'program') {
    return const OutcomeCrash('model-divergence', ['cst-disagree']);
  }
  final once = forms.map((f) => f.toString()).join(' ');
  final deep = _maxParenDepth(once) > 59;
  try {
    final redisplayed = parseMcCarthy(once).map((f) => f.toString()).join(' ');
    if (deep || redisplayed != once) {
      return const OutcomeCrash('model-divergence', ['display-unstable']);
    }
  } on McCarthyParseError {
    // Displayed output is always grammatically valid, so only the
    // depth caps may reject it — and the engine list cap (59) binds.
    if (!deep) {
      return const OutcomeCrash('model-divergence', ['display-unstable']);
    }
  }
  return const OutcomePass();
}

int _maxParenDepth(String s) {
  var depth = 0;
  var max = 0;
  for (var i = 0; i < s.length; i++) {
    final c = s.codeUnitAt(i);
    if (c == 0x28) {
      depth++;
      if (depth > max) max = depth;
    } else if (c == 0x29) {
      depth--;
    }
  }
  return max;
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode("(CAR '(A B C))")),
      Uint8List.fromList(utf8.encode('(LAMBDA (X) X)\nNIL\n-42\n')),
      Uint8List.fromList(utf8.encode("'X\n(A . B)\n() ; comment\n")),
      Uint8List.fromList(utf8.encode('(' * 100)),
      Uint8List.fromList(utf8.encode("${'\'' * 100}X")),
      Uint8List.fromList(utf8.encode('(A . . B)\n(. X)\ncar\n')),
      Uint8List.fromList(utf8.encode('9' * 40)),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(5720)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
