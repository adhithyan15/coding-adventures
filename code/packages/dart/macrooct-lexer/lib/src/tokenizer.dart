/// MacroOct tokenization over the compiled grammar: the factory,
/// the throwing and nullable one-shots, and the keyword-promotion
/// post-pass. Token types keep their grammar names verbatim (except
/// promoted keywords, see below); the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the MacroOct grammar.
///
/// Like the reference factory, this returns the RAW engine: the
/// keyword-promotion post-pass runs only in [tokenizeMacrooct] and
/// [tryTokenizeMacrooct], never here.
GrammarLexer createMacrooctLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// The reference's post-pass: every `KEYWORD` token's type becomes its
/// literal value (`KEYWORD("fn")` becomes `fn("fn")`). Identical to
/// the `oct` lane's loop, and it has to be: Oct's parser grammar
/// matches keywords by type name, and MacroOct hands its preprocessed
/// stream to that same grammar.
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
/// Dart lanes propagate the structured error instead). Anything
/// reachable from untrusted input (like an `@include`d file's text)
/// should use [tryTokenizeMacrooct].
List<Token> tokenizeMacrooct(String source) =>
    _promoteKeywords(createMacrooctLexer(source).tokenize());

/// Tokenizes [source], reporting a lex failure as `null` instead of
/// throwing — the preprocessor engine's no-panic contract reaches
/// through to here, because an included file's text is chosen by the
/// program being compiled. Mirrors the reference's
/// `Result`-returning `try_tokenize_macrooct` with idiomatic Dart
/// nullability (catch [LexerError] for the message).
List<Token>? tryTokenizeMacrooct(String source) {
  try {
    return tokenizeMacrooct(source);
  } on LexerError {
    return null;
  }
}
