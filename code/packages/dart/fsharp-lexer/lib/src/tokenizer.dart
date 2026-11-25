/// F# tokenization over the compiled versioned grammars: version
/// resolution, the factory, and the one-shot entry point. Token
/// types keep their grammar names verbatim; the final token is always
/// `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// The F# versions this lane can tokenize, in reference order.
const List<String> supportedFsharpVersions = [
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

/// The version used when the caller passes an empty version string,
/// mirroring the reference's `DEFAULT_VERSION`.
const String defaultFsharpVersion = '10';

/// Resolves [version] to a supported version: empty means
/// [defaultFsharpVersion], anything unsupported throws an
/// [ArgumentError] carrying the reference's exact message (the
/// reference returns `Err(String)` there instead).
String resolveFsharpVersion(String version) {
  final resolved = version.isEmpty ? defaultFsharpVersion : version;
  if (supportedFsharpVersions.contains(resolved)) {
    return resolved;
  }
  final valid = supportedFsharpVersions.map((v) => '"$v"').join(', ');
  throw ArgumentError(
    "Unknown F# version '$version'. Valid values: $valid",
  );
}

/// Creates a [GrammarLexer] for [source] under the [version] grammar
/// (empty selects [defaultFsharpVersion]).
/// Throws [ArgumentError] for an unknown version (the reference
/// returns `Err(String)` there too).
GrammarLexer createFsharpLexer(String source, [String version = '']) {
  return GrammarLexer(
      source, tokenGrammarForVersion(resolveFsharpVersion(version)));
}

/// Tokenizes F# [source] under the [version] grammar (empty selects
/// [defaultFsharpVersion]) in one shot.
/// Throws [ArgumentError] for an unknown version and [LexerError] on
/// unrecognized input (the reference wraps the latter in a String;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeFsharp(String source, [String version = '']) =>
    createFsharpLexer(source, version).tokenize();
