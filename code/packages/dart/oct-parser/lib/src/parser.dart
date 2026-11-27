/// Oct parsing over the compiled grammar: the factories, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_oct_lexer/oct_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the Oct [GrammarParser].
///
/// Mirrors the reference's measured `MAX_RULE_DEPTH` of 200: the
/// precedence cascade costs slightly fewer rule frames per level
/// than the sibling Nib grammar, so the engine refuses cleanly once
/// nesting passes the measured boundary (20 levels parse, 21 trip
/// the cap — re-measured on this engine and pinned by test) instead
/// of overflowing the stack on inputs like
/// `fn main() { let x: u8 = (((…))); }`.
const int maxRuleDepth = 200;

/// Creates a [GrammarParser] over an Oct source string, with the
/// recursion-depth guard enabled so pathologically deep nesting
/// fails cleanly instead of overflowing the stack. Most callers want
/// [parseOct] instead.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createOctParser(String source) =>
    createOctParserFromTokens(tokenizeOct(source));

/// Creates a [GrammarParser] over already-produced tokens, with the
/// same grammar and the same [maxRuleDepth] guard that
/// [createOctParser] uses.
///
/// Exists for preprocessor dialects (MacroOct, PREP01): they lex
/// with their own `.tokens` grammar, consume the directive tokens,
/// and parse the resulting pure Oct stream with Oct's grammar —
/// without embedding a second compiled copy that could drift.
GrammarParser createOctParserFromTokens(List<Token> tokens) => GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Parses an Oct source string into a grammar AST rooted at
/// `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference returns
/// `Err`; Dart lanes throw the structured errors instead, with
/// [tryParseOct] as the `Result`-shaped twin).
ASTNode parseOct(String source) => createOctParser(source).parse();

/// Parses an Oct source string, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `parse_oct` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseOct(String source) {
  try {
    return parseOct(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
