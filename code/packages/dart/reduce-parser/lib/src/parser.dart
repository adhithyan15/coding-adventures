/// Reduce parsing over the compiled grammar: the factory, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';
import 'package:coding_adventures_reduce_lexer/reduce_lexer.dart';

import '_grammar.dart';

/// Recursion-depth cap for the Reduce [GrammarParser].
///
/// Mirrors the reference's independently measured `MAX_RULE_DEPTH`
/// of 128 (which happens to equal the shared default): the five
/// recursion shapes (parens, `:=` chains, `if`/`else` chains, cons
/// chains, power chains) trip the cap at 9, 57, 57, 113, and 57
/// levels respectively — re-measured on this engine and pinned by
/// test — instead of overflowing the stack.
const int maxRuleDepth = 128;

/// Creates a [GrammarParser] wired to the Reduce grammar and the
/// tokens of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createReduceParser(String source) => GrammarParser(
      tokenizeReduce(source),
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Parses Reduce source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference panics with
/// the message; Dart lanes propagate the structured errors instead).
ASTNode parseReduce(String source) => createReduceParser(source).parse();

/// Parses Reduce source text, returning `null` instead of throwing.
/// The fully fallible path — a lexical error becomes `null` too (it
/// routes through [tryTokenizeReduce], not the throwing tokenizer
/// that [createReduceParser] uses), mirroring the reference's
/// `Result`-returning `try_parse_reduce` with idiomatic Dart
/// nullability (catch [LexerError] or [GrammarParseError] for the
/// reason).
ASTNode? tryParseReduce(String source) {
  final tokens = tryTokenizeReduce(source);
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
