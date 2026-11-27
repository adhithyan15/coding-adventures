/// MACSYMA parsing over the compiled grammar: the factory, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_macsyma_lexer/macsyma_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the MACSYMA [GrammarParser].
///
/// Mirrors the reference's measured `MAX_RULE_DEPTH` of 200: the
/// 13-rule precedence cascade (`expression -> assign -> ... ->
/// group -> expression`) burns ~13 rule frames per source-nesting
/// level, so the engine refuses cleanly once nesting passes the
/// measured boundary (14 levels parse, 15 trip the cap — re-measured
/// on this engine and pinned by test) instead of overflowing the
/// stack on inputs like `((((…))))`.
const int maxRuleDepth = 200;

/// Creates a [GrammarParser] wired to the MACSYMA grammar and the
/// tokens of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createMacsymaParser(String source) {
  final tokens = tokenizeMacsyma(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses MACSYMA source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference panics with the
/// message; Dart lanes propagate the structured errors instead).
ASTNode parseMacsyma(String source) => createMacsymaParser(source).parse();

/// Parses MACSYMA source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning shape with idiomatic
/// Dart nullability (catch [LexerError] or [GrammarParseError] for
/// the reason).
ASTNode? tryParseMacsyma(String source) {
  try {
    return parseMacsyma(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
