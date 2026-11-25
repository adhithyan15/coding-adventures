/// Derive tokenization over the compiled grammar: the factory,
/// the throwing and nullable one-shots, and the bracket-interior
/// newline hook. Token types keep their grammar names verbatim; the
/// final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Drops `NEWLINE` tokens inside an open `(` or `[` — the contents of
/// a grouping/call or a vector/matrix literal may legally span
/// several physical lines. Tracks the combined depth (Derive has no
/// `{ }` and no `[[ ]]` part-sugar, unlike Wolfram) and saturates at
/// zero on an unbalanced closer, mirroring the reference's
/// `saturating_sub`.
List<Token> dropBracketedNewlines(List<Token> tokens) {
  final result = <Token>[];
  var depth = 0;
  for (final token in tokens) {
    if (token.type == 'NEWLINE' && depth > 0) {
      continue;
    }
    if (token.type == 'LPAREN' || token.type == 'LBRACKET') {
      depth += 1;
    } else if (token.type == 'RPAREN' || token.type == 'RBRACKET') {
      if (depth > 0) depth -= 1;
    }
    result.add(token);
  }
  return result;
}

/// Creates a [GrammarLexer] for [source] with the Derive grammar and
/// the bracket-interior newline hook registered.
GrammarLexer createDeriveLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPostTokenize(dropBracketedNewlines);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeDerive(String source) =>
    createDeriveLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_derive`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeDerive(String source) {
  try {
    return tokenizeDerive(source);
  } on LexerError {
    return null;
  }
}
