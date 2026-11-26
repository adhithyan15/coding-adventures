/// Wolfram tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots, and the bracket-interior newline
/// hook. Token types keep their grammar names verbatim; the final
/// token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Drops `NEWLINE` tokens inside an open `(`, `[`, `{`, or `[[`.
///
/// A newline ends a top-level statement, but group contents, `f[…]`
/// applications, `{…}` lists, and `[[ … ]]` part expressions may span
/// lines. `[[` is one token but two nesting levels (closed by two
/// single `]`), so it adds 2 while each `]` subtracts 1.
List<Token> dropBracketedNewlines(List<Token> tokens) {
  final result = <Token>[];
  var depth = 0;
  for (final token in tokens) {
    if (token.type == 'NEWLINE' && depth > 0) {
      continue;
    }
    if (token.type == 'LDBRACKET') {
      depth += 2;
    } else {
      switch (token.type) {
        case 'LPAREN':
        case 'LBRACKET':
        case 'LBRACE':
          depth += 1;
        case 'RPAREN':
        case 'RBRACKET':
        case 'RBRACE':
          depth -= 1;
        default:
          break;
      }
    }
    result.add(token);
  }
  return result;
}

/// Creates a [GrammarLexer] for [source] with the Wolfram grammar and
/// the bracket-interior newline hook registered.
GrammarLexer createWolframLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPostTokenize(dropBracketedNewlines);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeWolfram(String source) =>
    createWolframLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_wolfram`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeWolfram(String source) {
  try {
    return tokenizeWolfram(source);
  } on LexerError {
    return null;
  }
}
