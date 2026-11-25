/// CSS parsing over the compiled grammar: the factory and the
/// one-shot entry point. The tree is rooted at `stylesheet`.
library;

import 'package:coding_adventures_css_lexer/css_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the CSS [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 170 (~31% below the
/// binding floor across the five measured nesting shapes).
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so every measured boundary holds
/// here too.
const int maxRuleDepth = 170;

/// Creates a [GrammarParser] wired to the CSS grammar and the tokens
/// of [source], with the recursion-depth guard enabled so
/// pathologically deep nesting fails cleanly instead of overflowing
/// the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createCssParser(String source) {
  final tokens = tokenizeCss(source);
  return GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
}

/// Parses CSS source text into an [ASTNode] rooted at `stylesheet`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
///
/// Like the reference, this lane offers no `Result`/nullable twin:
/// use [createCssParser] + `parse()` when the call site needs to
/// handle errors explicitly.
ASTNode parseCss(String source) => createCssParser(source).parse();
