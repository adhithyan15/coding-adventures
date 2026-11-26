/// COBOL-60 tokenization over the compiled grammar: the
/// column-strip pre-hook, the factory, and the throwing and nullable
/// one-shots. Token types keep their grammar names verbatim; the final
/// token is always `EOF`.
///
/// PICTURE strings need no hook: the grammar's declarative mode
/// transition switches into the `picture` group on `PIC`/`PICTURE`
/// and back out after one `PIC_STRING`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Strips COBOL-60 fixed-format columns, turning 80-column card images
/// into the free-form text the grammar tokenizes.
///
/// COBOL source is laid out in fixed columns (1-based): 1–6 sequence
/// number (dropped), 7 indicator (`*`/`/` comment, `-`
/// continuation), 8–72 code area, 73–80 identification (dropped).
/// Comment lines vanish entirely; `-` continuation lines splice onto
/// their predecessor (a simple joint-space splice, like the
/// reference; literal-aware continuation is future work).
///
/// A pure function of the source text — no state, no I/O — exactly
/// the contract a pre-tokenize hook requires. Exported for direct
/// use and testing, like the reference.
String stripCobolColumns(String source) {
  final out = <String>[];
  for (final rawLine in source.split('\n')) {
    // Normalise a trailing CR from CRLF-terminated cards so it never
    // lands inside the code area of a short line.
    var raw = rawLine;
    if (raw.endsWith('\r')) {
      raw = raw.substring(0, raw.length - 1);
    }
    final chars = raw.runes.toList();

    // A line with no indicator column (fewer than 7 chars) carries no
    // code.
    if (chars.length < 7) {
      out.add('');
      continue;
    }

    // Column 7 (index 6) is the indicator.
    final indicator = chars[6];
    if (indicator == 0x2A || indicator == 0x2F) {
      // Whole-line comment (`/` also means page-eject) — drop it.
      continue;
    }

    // Code area: columns 8–72 → char indices 7..72, clipped to the
    // line.
    final end = chars.length < 72 ? chars.length : 72;
    final code = String.fromCharCodes(chars.sublist(7, end));

    if (indicator == 0x2D) {
      // Continuation: append this line's code onto the previous one.
      if (out.isNotEmpty) {
        out[out.length - 1] = out.last.trimRight() + code.trimLeft();
        continue;
      }
      // No predecessor to continue — fall through and keep the code
      // as-is.
    }

    out.add(code);
  }
  return out.join('\n');
}

/// Creates a [GrammarLexer] for [source] with the COBOL-60 grammar
/// and the [stripCobolColumns] pre-tokenize hook registered, so
/// callers can pass raw carded source directly.
GrammarLexer createCobolLexer(String source) {
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPreTokenize(stripCobolColumns);
  return lexer;
}

/// Tokenizes COBOL-60 [source] (carded or already free-form) in one
/// shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeCobol(String source) => createCobolLexer(source).tokenize();

/// Tokenizes COBOL-60 [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_cobol`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeCobol(String source) {
  try {
    return tokenizeCobol(source);
  } on LexerError {
    return null;
  }
}
