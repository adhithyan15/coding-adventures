/// COBOL-60 parsing over the compiled grammar: the factory, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program`.
library;

import 'package:coding_adventures_cobol_lexer/cobol_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the COBOL [GrammarParser].
///
/// Mirrors the reference's shared `DEFAULT_MAX_RULE_DEPTH` of 128:
/// deeply-nested syntax (hundreds of nested `IF … IF … IF …`)
/// recurses once per level through the engine, so the cap surfaces
/// a recoverable [GrammarParseError] instead of overflowing the
/// stack.
const int maxRuleDepth = 128;

/// Creates a [GrammarParser] wired to the COBOL grammar and the
/// tokens of [source] (with the lexer's column-strip hook), ready to
/// call `.parse()`. Uses the throwing tokenizer; for the fully
/// fallible path use [tryParseCobol].
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createCobolParser(String source) => GrammarParser(
      tokenizeCobol(source),
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Parses COBOL-60 [source] into an [ASTNode] CST rooted at
/// `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference panics with
/// the message; Dart lanes propagate the structured errors instead).
ASTNode parseCobol(String source) => createCobolParser(source).parse();

/// Parses COBOL-60 [source], returning `null` instead of throwing.
/// The fully fallible path — a lexical error becomes `null` too (it
/// routes through [tryTokenizeCobol], not the throwing tokenizer
/// that [createCobolParser] uses), mirroring the reference's
/// `Result`-returning `try_parse_cobol` with idiomatic Dart
/// nullability (catch [LexerError] or [GrammarParseError] for the
/// reason).
ASTNode? tryParseCobol(String source) {
  final tokens = tryTokenizeCobol(source);
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
