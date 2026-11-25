/// Hostile-input fuzz campaign for `coding_adventures_mermaid_lexer`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// every diagram family. Only [LexerError] may escape the factories;
/// every successful run must end with `EOF` and re-lex
/// deterministically. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mermaid_lexer/mermaid_lexer.dart';
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

final _lexers = <GrammarLexer Function(String)>[
  createMermaidLexer,
  createMermaidPieLexer,
  createMermaidSankeyLexer,
  createMermaidGitgraphLexer,
  createMermaidErLexer,
  createMermaidC4Lexer,
  createMermaidSequenceLexer,
  createMermaidStateLexer,
  createMermaidQuadrantLexer,
  createMermaidJourneyLexer,
  createMermaidTimelineLexer,
  createMermaidMindmapLexer,
  createMermaidBlockLexer,
  createMermaidPacketLexer,
  createMermaidKanbanLexer,
  createMermaidArchitectureLexer,
  createMermaidRadarLexer,
  createMermaidEventmodelingLexer,
  createMermaidTreemapLexer,
  createMermaidVennLexer,
  createMermaidIshikawaLexer,
  createMermaidWardleyLexer,
  createMermaidCynefinLexer,
  createMermaidTreeviewLexer,
  createMermaidSwimlaneLexer,
  createMermaidRailroadLexer,
  createMermaidRailroadEbnfLexer,
  createMermaidRailroadAbnfLexer,
  createMermaidRailroadPegLexer,
  createMermaidInfoLexer,
  createMermaidZenumlLexer,
  createMermaidRequirementLexer,
  createMermaidXychartLexer,
  createMermaidGanttLexer,
];

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  for (var i = 0; i < _lexers.length; i++) {
    late final List<Token> tokens;
    try {
      tokens = _lexers[i](text).tokenize();
    } on LexerError catch (e) {
      cov.recordEdge(100 + (e.line % 8));
      if (e.errorId.isEmpty || e.line < 1 || e.column < 1) {
        return const OutcomeCrash('model-divergence', ['error-shape']);
      }
      continue;
    }
    cov.recordEdge(200 + (tokens.length % 16));
    if (tokens.isEmpty || tokens.last.type != 'EOF') {
      return const OutcomeCrash('model-divergence', ['eof-shape']);
    }
    final again = _lexers[i](text).tokenize();
    if (!tokensEqual(again, tokens)) {
      return const OutcomeCrash('model-divergence', ['nondeterministic']);
    }
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes fail closed and stay consistent', () {
    expect(_lexers.length, equals(34));
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('flowchart LR\nA[Start] --> B\n')),
      Uint8List.fromList(utf8.encode('erDiagram\nCUSTOMER ||--o{ ORDER\n')),
      Uint8List.fromList(utf8.encode('sequenceDiagram\nA->>B: Hi\n')),
      Uint8List.fromList(utf8.encode('pie\n"a" : 1\n%%c\n')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(6509)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
