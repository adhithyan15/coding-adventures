/// C tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots, and the cached keyword predicate.
/// Token types keep their grammar names verbatim; the final token is
/// always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the C grammar.
GrammarLexer createCLexer(String source) => GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeC(String source) => createCLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_c` with
/// idiomatic Dart nullability (catch [LexerError] for the message).
List<Token>? tryTokenizeC(String source) {
  try {
    return tokenizeC(source);
  } on LexerError {
    return null;
  }
}

/// Whether the compiled C grammar reserves [word] as a keyword.
/// Built once from the same generated grammar the lexer uses, so macro
/// expansion can call it per token without rebuilding anything.
final _keywords = <String>{
  ...tokenGrammar.keywords,
  ...tokenGrammar.reservedKeywords,
};

bool isCKeyword(String word) => _keywords.contains(word);
