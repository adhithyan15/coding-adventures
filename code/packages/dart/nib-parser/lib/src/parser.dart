/// Nib parsing over the compiled grammar: the factories, the
/// throwing source and token-input one-shots, and their nullable
/// twins. The tree is rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_nib_lexer/nib_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the Nib [GrammarParser].
///
/// Mirrors the reference's measured `MAX_RULE_DEPTH` of 200: the
/// precedence cascade burns several rule frames per source-nesting
/// level, so the engine refuses cleanly once nesting passes the
/// measured boundary (16 levels parse, 17 trip the cap — re-measured
/// on this engine and pinned by test) instead of overflowing the
/// stack on inputs like `fn main() { let x: u4 = (((…))); }`.
const int maxRuleDepth = 200;

/// Creates a [GrammarParser] wired to the Nib grammar and the tokens
/// of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createNibParser(String source) =>
    createNibParserFromTokens(tokenizeNib(source));

/// Creates the guarded Nib parser over an already-tokenized stream.
///
/// A preprocessing dialect can pass its directive-free tokens here
/// without copying Nib's compiled grammar or weakening the depth
/// limit.
GrammarParser createNibParserFromTokens(List<Token> tokens) => GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Parses Nib source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference returns
/// `Err`; Dart lanes throw the structured errors instead, with
/// [tryParseNib] as the `Result`-shaped twin).
ASTNode parseNib(String source) => createNibParser(source).parse();

/// Parses Nib source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `parse_nib` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseNib(String source) {
  try {
    return parseNib(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}

/// Parses pre-tokenized Nib syntax with the same grammar and depth
/// guard as [parseNib].
/// Throws [GrammarParseError] on empty input, syntax errors, or
/// depth-cap trips. The empty check is this library's own: the
/// engine indexes the token list directly, so an empty stream would
/// otherwise escape as an undocumented `StateError` instead of the
/// documented error vocabulary.
ASTNode parseNibTokens(List<Token> tokens) {
  if (tokens.isEmpty) {
    throw GrammarParseError('Nib token input is empty');
  }
  return createNibParserFromTokens(tokens).parse();
}

/// Parses pre-tokenized Nib syntax, returning `null` instead of
/// throwing. Mirrors the reference's `Result`-returning
/// `parse_nib_tokens` with idiomatic Dart nullability.
ASTNode? tryParseNibTokens(List<Token> tokens) {
  try {
    return parseNibTokens(tokens);
  } on GrammarParseError {
    return null;
  }
}
