/// Maple tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots. Token types keep their grammar
/// names verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the Maple grammar.
GrammarLexer createMapleLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeMaple(String source) => createMapleLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_maple`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeMaple(String source) {
  try {
    return tokenizeMaple(source);
  } on LexerError {
    return null;
  }
}
