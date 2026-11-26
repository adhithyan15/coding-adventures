/// S tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots, and the bracket-interior newline
/// hook. Token types keep their grammar names verbatim; the final
/// token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Drops `NEWLINE` tokens inside open `(` or `[` (but not `{`).
/// (Identical to the R rule.)
List<Token> dropBracketedNewlines(List<Token> tokens) {
  final result = <Token>[];
  var depth = 0;
  for (final token in tokens) {
    if (token.type == 'NEWLINE' && depth > 0) {
      continue;
    }
    switch (token.type) {
      case 'LPAREN':
      case 'LBRACKET':
        depth += 1;
      case 'RPAREN':
      case 'RBRACKET':
        depth -= 1;
      default:
        break;
    }
    result.add(token);
  }
  return result;
}

/// Creates a [GrammarLexer] for [source] with the S grammar and the
/// bracket-interior newline hook registered.
GrammarLexer createSLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPostTokenize(dropBracketedNewlines);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeS(String source) => createSLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_s` with
/// idiomatic Dart nullability (catch [LexerError] for the message).
List<Token>? tryTokenizeS(String source) {
  try {
    return tokenizeS(source);
  } on LexerError {
    return null;
  }
}
