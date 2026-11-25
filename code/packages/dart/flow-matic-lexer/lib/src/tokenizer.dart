/// FLOW-MATIC tokenization over the compiled grammar: the factory,
/// the throwing and nullable one-shots. Token types keep their grammar
/// names verbatim; the final token is always `EOF`. Keywords match
/// case-insensitively and fold to uppercase values while NAMEs keep
/// their case.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the FLOW-MATIC grammar.
GrammarLexer createFlowMaticLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeFlowMatic(String source) =>
    createFlowMaticLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_flow_matic`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeFlowMatic(String source) {
  try {
    return tokenizeFlowMatic(source);
  } on LexerError {
    return null;
  }
}
