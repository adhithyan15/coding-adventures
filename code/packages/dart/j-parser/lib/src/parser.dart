/// J parsing over the compiled grammar: the factory and the
/// throwing and nullable one-shots. The tree is rooted at `program`.
library;

import 'package:coding_adventures_j_lexer/j_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the J [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 70: parenthesised nesting
/// parses cleanly up to 32 levels (33 trips the cap), flat dyadic
/// chains up to 63 terms (64 trips it), and trains up to 61 teeth (62
/// trips it) — the paren shape binding at ~30% below its measured
/// native-stack floor.
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so all three measured boundaries
/// hold here too.
const int maxRuleDepth = 70;

/// Creates a [GrammarParser] wired to the J grammar and the tokens of
/// [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createJParser(String source) {
  final tokens = tokenizeJ(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses J source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
ASTNode parseJ(String source) => createJParser(source).parse();

/// Parses J source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_j` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseJ(String source) {
  try {
    return parseJ(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
