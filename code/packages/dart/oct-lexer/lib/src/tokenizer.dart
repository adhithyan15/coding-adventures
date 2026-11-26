/// Oct tokenization over the compiled grammar: the factory and the
/// one-shot entry point. Token types keep their grammar names verbatim
/// (except promoted keywords, see below); the final token is always
/// `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Creates a [GrammarLexer] for [source] with the Oct grammar.
GrammarLexer createOctLexer(String source) =>
    GrammarLexer(source, tokenGrammar);

/// Tokenizes [source] in one shot, then promotes every `KEYWORD`
/// token's type to its literal value — the same convention as Nib's
/// lexer, so quoted-keyword grammar rules match by type name.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeOct(String source) {
  final tokens = createOctLexer(source).tokenize();
  return [
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
}
