/// MATLAB parsing over the compiled grammar: the factory and the
/// throwing and nullable one-shots. The tree is rooted at `program`.
library;

import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_matlab_lexer/matlab_lexer.dart';
import 'package:coding_adventures_parser/parser.dart';

import '_grammar.dart';

/// Recursion-depth cap for the MATLAB [GrammarParser], mirroring the
/// reference's measured `MAX_RULE_DEPTH` of 200 (12 nesting levels
/// parse, 13 trips the guard).
///
/// The cap counts rule frames identically in both engines (same
/// grammar cascade, same counting), so the measured boundary holds
/// here too.
const int maxRuleDepth = 200;

/// Rewrites each `end` keyword that occurs inside `( )`, `[ ]`, or
/// `{ }` into a `NAME` token, leaving depth-0 `end`s (block
/// terminators) untouched. Tracks the combined bracket depth across
/// all three bracket kinds, saturating at zero on unbalanced closers.
///
/// `end` is both a block terminator (`if … end`) and the last-index
/// sentinel (`A(end)`): the sentinel reaches the parser as an
/// ordinary `NAME` thanks to this pre-parse hook, so the two uses
/// never collide.
List<Token> retagIndexEnd(List<Token> tokens) {
  final result = <Token>[];
  var depth = 0;
  for (final token in tokens) {
    if (token.type == 'LPAREN' ||
        token.type == 'LBRACKET' ||
        token.type == 'LBRACE') {
      depth += 1;
    } else if (token.type == 'RPAREN' ||
        token.type == 'RBRACKET' ||
        token.type == 'RBRACE') {
      if (depth > 0) depth -= 1;
    }
    if (depth > 0 && token.type == 'KEYWORD' && token.value == 'end') {
      result.add(Token(
        type: 'NAME',
        value: token.value,
        line: token.line,
        column: token.column,
        flags: token.flags,
      ));
    } else {
      result.add(token);
    }
  }
  return result;
}

/// Creates a [GrammarParser] wired to the MATLAB grammar and the
/// tokens of [source], with the `end`-retag hook installed and the
/// recursion-depth guard enabled so pathologically deep nesting
/// fails cleanly instead of overflowing the stack.
/// Throws [LexerError] when [source] does not lex (the reference
/// factory panics there too).
GrammarParser createMatlabParser(String source) {
  final tokens = tokenizeMatlab(source);
  final parser = GrammarParser(
    tokens,
    parserGrammar,
    options: const GrammarParserOptions(maxDepth: maxRuleDepth),
  );
  parser.addPreParse(retagIndexEnd);
  return parser;
}

/// Parses MATLAB source text into an [ASTNode] rooted at `program`.
/// Throws [LexerError] on unrecognized input and [GrammarParseError]
/// on syntax errors (the reference panics on both; Dart lanes
/// propagate the structured errors instead).
ASTNode parseMatlab(String source) => createMatlabParser(source).parse();

/// Parses MATLAB source text, returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_parse_matlab` with
/// idiomatic Dart nullability (catch [LexerError] or
/// [GrammarParseError] for the reason).
ASTNode? tryParseMatlab(String source) {
  try {
    return parseMatlab(source);
  } on LexerError {
    return null;
  } on GrammarParseError {
    return null;
  }
}
