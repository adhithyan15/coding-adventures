/// Q tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots, and the two hooks — comment
/// stripping (pre) and negative-literal folding (post). Token types
/// keep their grammar names verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Blanks out `/`-to-end-of-line comments before the engine ever sees
/// the source text.
///
/// A `/` opens a comment when preceded by whitespace or at line start
/// (including start of input); glued to a non-whitespace character it
/// stays the ordinary `REDUCE` token. Comment characters become spaces
/// one-for-one (never deleted), so every other character's line/column
/// is unaffected; the terminating newline itself is left alone because
/// it is its own significant `NEWLINE` token.
String stripSlashComments(String source) {
  final chars = source.runes.toList();
  final out = StringBuffer();
  var i = 0;
  var prevIsWhitespaceLike = true;
  bool isWhitespaceLike(int rune) =>
      rune == 0x20 || rune == 0x09 || rune == 0x0D || rune == 0x0A;
  while (i < chars.length) {
    final c = chars[i];
    if (c == 0x2F && prevIsWhitespaceLike) {
      while (i < chars.length && chars[i] != 0x0A) {
        out.write(' ');
        i++;
      }
      prevIsWhitespaceLike = true;
      continue;
    }
    out.writeCharCode(c);
    prevIsWhitespaceLike = isWhitespaceLike(c);
    i++;
  }
  return out.toString();
}

/// Type names that complete a noun — a value a directly-glued `-`
/// could plausibly subtract from.
bool endsANoun(String effectiveTypeName) {
  switch (effectiveTypeName) {
    case 'NUMBER':
    case 'NAME':
    case 'RPAREN':
    case 'RBRACKET':
    case 'RBRACE':
      return true;
    default:
      return false;
  }
}

/// The column immediately after [token] (single-line tokens only,
/// which is every token this grammar can produce).
int endColumn(Token token) => token.column + token.value.runes.length;

/// Folds a `MINUS` immediately followed by a glued `NUMBER` into one
/// signed `NUMBER` token, when the position may start a new
/// list-stranding element: the previously emitted token is absent, not
/// noun-ending, or noun-ending but not glued to this `MINUS`.
///
/// An un-folded `MINUS` still becomes the previous token for the next
/// iteration, so chains like `1 - -1` resolve each `-` independently.
List<Token> foldNegativeNumberLiterals(List<Token> tokens) {
  final out = <Token>[];
  var i = 0;
  while (i < tokens.length) {
    final tok = tokens[i];
    if (tok.type == 'MINUS' && i + 1 < tokens.length) {
      final next = tokens[i + 1];
      final gluedToDigit = next.type == 'NUMBER' &&
          next.line == tok.line &&
          next.column == endColumn(tok);
      final prev = out.isEmpty ? null : out.last;
      final gluedToPrevNoun = prev != null &&
          endsANoun(prev.type) &&
          prev.line == tok.line &&
          endColumn(prev) == tok.column;
      if (gluedToDigit && !gluedToPrevNoun) {
        out.add(Token(
          type: next.type,
          value: '-${next.value}',
          line: tok.line,
          column: tok.column,
          flags: next.flags,
        ));
        i += 2;
        continue;
      }
    }
    out.add(tok);
    i += 1;
  }
  return out;
}

/// Creates a [GrammarLexer] for [source] with the Q grammar and both
/// hooks installed.
GrammarLexer createQLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPreTokenize(stripSlashComments);
  lexer.addPostTokenize(foldNegativeNumberLiterals);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeQ(String source) => createQLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_q` with
/// idiomatic Dart nullability (catch [LexerError] for the message).
List<Token>? tryTokenizeQ(String source) {
  try {
    return tokenizeQ(source);
  } on LexerError {
    return null;
  }
}
