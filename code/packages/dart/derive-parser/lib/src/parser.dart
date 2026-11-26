/// Derive parsing over the compiled grammar: the factory and the
/// throwing and nullable one-shots. The tree is rooted at `program`;
/// rule names (`assignment`, `logical_or`, …) are the surface tree a
/// future runtime lowers.
library;

import 'package:coding_adventures_derive_lexer/derive_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the Derive [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 200: one layer of `(...)`
/// grouping loops back through the entire 12-rule precedence cascade,
/// and 200 sits ~33% below the 297-rule-frame floor the reference
/// measured against candidate cap values on adversarial input.
///
/// The cap counts rule frames identically in both engines (same
/// grammar, same counting), so the 14/15 measured boundary holds
/// here too: 14 nesting levels parse, 15 trips the guard.
const int maxRuleDepth = 200;

/// Creates a [GrammarParser] wired to the Derive grammar and the
/// tokens of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createDeriveParser(String source) {
  final tokens = tokenizeDerive(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses Derive source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
ASTNode parseDerive(String source) => createDeriveParser(source).parse();

/// Parses Derive source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_derive`
/// with idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseDerive(String source) {
  try {
    return parseDerive(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
