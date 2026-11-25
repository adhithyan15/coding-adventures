/// S parsing over the compiled grammar: the factory and the
/// throwing and nullable one-shots. The tree is rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_s_lexer/s_lexer.dart';

import '_grammar.dart';

/// Recursion-depth cap for the S [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 200.
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so the measured boundary holds
/// here too.
const int maxRuleDepth = 200;

/// Creates a [GrammarParser] wired to the S grammar and the tokens of
/// [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createSParser(String source) {
  final tokens = tokenizeS(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses S source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
ASTNode parseS(String source) => createSParser(source).parse();

/// Parses S source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_s` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseS(String source) {
  try {
    return parseS(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
