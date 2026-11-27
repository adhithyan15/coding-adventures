/// Hostile-input fuzz campaign for `coding_adventures_lisp_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [parseLisp]. Only [LispParseError] may escape; every successful run
/// must agree with the nullable twin and re-parse deterministically.
/// Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lisp_parser/lisp_parser.dart';
import 'package:test/test.dart';

bool forestsEqual(List<SExpr> a, List<SExpr> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final List<SExpr> forest;
  try {
    forest = parseLisp(text);
  } on LispParseError catch (e) {
    cov.recordEdge(100 + (e.message.length % 8));
    if (e.message.isEmpty) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryParseLisp(text) != null) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (forest.length % 16));
  final soft = tryParseLisp(text);
  if (soft == null || !forestsEqual(soft, forest)) {
    return const OutcomeCrash('model-divergence', ['twin-disagree']);
  }
  // Counters stay consistent across re-parse.
  int atoms(List<SExpr> f) =>
      f.fold<int>(0, (n, t) => n + t.findAtoms().length);
  if (atoms(parseLisp(text)) != atoms(forest)) {
    return const OutcomeCrash('model-divergence', ['nondeterministic']);
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('(+ 1 2)')),
      Uint8List.fromList(utf8.encode("(define fact (lambda (n) n))\n'foo")),
      Uint8List.fromList(utf8.encode('(a . b)\n"str" ; comment\n')),
      Uint8List.fromList(utf8.encode('((((x)))) # ` ~ @ é\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(5500)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
