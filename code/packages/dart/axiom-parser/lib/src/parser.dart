/// Axiom parsing over the compiled grammar: the factory, the
/// throwing source one-shot, and its nullable twin. The tree is
/// rooted at `program` (exactly one top-level expression — the
/// tokens give top-level inputs no separator).
library;

import 'package:coding_adventures_axiom_lexer/axiom_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the Axiom [GrammarParser].
///
/// Mirrors the reference's independently measured `MAX_RULE_DEPTH`
/// of 140: the four recursion shapes (parens, nested calls, unary
/// prefix chains, power chains) trip the cap at 14, 14, 131, and 66
/// levels respectively — re-measured on this engine and pinned by
/// test — instead of overflowing the stack.
const int maxRuleDepth = 140;

/// Creates a [GrammarParser] wired to the Axiom grammar and the
/// tokens of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createAxiomParser(String source) => GrammarParser(
      tokenizeAxiom(source),
      parserGrammar,
      options: const GrammarParserOptions(maxDepth: maxRuleDepth),
    );

/// Parses Axiom source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors or depth-cap trips (the reference panics with
/// the message; Dart lanes propagate the structured errors instead).
ASTNode parseAxiom(String source) => createAxiomParser(source).parse();

/// Parses Axiom source text, returning `null` instead of throwing.
/// The fully fallible path — a lexical error becomes `null` too (it
/// routes through [tryTokenizeAxiom], not the throwing tokenizer
/// that [createAxiomParser] uses), mirroring the reference's
/// `Result`-returning `try_parse_axiom` with idiomatic Dart
/// nullability (catch [LexerError] or [GrammarParseError] for the
/// reason).
ASTNode? tryParseAxiom(String source) {
  final tokens = tryTokenizeAxiom(source);
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
