/// Maple parsing over the compiled grammar: the factory, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_maple_lexer/maple_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the Maple [GrammarParser].
///
/// Mirrors the reference's independently measured `MAX_RULE_DEPTH`
/// of 150: the six recursion shapes (parens, list/set literals,
/// `not` chains, unary-minus chains, power chains, nested `if`s)
/// trip the cap at 12, 11, 136, 136, 68, and 68 levels respectively
/// — re-measured on this engine and pinned by test — instead of
/// overflowing the stack.
const int maxRuleDepth = 150;

/// Creates a [GrammarParser] wired to the Maple grammar and the
/// tokens of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createMapleParser(String source) => GrammarParser(
      tokenizeMaple(source),
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Parses Maple source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference panics with
/// the message; Dart lanes propagate the structured errors instead).
ASTNode parseMaple(String source) => createMapleParser(source).parse();

/// Parses Maple source text, returning `null` instead of throwing.
/// The fully fallible path — a lexical error becomes `null` too (it
/// routes through [tryTokenizeMaple], not the throwing tokenizer
/// that [createMapleParser] uses), mirroring the reference's
/// `Result`-returning `try_parse_maple` with idiomatic Dart
/// nullability (catch [LexerError] or [GrammarParseError] for the
/// reason).
ASTNode? tryParseMaple(String source) {
  final tokens = tryTokenizeMaple(source);
  if (tokens == null) return null;
  try {
    return GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    ).parse();
  } on GrammarParseError {
    return null;
  }
}
