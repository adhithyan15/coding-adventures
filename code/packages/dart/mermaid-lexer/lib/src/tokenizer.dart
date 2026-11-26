/// Mermaid tokenization over the compiled per-family grammars: one
/// factory per diagram family, throwing one-shots where the
/// reference has them, and nullable twins where it returns
/// `Result`. Token types keep their grammar names verbatim; the
/// final token is always `EOF`.
///
/// The reference parses each `.tokens` file at runtime on every
/// factory call; this lane compiles all 34 grammars at authoring
/// time into `_grammar.dart`, so construction is just a lookup.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// The mermaid-lexer version, mirroring the reference's `VERSION`.
const String mermaidVersion = '0.84.0';

/// Creates a [GrammarLexer] for flowchart [source].
GrammarLexer createMermaidLexer(String source) =>
    GrammarLexer(source, tokenGrammarMermaid);

/// Creates a [GrammarLexer] for pie [source].
GrammarLexer createMermaidPieLexer(String source) =>
    GrammarLexer(source, tokenGrammarPie);

/// Creates a [GrammarLexer] for sankey [source].
GrammarLexer createMermaidSankeyLexer(String source) =>
    GrammarLexer(source, tokenGrammarSankey);

/// Creates a [GrammarLexer] for gitgraph [source].
GrammarLexer createMermaidGitgraphLexer(String source) =>
    GrammarLexer(source, tokenGrammarGitgraph);

/// Creates a [GrammarLexer] for er [source].
GrammarLexer createMermaidErLexer(String source) =>
    GrammarLexer(source, tokenGrammarEr);

/// Creates a [GrammarLexer] for c4 [source].
GrammarLexer createMermaidC4Lexer(String source) =>
    GrammarLexer(source, tokenGrammarC4);

/// Creates a [GrammarLexer] for sequence [source].
GrammarLexer createMermaidSequenceLexer(String source) =>
    GrammarLexer(source, tokenGrammarSequence);

/// Creates a [GrammarLexer] for state [source].
GrammarLexer createMermaidStateLexer(String source) =>
    GrammarLexer(source, tokenGrammarState);

/// Creates a [GrammarLexer] for quadrant [source].
GrammarLexer createMermaidQuadrantLexer(String source) =>
    GrammarLexer(source, tokenGrammarQuadrant);

/// Creates a [GrammarLexer] for journey [source].
GrammarLexer createMermaidJourneyLexer(String source) =>
    GrammarLexer(source, tokenGrammarJourney);

/// Creates a [GrammarLexer] for timeline [source].
GrammarLexer createMermaidTimelineLexer(String source) =>
    GrammarLexer(source, tokenGrammarTimeline);

/// Creates a [GrammarLexer] for mindmap [source].
GrammarLexer createMermaidMindmapLexer(String source) =>
    GrammarLexer(source, tokenGrammarMindmap);

/// Creates a [GrammarLexer] for block [source].
GrammarLexer createMermaidBlockLexer(String source) =>
    GrammarLexer(source, tokenGrammarBlock);

/// Creates a [GrammarLexer] for packet [source].
GrammarLexer createMermaidPacketLexer(String source) =>
    GrammarLexer(source, tokenGrammarPacket);

/// Creates a [GrammarLexer] for kanban [source].
GrammarLexer createMermaidKanbanLexer(String source) =>
    GrammarLexer(source, tokenGrammarKanban);

/// Creates a [GrammarLexer] for architecture [source].
GrammarLexer createMermaidArchitectureLexer(String source) =>
    GrammarLexer(source, tokenGrammarArchitecture);

/// Creates a [GrammarLexer] for radar [source].
GrammarLexer createMermaidRadarLexer(String source) =>
    GrammarLexer(source, tokenGrammarRadar);

/// Creates a [GrammarLexer] for eventmodeling [source].
GrammarLexer createMermaidEventmodelingLexer(String source) =>
    GrammarLexer(source, tokenGrammarEventmodeling);

/// Creates a [GrammarLexer] for treemap [source].
GrammarLexer createMermaidTreemapLexer(String source) =>
    GrammarLexer(source, tokenGrammarTreemap);

/// Creates a [GrammarLexer] for venn [source].
GrammarLexer createMermaidVennLexer(String source) =>
    GrammarLexer(source, tokenGrammarVenn);

/// Creates a [GrammarLexer] for ishikawa [source].
GrammarLexer createMermaidIshikawaLexer(String source) =>
    GrammarLexer(source, tokenGrammarIshikawa);

/// Creates a [GrammarLexer] for wardley [source].
GrammarLexer createMermaidWardleyLexer(String source) =>
    GrammarLexer(source, tokenGrammarWardley);

/// Creates a [GrammarLexer] for cynefin [source].
GrammarLexer createMermaidCynefinLexer(String source) =>
    GrammarLexer(source, tokenGrammarCynefin);

/// Creates a [GrammarLexer] for treeview [source].
GrammarLexer createMermaidTreeviewLexer(String source) =>
    GrammarLexer(source, tokenGrammarTreeview);

/// Creates a [GrammarLexer] for swimlane [source].
GrammarLexer createMermaidSwimlaneLexer(String source) =>
    GrammarLexer(source, tokenGrammarSwimlane);

/// Creates a [GrammarLexer] for railroad [source].
GrammarLexer createMermaidRailroadLexer(String source) =>
    GrammarLexer(source, tokenGrammarRailroad);

/// Creates a [GrammarLexer] for railroad ebnf [source].
GrammarLexer createMermaidRailroadEbnfLexer(String source) =>
    GrammarLexer(source, tokenGrammarRailroadEbnf);

/// Creates a [GrammarLexer] for railroad abnf [source].
GrammarLexer createMermaidRailroadAbnfLexer(String source) =>
    GrammarLexer(source, tokenGrammarRailroadAbnf);

/// Creates a [GrammarLexer] for railroad peg [source].
GrammarLexer createMermaidRailroadPegLexer(String source) =>
    GrammarLexer(source, tokenGrammarRailroadPeg);

/// Creates a [GrammarLexer] for info [source].
GrammarLexer createMermaidInfoLexer(String source) =>
    GrammarLexer(source, tokenGrammarInfo);

/// Creates a [GrammarLexer] for zenuml [source].
GrammarLexer createMermaidZenumlLexer(String source) =>
    GrammarLexer(source, tokenGrammarZenuml);

/// Creates a [GrammarLexer] for requirement [source].
GrammarLexer createMermaidRequirementLexer(String source) =>
    GrammarLexer(source, tokenGrammarRequirement);

/// Creates a [GrammarLexer] for xychart [source].
GrammarLexer createMermaidXychartLexer(String source) =>
    GrammarLexer(source, tokenGrammarXychart);

/// Creates a [GrammarLexer] for gantt [source].
GrammarLexer createMermaidGanttLexer(String source) =>
    GrammarLexer(source, tokenGrammarGantt);

/// Tokenizes flowchart [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaid(String source) =>
    createMermaidLexer(source).tokenize();

/// Tokenizes pie [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaidPie(String source) =>
    createMermaidPieLexer(source).tokenize();

/// Tokenizes sankey [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaidSankey(String source) =>
    createMermaidSankeyLexer(source).tokenize();

/// Tokenizes gitgraph [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaidGitgraph(String source) =>
    createMermaidGitgraphLexer(source).tokenize();

/// Tokenizes er [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaidEr(String source) =>
    createMermaidErLexer(source).tokenize();

/// Tokenizes c4 [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaidC4(String source) =>
    createMermaidC4Lexer(source).tokenize();

/// Tokenizes sequence [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
///
/// Then applies the reference's keyword normalization: statement
/// keywords at line starts (and a few contextually allowed
/// followers) are rewritten to their canonical grammar spelling,
/// `as` is retagged to `AS`, and `WRAP_DIRECTIVE` values are
/// lowercased. `HASH_COMMENT` tokens are dropped.
List<Token> tokenizeMermaidSequence(String source) {
  final tokens = createMermaidSequenceLexer(source).tokenize();
  const statementKeywords = {
    'participant',
    'actor',
    'create',
    'destroy',
    'link',
    'links',
    'properties',
    'details',
    'accTitle',
    'accDescr',
    'box',
    'activate',
    'deactivate',
    'note',
    'title',
    'autonumber',
    'loop',
    'rect',
    'opt',
    'alt',
    'else',
    'par',
    'par_over',
    'and',
    'critical',
    'option',
    'break',
    'end',
  };
  final keywords = tokenGrammarSequence.keywords;
  var lineStart = true;
  String? context;
  final normalized = <Token>[];
  for (final token in tokens) {
    if (token.type == 'NEWLINE' || token.type == 'SEMICOLON') {
      lineStart = true;
      context = null;
      normalized.add(token);
      continue;
    }
    if (token.type == 'HEADER') {
      lineStart = false;
      normalized.add(token);
      continue;
    }
    String? keyword;
    for (final candidate in keywords) {
      if (_asciiEqualsIgnoreCase(candidate, token.value)) {
        keyword = candidate;
        break;
      }
    }
    var current = token;
    if (lineStart) {
      if (keyword != null && statementKeywords.contains(keyword)) {
        current = _withValue(token, keyword);
        context = keyword;
      }
      lineStart = false;
    } else if (keyword != null) {
      final allowed = switch (context) {
        'create' => keyword == 'participant' || keyword == 'actor',
        'participant' || 'actor' => keyword == 'as',
        'note' => keyword == 'left' || keyword == 'right' || keyword == 'over',
        'note-placement' => keyword == 'of',
        'autonumber' => keyword == 'off',
        _ => false,
      };
      if (allowed) {
        current = _withValue(token, keyword, keyword == 'as' ? 'AS' : null);
        context = switch ((context, keyword)) {
          ('create', 'participant' || 'actor') => keyword,
          ('note', 'left' || 'right') => 'note-placement',
          _ => null,
        };
      }
    }
    if (current.type == 'WRAP_DIRECTIVE') {
      current = _withValue(current, current.value.toLowerCase());
    }
    normalized.add(current);
  }
  normalized.removeWhere((token) => token.type == 'HASH_COMMENT');
  return normalized;
}

Token _withValue(Token token, String value, [String? type]) => Token(
      type: type ?? token.type,
      value: value,
      line: token.line,
      column: token.column,
      flags: token.flags,
    );

/// Tokenizes state [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
///
/// Then applies the reference's line filter: `HASH_COMMENT` tokens
/// are always dropped, and a `HASH_COLOR`/`ENTITY` outside a
/// semantic hash context (after `:`, `ACC_TITLE`, `ACC_DESCR`, or
/// `title`) suppresses its whole line. The context resets on every
/// `NEWLINE`, `SEMICOLON`, and `EOF` (which are always kept).
List<Token> tokenizeMermaidState(String source) {
  final tokens = createMermaidStateLexer(source).tokenize();
  final filtered = <Token>[];
  var semanticHashContext = false;
  var suppressLine = false;
  for (final token in tokens) {
    if (token.type == 'NEWLINE' ||
        token.type == 'SEMICOLON' ||
        token.type == 'EOF') {
      semanticHashContext = false;
      suppressLine = false;
      filtered.add(token);
      continue;
    }
    if (suppressLine) {
      continue;
    }
    if (token.type == 'COLON' ||
        token.type == 'ACC_TITLE' ||
        token.type == 'ACC_DESCR' ||
        _asciiEqualsIgnoreCase(token.value, 'title')) {
      semanticHashContext = true;
    }
    switch (token.type) {
      case 'HASH_COMMENT':
        break;
      case 'HASH_COLOR':
      case 'ENTITY':
        if (!semanticHashContext) {
          suppressLine = true;
        } else {
          filtered.add(token);
        }
      default:
        filtered.add(token);
    }
  }
  return filtered;
}

/// ASCII-only case-insensitive equality (mirrors
/// `eq_ignore_ascii_case`; Dart's [String.toLowerCase] would also
/// fold non-ASCII characters).
bool _asciiEqualsIgnoreCase(String a, String b) {
  if (a.length != b.length) {
    return false;
  }
  for (var i = 0; i < a.length; i++) {
    var x = a.codeUnitAt(i);
    var y = b.codeUnitAt(i);
    if (x >= 0x41 && x <= 0x5A) {
      x += 0x20;
    }
    if (y >= 0x41 && y <= 0x5A) {
      y += 0x20;
    }
    if (x != y) {
      return false;
    }
  }
  return true;
}

/// Tokenizes quadrant [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference
/// panics with a family-prefixed message; Dart lanes propagate
/// the structured error instead).
List<Token> tokenizeMermaidQuadrant(String source) =>
    createMermaidQuadrantLexer(source).tokenize();

/// Tokenizes architecture [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidArchitecture(String source) {
  try {
    return createMermaidArchitectureLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes block [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidBlock(String source) {
  try {
    return createMermaidBlockLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes cynefin [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidCynefin(String source) {
  try {
    return createMermaidCynefinLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes er [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidEr(String source) {
  try {
    return createMermaidErLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes eventmodeling [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidEventmodeling(String source) {
  try {
    return createMermaidEventmodelingLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes gantt [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidGantt(String source) {
  try {
    return createMermaidGanttLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes info [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidInfo(String source) {
  try {
    return createMermaidInfoLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes ishikawa [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidIshikawa(String source) {
  try {
    return createMermaidIshikawaLexer(source).tokenize();
    // coverage:ignore-start
    // Unreachable: the grammar's catch-all NODE_LINE accepts every
    // input (even NUL). Kept so the twin honors its contract if the
    // grammar ever tightens.
  } on LexerError {
    return null;
  }
  // coverage:ignore-end
}

/// Tokenizes journey [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidJourney(String source) {
  try {
    return createMermaidJourneyLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes kanban [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidKanban(String source) {
  try {
    return createMermaidKanbanLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes mindmap [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidMindmap(String source) {
  try {
    return createMermaidMindmapLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes packet [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidPacket(String source) {
  try {
    return createMermaidPacketLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes quadrant [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidQuadrant(String source) {
  try {
    return createMermaidQuadrantLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes radar [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidRadar(String source) {
  try {
    return createMermaidRadarLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes railroad [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidRailroad(String source) {
  try {
    return createMermaidRailroadLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes railroad abnf [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidRailroadAbnf(String source) {
  try {
    return createMermaidRailroadAbnfLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes railroad ebnf [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidRailroadEbnf(String source) {
  try {
    return createMermaidRailroadEbnfLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes railroad peg [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidRailroadPeg(String source) {
  try {
    return createMermaidRailroadPegLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes requirement [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidRequirement(String source) {
  try {
    return createMermaidRequirementLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes swimlane [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidSwimlane(String source) {
  try {
    return createMermaidSwimlaneLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes timeline [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidTimeline(String source) {
  try {
    return createMermaidTimelineLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes treemap [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidTreemap(String source) {
  try {
    return createMermaidTreemapLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes treeview [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidTreeview(String source) {
  try {
    return createMermaidTreeviewLexer(source).tokenize();
    // coverage:ignore-start
    // Unreachable: the grammar's catch-all NODE_LINE accepts every
    // input (even NUL). Kept so the twin honors its contract if the
    // grammar ever tightens.
  } on LexerError {
    return null;
  }
  // coverage:ignore-end
}

/// Tokenizes venn [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidVenn(String source) {
  try {
    return createMermaidVennLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes wardley [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidWardley(String source) {
  try {
    return createMermaidWardleyLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes xychart [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidXychart(String source) {
  try {
    return createMermaidXychartLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}

/// Tokenizes zenuml [source], returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `try_tokenize` with idiomatic Dart nullability.
List<Token>? tryTokenizeMermaidZenuml(String source) {
  try {
    return createMermaidZenumlLexer(source).tokenize();
  } on LexerError {
    return null;
  }
}
