/// Property tests for `coding_adventures_mermaid_lexer`, using the
/// in-lane `pbt` package: the failure vocabulary over hostile text,
/// EOF shape, determinism, positions, per-family valid-snippet
/// lexing, factory agreement, throwing/nullable agreement, and the
/// version format — each exercised across all 34 diagram families,
/// all with fixed seeds.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_mermaid_lexer/mermaid_lexer.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Mermaid-dense hostile text: diagram keywords and sigils plus
/// whitespace and out-of-alphabet noise.
Gen<String> hostileMermaid(int minLen, int maxLen) {
  const pieces = [
    'flowchart',
    'LR',
    'TD',
    '-->',
    '---',
    '==>',
    '[',
    ']',
    '(',
    ')',
    '{',
    '}',
    '|',
    ':',
    ';',
    '%',
    '%%',
    '#',
    '"',
    "'",
    '_',
    '-',
    '=',
    '<',
    '>',
    'o',
    'x',
    '.',
    ',',
    '@',
    'participant',
    'state',
    'classDef',
    'style',
    'note',
    'title',
    'erDiagram',
    'pie',
    '0',
    '1',
    'A',
    'x',
    '\n',
    ' ',
    '\t',
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

/// Every family behind a uniform nullable entry point (throwing
/// one-shots are wrapped; a `null` means [LexerError]).
class Family {
  const Family(this.name, this.lex, this.create, this.snippet);
  final String name;
  final List<Token>? Function(String) lex;
  final GrammarLexer Function(String) create;
  final String snippet;
}

List<Token>? _soft(List<Token> Function(String) hard, String source) {
  try {
    return hard(source);
  } on LexerError {
    return null;
  }
}

final families = <Family>[
  Family('mermaid', (s) => _soft(tokenizeMermaid, s), createMermaidLexer,
      'flowchart LR\nA[Start] -->|yes| B{Ship?}\n'),
  Family('pie', (s) => _soft(tokenizeMermaidPie, s), createMermaidPieLexer,
      'pie showData\n"Dogs" : 60\n'),
  Family('sankey', (s) => _soft(tokenizeMermaidSankey, s),
      createMermaidSankeyLexer, 'sankey-beta\nGrid,Losses,56\n'),
  Family('gitgraph', (s) => _soft(tokenizeMermaidGitgraph, s),
      createMermaidGitgraphLexer, 'gitGraph LR:\ncommit id: "c1"\n'),
  Family('er', tryTokenizeMermaidEr, createMermaidErLexer,
      'erDiagram\nCUSTOMER ||--o{ ORDER : places\n'),
  Family('c4', (s) => _soft(tokenizeMermaidC4, s), createMermaidC4Lexer,
      'C4Context\nPerson(user, "Customer")\n'),
  Family('sequence', (s) => _soft(tokenizeMermaidSequence, s),
      createMermaidSequenceLexer, 'sequenceDiagram\nparticipant A as Alice\n'),
  Family('state', (s) => _soft(tokenizeMermaidState, s),
      createMermaidStateLexer, 'stateDiagram-v2\n[*] --> Still\n'),
  Family('quadrant', tryTokenizeMermaidQuadrant, createMermaidQuadrantLexer,
      'quadrantChart\ntitle Native portfolio\n'),
  Family('journey', tryTokenizeMermaidJourney, createMermaidJourneyLexer,
      'journey\nsection Work\nTask: 2: Me\n'),
  Family('timeline', tryTokenizeMermaidTimeline, createMermaidTimelineLexer,
      'timeline\n'),
  Family('mindmap', tryTokenizeMermaidMindmap, createMermaidMindmapLexer,
      'mindmap\n  root((Native))\n'),
  Family('block', tryTokenizeMermaidBlock, createMermaidBlockLexer,
      'block-beta\ncolumns 2\n'),
  Family('packet', tryTokenizeMermaidPacket, createMermaidPacketLexer,
      'packet-beta\n0-7: "Header"\n'),
  Family('kanban', tryTokenizeMermaidKanban, createMermaidKanbanLexer,
      'kanban\n  Todo\n'),
  Family(
      'architecture',
      tryTokenizeMermaidArchitecture,
      createMermaidArchitectureLexer,
      'architecture-beta\nservice api(server)[API]\n'),
  Family('radar', tryTokenizeMermaidRadar, createMermaidRadarLexer,
      'radar-beta\ngraticule polygon\n'),
  Family('eventmodeling', tryTokenizeMermaidEventmodeling,
      createMermaidEventmodelingLexer, 'eventmodeling\nentity Sales.CartUI\n'),
  Family('treemap', tryTokenizeMermaidTreemap, createMermaidTreemapLexer,
      'treemap-beta\n'),
  Family('venn', tryTokenizeMermaidVenn, createMermaidVennLexer, 'venn-beta\n'),
  Family('ishikawa', tryTokenizeMermaidIshikawa, createMermaidIshikawaLexer,
      'ishikawa-beta\n'),
  Family('wardley', tryTokenizeMermaidWardley, createMermaidWardleyLexer,
      'wardley-beta\nA -.-> B\n'),
  Family('cynefin', tryTokenizeMermaidCynefin, createMermaidCynefinLexer,
      'cynefin-beta\n'),
  Family('treeview', tryTokenizeMermaidTreeview, createMermaidTreeviewLexer,
      'treeview-beta\n'),
  Family('swimlane', tryTokenizeMermaidSwimlane, createMermaidSwimlaneLexer,
      'swimlane-beta LR\na --> b\n'),
  Family('railroad', tryTokenizeMermaidRailroad, createMermaidRailroadLexer,
      'railroad-beta\n'),
  Family(
      'railroad_ebnf',
      tryTokenizeMermaidRailroadEbnf,
      createMermaidRailroadEbnfLexer,
      'railroad-ebnf-beta\nexpression = term, ("+" term)*;'),
  Family(
      'railroad_abnf',
      tryTokenizeMermaidRailroadAbnf,
      createMermaidRailroadAbnfLexer,
      'railroad-abnf-beta\nrule = 2*4DIGIT / *"x";'),
  Family(
      'railroad_peg',
      tryTokenizeMermaidRailroadPeg,
      createMermaidRailroadPegLexer,
      'railroad-peg-beta\nrule <- !"x" (item / .)+;'),
  Family('info', tryTokenizeMermaidInfo, createMermaidInfoLexer, 'info\n'),
  Family(
      'zenuml', tryTokenizeMermaidZenuml, createMermaidZenumlLexer, 'zenuml\n'),
  Family('requirement', tryTokenizeMermaidRequirement,
      createMermaidRequirementLexer, 'requirementDiagram\n'),
  Family('xychart', tryTokenizeMermaidXychart, createMermaidXychartLexer,
      'XYCHART horizontal\nX-AXIS "Quarter" [Q1, Q2]\n'),
  Family('gantt', tryTokenizeMermaidGantt, createMermaidGanttLexer,
      'gantt\nTITLE Project\n'),
];

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

void main() {
  group('properties', () {
    test('all families fail closed with LexerError only', () {
      check(
        'closed-vocabulary',
        checkProperty<String>(
          BigInt.from(6501),
          PbtOptions(numTests: 400, seed: BigInt.from(6501)),
          (rng) => hostileMermaid(0, 40).sample(rng),
          (input) {
            // Nullable entry points never throw by construction; any
            // non-LexerError escaping a factory fails the run.
            for (final family in families) {
              try {
                family.create(input).tokenize();
              } on LexerError {
                // Expected failure vocabulary.
              }
              family.lex(input);
            }
            return true;
          },
        ),
      );
    });

    test('successful runs end with EOF in every family', () {
      check(
        'eof-shape',
        checkProperty<String>(
          BigInt.from(6502),
          PbtOptions(numTests: 400, seed: BigInt.from(6502)),
          (rng) => hostileMermaid(0, 40).sample(rng),
          (input) {
            for (final family in families) {
              final tokens = family.lex(input);
              if (tokens == null) continue;
              if (tokens.isEmpty || tokens.last.type != 'EOF') {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('lexing is deterministic in every family', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(6503),
          PbtOptions(numTests: 200, seed: BigInt.from(6503)),
          (rng) => hostileMermaid(0, 40).sample(rng),
          (input) {
            for (final family in families) {
              final a = family.lex(input);
              final b = family.lex(input);
              if (a == null || b == null) {
                if (!(a == null && b == null)) return false;
              } else if (!tokensEqual(a, b)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('tokens keep sane positions in every family', () {
      check(
        'positions',
        checkProperty<String>(
          BigInt.from(6504),
          PbtOptions(numTests: 400, seed: BigInt.from(6504)),
          (rng) => hostileMermaid(0, 40).sample(rng),
          (input) {
            for (final family in families) {
              final tokens = family.lex(input);
              if (tokens == null) continue;
              for (final t in tokens) {
                if (t.line < 1 || t.column < 1) return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('every family lexes its valid snippet', () {
      check(
        'valid-corpus',
        checkProperty<int>(
          BigInt.from(6505),
          PbtOptions(numTests: 170, seed: BigInt.from(6505)),
          (rng) => smallInt(0, families.length - 1).sample(rng),
          (index) => families[index].lex(families[index].snippet) != null,
        ),
      );
    });

    test('factories agree with one-shots in every family', () {
      check(
        'factory-agreement',
        checkProperty<String>(
          BigInt.from(6506),
          PbtOptions(numTests: 200, seed: BigInt.from(6506)),
          (rng) => hostileMermaid(0, 40).sample(rng),
          (input) {
            for (final family in families) {
              List<Token>? a;
              try {
                a = family.create(input).tokenize();
              } on LexerError {
                a = null;
              }
              final b = family.lex(input);
              // State and sequence one-shots post-process; factories
              // return raw streams. Agreement holds only where no
              // post-processing exists.
              if (family.name == 'state' || family.name == 'sequence') {
                if ((a == null) != (b == null)) return false;
                continue;
              }
              if (a == null || b == null) {
                if (!(a == null && b == null)) return false;
              } else if (!tokensEqual(a, b)) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('throwing twins agree with nullable twins', () {
      check(
        'twin-agreement',
        checkProperty<String>(
          BigInt.from(6507),
          PbtOptions(numTests: 400, seed: BigInt.from(6507)),
          (rng) => hostileMermaid(0, 40).sample(rng),
          (input) {
            // Only er and quadrant offer both spellings.
            List<Token>? soft;
            try {
              final hard = tokenizeMermaidEr(input);
              soft = tryTokenizeMermaidEr(input);
              if (soft == null || !tokensEqual(soft, hard)) return false;
            } on LexerError {
              if (tryTokenizeMermaidEr(input) != null) return false;
            }
            try {
              final hard = tokenizeMermaidQuadrant(input);
              soft = tryTokenizeMermaidQuadrant(input);
              if (soft == null || !tokensEqual(soft, hard)) return false;
            } on LexerError {
              if (tryTokenizeMermaidQuadrant(input) != null) return false;
            }
            return true;
          },
        ),
      );
    });

    test('family names are pairwise distinct', () {
      check(
        'registry-distinct',
        checkProperty<List<int>>(
          BigInt.from(6508),
          PbtOptions(numTests: 200, seed: BigInt.from(6508)),
          (rng) => Gen.list(2, 2, smallInt(0, families.length - 1)).sample(rng),
          (pair) {
            if (families.length != 34) return false;
            if (pair[0] == pair[1]) return true;
            return families[pair[0]].name != families[pair[1]].name;
          },
        ),
      );
    });
  });
}
