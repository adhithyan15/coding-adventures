/// F# parsing over the compiled versioned grammars: version
/// resolution, the factory, and the throwing and nullable one-shots.
/// Every tree is rooted at `compilation_unit`.
library;

import 'package:coding_adventures_fsharp_lexer/fsharp_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// The F# versions this lane can parse, in reference order.
/// (Kept separate from the lexer's list, exactly like the reference
/// keeps its own `SUPPORTED_VERSIONS`.)
const List<String> supportedFsharpParserVersions = [
  '1.0',
  '2.0',
  '3.0',
  '3.1',
  '4.0',
  '4.1',
  '4.5',
  '4.6',
  '4.7',
  '5',
  '6',
  '7',
  '8',
  '9',
  '10',
];

/// Recursion-depth cap for the F# [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 200 (safe at 289, ~31%
/// below the native-stack floor, for ordinary parenthesised
/// grouping — the one recursion shape measured so far).
const int maxRuleDepth = 200;

/// Resolves [version] to a supported version: empty means the
/// lexer's [defaultFsharpVersion], anything unsupported throws an
/// [ArgumentError] carrying the reference's exact message (the
/// reference returns `Err(String)` there instead).
String resolveFsharpParserVersion(String version) {
  final resolved = version.isEmpty ? defaultFsharpVersion : version;
  if (supportedFsharpParserVersions.contains(resolved)) {
    return resolved;
  }
  final valid = supportedFsharpParserVersions.map((v) => '"$v"').join(', ');
  throw ArgumentError(
    "Unknown F# version '$version'. Valid values: $valid",
  );
}

/// Creates a [GrammarParser] for [source] under the [version] grammar
/// (empty selects the default), with the recursion-depth guard
/// enabled so pathologically deep nesting fails cleanly instead of
/// overflowing the stack.
/// Throws [ArgumentError] for an unknown version and [LexerError] on
/// unrecognized input (the reference returns `Err(String)` for
/// both).
GrammarParser createFsharpParser(String source, [String version = '']) {
  final resolved = resolveFsharpParserVersion(version);
  final tokens = tokenizeFsharp(source, resolved);
  return GrammarParser(
    tokens,
    parserGrammarForVersion(resolved),
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses F# [source] under the [version] grammar (empty selects the
/// default) into an [ASTNode] rooted at `compilation_unit`.
/// Throws [ArgumentError] for an unknown version, [LexerError] on
/// unrecognized input, and [GrammarParseError] on grammar violations
/// (the reference wraps the latter in a String; Dart lanes propagate
/// the structured error instead).
ASTNode parseFsharp(String source, [String version = '']) =>
    createFsharpParser(source, version).parse();

/// Parses F# [source], returning `null` instead of throwing on any
/// failure (unknown version, lexing, or syntax). Mirrors the
/// reference's `Result`-returning `parse_fsharp` with idiomatic Dart
/// nullability.
ASTNode? tryParseFsharp(String source, [String version = '']) {
  try {
    return parseFsharp(source, version);
  } on ArgumentError {
    return null;
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
