/// APL parsing over the compiled grammar: the factory and the
/// throwing and nullable one-shots. The tree is rooted at `program`.
library;

import 'package:coding_adventures_apl_lexer/apl_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the APL [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 100: parenthesised
/// nesting parses cleanly up to 47 levels (48 trips the cap), and
/// flat dyadic chains up to 94 terms (95 trips it) — the chain shape
/// being the binding constraint the value was corrected against.
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so both measured boundaries hold
/// here too.
const int maxRuleDepth = 100;

/// Creates a [GrammarParser] wired to the APL grammar and the tokens
/// of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createAplParser(String source) {
  final tokens = tokenizeApl(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses APL source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
ASTNode parseApl(String source) => createAplParser(source).parse();

/// Parses APL source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_apl` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseApl(String source) {
  try {
    return parseApl(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
