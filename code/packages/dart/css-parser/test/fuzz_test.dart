/// Hostile-input fuzz campaign for `coding_adventures_css_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [parseCss]. Only [LexerError] and [GrammarParseError] may escape;
/// every successful run must root at `stylesheet`, agree with the
/// factory path, and re-parse deterministically. Fixed seed, so the
/// campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_css_parser/css_parser.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final ASTNode ast;
  try {
    ast = parseCss(text);
  } on LexerError catch (e) {
    cov.recordEdge(100 + (e.line % 8));
    if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    var factoryThrew = false;
    try {
      createCssParser(text).parse();
    } on LexerError {
      factoryThrew = true;
    } on GrammarParseError {
      factoryThrew = true;
    }
    if (!factoryThrew) {
      return const OutcomeCrash('model-divergence', ['factory-disagree']);
    }
    return const OutcomePass();
  } on GrammarParseError catch (e) {
    cov.recordEdge(120 + (e.message.length % 8));
    if (e.message.isEmpty) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    final token = e.token;
    if (token != null && (token.line < 1 || token.column < 1)) {
      return const OutcomeCrash('model-divergence', ['error-shape']);
    }
    var factoryThrew = false;
    try {
      createCssParser(text).parse();
    } on LexerError {
      factoryThrew = true;
    } on GrammarParseError {
      factoryThrew = true;
    }
    if (!factoryThrew) {
      return const OutcomeCrash('model-divergence', ['factory-disagree']);
    }
    return const OutcomePass();
  }
  cov.recordEdge(200 + (ast.ruleName.length % 16));
  if (ast.ruleName != 'stylesheet') {
    return const OutcomeCrash('model-divergence', ['root-shape']);
  }
  final again = parseCss(text);
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
      Uint8List.fromList(
          utf8.encode('body { color: red; }\n.a{.b{color:red;}}\n')),
      Uint8List.fromList(utf8.encode(
          '@media screen { a{width:calc(1);} }\n:not(a){color:red;}\n')),
      Uint8List.fromList(
          utf8.encode('@supports (a:1) {a{color:red;}}\ndiv p > a+b {}\n')),
      Uint8List.fromList(utf8.encode('{{{{}}}} ((( ))) ` ~ \$ @ é\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(5510)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
