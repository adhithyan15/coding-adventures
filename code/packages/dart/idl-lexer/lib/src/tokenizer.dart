/// IDL tokenization over the compiled grammar: the factory, the
/// throwing one-shot, and the nullable one-shot. Token types keep their
/// grammar names verbatim; the final token is always `EOF`. Keywords
/// fold to uppercase values (`if` → `KEYWORD("IF")`) while NAMEs keep
/// their case, per the grammar's case-insensitive lookup.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the IDL grammar.
GrammarLexer createIdlLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeIdl(String source) => createIdlLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_idl` with
/// idiomatic Dart nullability (catch [LexerError] for the message).
List<Token>? tryTokenizeIdl(String source) {
  try {
    return tokenizeIdl(source);
  } on LexerError {
    return null;
  }
}
