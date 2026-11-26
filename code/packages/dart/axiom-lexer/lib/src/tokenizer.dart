/// Axiom tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots. Token types keep their grammar
/// names verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the Axiom grammar.
GrammarLexer createAxiomLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeAxiom(String source) => createAxiomLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_axiom`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeAxiom(String source) {
  try {
    return tokenizeAxiom(source);
  } on LexerError {
    return null;
  }
}
