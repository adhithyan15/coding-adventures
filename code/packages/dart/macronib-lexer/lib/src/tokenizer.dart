/// MacroNib tokenization over the compiled grammar: the factory,
/// the throwing and nullable one-shots, and the keyword-promotion
/// post-pass. Token types keep their grammar names verbatim (except
/// promoted keywords, see below); the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the MacroNib grammar.
///
/// Like the reference factory, this returns the RAW engine: the
/// keyword-promotion post-pass runs only in [tokenizeMacronib] and
/// [tryTokenizeMacronib], never here.
GrammarLexer createMacronibLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// The reference's post-pass: every `KEYWORD` token's type becomes its
/// literal value (`KEYWORD("fn")` becomes `fn("fn")`).
List<Token> _promoteKeywords(List<Token> tokens) => [
      for (final token in tokens)
        if (token.type == 'KEYWORD')
          Token(
            type: token.value,
            value: token.value,
            line: token.line,
            column: token.column,
            flags: token.flags,
          )
        else
          token,
    ];

/// Tokenizes [source] in one shot, then promotes every `KEYWORD`
/// token's type to its literal value, exactly like the reference's
/// post-pass.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeMacronib(String source) =>
    _promoteKeywords(createMacronibLexer(source).tokenize());

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_macronib`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeMacronib(String source) {
  try {
    return tokenizeMacronib(source);
  } on LexerError {
    return null;
  }
}
