/// Hostile-input fuzz campaign for the [GrammarLexer] group/callback
/// engine, using the in-lane `fuzz` package: byte strings decode to
/// hostile grammars plus hostile sources driving scripted callbacks,
/// and the model asserts the fail-closed vocabulary (only [LexerError],
/// [ArgumentError], and [UnsupportedError] escape), the EOF invariant,
/// and group-stack validity. Fixed seed, so the campaign reproduces
/// exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_grammar_tools/grammar_tools.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:test/test.dart';

const _pieces = [
  'a',
  'b',
  '<',
  '>',
  '=',
  '"',
  ' ',
  '[ab]+',
  '[0-9]+',
  '[^<]+',
  'a*',
  'ab?',
];

TokenDefinition _def(String name, String pattern) =>
    TokenDefinition(name: name, pattern: pattern, isRegex: true, lineNumber: 1);

/// Builds a hostile grammar from [bytes] starting at [offset].
TokenGrammar _grammarFromBytes(Uint8List bytes, int offset) {
  String piece(int i) =>
      _pieces[bytes[(offset + i) % bytes.length] % _pieces.length];
  return TokenGrammar(
    escapeMode: bytes[offset % bytes.length].isEven ? 'none' : null,
    skipDefinitions: [
      if (bytes[(offset + 1) % bytes.length].isEven) _def('S0', piece(2))
    ],
    definitions: [_def('D0', piece(3)), _def('D1', piece(4))],
    groups: {
      'g1': PatternGroup(
          name: 'g1',
          definitions: [_def('G0', piece(5)), _def('G1', piece(6))]),
      if (bytes[(offset + 7) % bytes.length].isEven)
        'g2': PatternGroup(name: 'g2', definitions: [_def('H0', piece(8))]),
    },
  );
}

/// Builds hostile source text from the tail of [bytes].
String _sourceFromBytes(Uint8List bytes) {
  const alphabet = 'ab<>= "01xy\n\t;';
  final buffer = StringBuffer();
  for (var i = 0; i < bytes.length; i++) {
    buffer.write(alphabet[bytes[i] % alphabet.length]);
  }
  return buffer.toString();
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  cov.recordEdge(input.length % 64);
  if (input.length < 10) return const OutcomePass();
  final grammar = _grammarFromBytes(input, 0);
  final source = _sourceFromBytes(input);
  cov.recordEdge(source.length % 32);

  final lexer = GrammarLexer(source, grammar);
  var step = 0;
  lexer.setOnToken((token, ctx) {
    // The active group is always known.
    if (ctx.activeGroup != 'default' &&
        ctx.activeGroup != 'g1' &&
        ctx.activeGroup != 'g2') {
      throw StateError('unknown active group');
    }
    final action = input[(step++ + 5) % input.length] % 4;
    if (action == 0) {
      ctx.pushGroup(step.isEven ? 'g1' : 'g2');
    } else if (action == 1) {
      ctx.popGroup();
    } else if (action == 2) {
      ctx.suppress();
    }
    // action 3: observe only.
  });

  List<Token> tokens;
  try {
    tokens = lexer.tokenize();
  } on LexerError {
    cov.recordEdge(100);
    return const OutcomePass();
  } on ArgumentError {
    // Unknown-group pushes (the 'g2' group is conditional).
    cov.recordEdge(101);
    return const OutcomePass();
  } on UnsupportedError {
    cov.recordEdge(102);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['${e.runtimeType}']);
  }
  cov.recordEdge(103);

  if (tokens.isEmpty || tokens.last.type != 'EOF') {
    return const OutcomeCrash('model-divergence', ['eof']);
  }
  if (tokens.take(tokens.length - 1).any((t) => t.type == 'EOF')) {
    return const OutcomeCrash('model-divergence', ['eof-once']);
  }
  cov.recordEdge(110 + (tokens.length % 8));
  return const OutcomePass();
}

void main() {
  test('hostile campaign finds no escapes or divergences', () {
    final seeds = <Uint8List>[
      Uint8List(0),
      Uint8List.fromList(List.filled(16, 0)),
      Uint8List.fromList([0, 1, 2, 3, 4, 5, 60, 97, 62, 32, 98]),
      Uint8List.fromList(List.generate(32, (i) => (i * 37 + 11) % 256)),
      Uint8List.fromList(List.generate(64, (i) => (i * 53 + 7) % 256)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(1506)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
