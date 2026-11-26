/// Hostile-input fuzz campaign for `coding_adventures_toml_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [parseToml]. Only [TomlParseError] may escape; every successful run
/// must root at `document`, agree with [tryParseToml], and re-parse
/// deterministically. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_toml_parser/toml_parser.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final ASTNode ast;
  try {
    ast = parseToml(text);
  } on TomlParseError catch (e) {
    cov.recordEdge(100 + (e.message.length % 8));
    if (e != TomlParseError.lexical && e != TomlParseError.syntax) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    if (tryParseToml(text) != null) {
      return const OutcomeCrash('model-divergence', ['api-disagree']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (ast.ruleName.length % 16));
  if (ast.ruleName != 'document') {
    return const OutcomeCrash('model-divergence', ['root-shape']);
  }
  final again = parseToml(text);
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
      Uint8List.fromList(utf8.encode('title = "x"\n[server]\nport = 1\n')),
      Uint8List.fromList(utf8.encode('a = [[1]]\nb = { c = [2] }\n')),
      Uint8List.fromList(utf8.encode('[[t]]\n[[t]]\n# c\n')),
      Uint8List.fromList(utf8.encode('[[[unclosed\n= dangling\n{[(])}\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(5809)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
