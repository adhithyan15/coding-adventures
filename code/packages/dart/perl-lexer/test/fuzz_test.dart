/// Hostile-input fuzz campaign for `coding_adventures_perl_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// [tokenizePerl] plus every distinct release class table. Only
/// [LexerError] (and [ArgumentError] past the source cap) may escape;
/// every successful run must end with `EOF` and re-lex
/// deterministically. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_perl_lexer/perl_lexer.dart';
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

/// One class representative per distinct table: members of a class
/// share byte-identical tables, so lexing the representative covers
/// the class.
final classProbes = <String>[
  '1.000',
  '5.003_26',
  '5.003_97i',
  '5.004_72',
  '5.44.0',
];

FuzzOutcome checkRun(List<Token> Function() lex, List<Token> Function() relex) {
  late final List<Token> tokens;
  try {
    tokens = lex();
  } on ArgumentError {
    // Only the source cap throws this; the campaign never feeds
    // over-cap inputs to the capped entry point (see target).
    return const OutcomeCrash('model-divergence', ['unexpected-cap']);
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
  if (text.length <= maxPerlSourceBytes) {
    final outcome = checkRun(
      () => tokenizePerl(text),
      () => tokenizePerl(text),
    );
    if (outcome is! OutcomePass) return outcome;
  }
  for (final release in classProbes) {
    final grammar = perlReleaseTokenGrammar(release)!;
    final outcome = checkRun(
      () => GrammarLexer(text, grammar).tokenize(),
      () => GrammarLexer(text, grammar).tokenize(),
    );
    if (outcome is! OutcomePass) return outcome;
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    // The class probes must span all five distinct tables.
    expect(classProbes.map(perlGrammarClass).toSet().length, equals(5));
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('print(7 - 2 * 3);\nprint(1--2);\n')),
      Uint8List.fromList(utf8.encode('my \$x = 1;\n\$x--;\n')),
      Uint8List.fromList(utf8.encode('{{{((()))}}}\n;;;---\n')),
      Uint8List.fromList(utf8.encode('\$@%#\n\x00\xff\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6110)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
