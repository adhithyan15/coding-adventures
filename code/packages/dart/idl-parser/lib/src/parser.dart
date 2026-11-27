/// IDL parsing over the compiled grammar: the factory, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program`.
library;

import 'package:coding_adventures_idl_lexer/idl_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the IDL [GrammarParser].
///
/// Mirrors the reference's independently measured `MAX_RULE_DEPTH`
/// of 148: the six recursion shapes (parens, nested `IF`s, call
/// args, subscripts, unary chains, array literals) trip the cap at
/// 14, 27, 13, 12, 135, and 13 levels respectively — re-measured on
/// this engine and pinned by test — instead of overflowing the
/// stack.
const int maxRuleDepth = 148;

/// Builds a [GrammarParser] wired to the IDL grammar for the
/// already-tokenized [tokens], with the recursion-depth guard
/// enabled. The ONE place that constructs a capped IDL
/// [GrammarParser] — [createIdlParser] and [tryParseIdl] both funnel
/// through this, so a future change to the cap (or to how the
/// parser is wired up) can never miss one of the two call sites.
GrammarParser _cappedIdlParser(List<Token> tokens) => GrammarParser(
      tokens,
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Creates a [GrammarParser] wired to the IDL grammar and the tokens
/// of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createIdlParser(String source) =>
    _cappedIdlParser(tokenizeIdl(source));

/// Parses IDL source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference panics with
/// the message; Dart lanes propagate the structured errors instead).
ASTNode parseIdl(String source) => createIdlParser(source).parse();

/// Parses IDL source text, returning `null` instead of throwing.
/// The fully fallible path — a lexical error becomes `null` too (it
/// routes through [tryTokenizeIdl], not the throwing tokenizer that
/// [createIdlParser] uses), mirroring the reference's
/// `Result`-returning `try_parse_idl` with idiomatic Dart nullability
/// (catch [LexerError] or [GrammarParseError] for the reason).
ASTNode? tryParseIdl(String source) {
  final tokens = tryTokenizeIdl(source);
  if (tokens == null) return null;
  try {
    return _cappedIdlParser(tokens).parse();
  } on GrammarParseError {
    return null;
  }
}
