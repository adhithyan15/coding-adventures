/// R parsing over the compiled grammar: the factory and the
/// throwing and nullable one-shots. The tree is rooted at `program`;
/// rule names match `s.grammar` so the shared S evaluator can walk R
/// trees unchanged.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_r_lexer/r_lexer.dart';

import '_grammar.dart';

/// Recursion-depth cap for the R [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 200 (~33% below its
/// 297-rule-frame floor; 14 nesting levels parse, 15 trips the
/// guard).
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so the measured boundary holds
/// here too.
const int maxRuleDepth = 200;

/// Creates a [GrammarParser] wired to the R grammar and the tokens of
/// [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createRParser(String source) {
  final tokens = tokenizeR(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses R source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
ASTNode parseR(String source) => createRParser(source).parse();

/// Parses R source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_r` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseR(String source) {
  try {
    return parseR(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
