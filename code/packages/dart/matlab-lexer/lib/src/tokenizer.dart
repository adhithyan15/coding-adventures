/// MATLAB tokenization over the compiled grammar: the two
/// pre-hooks (block-comment strip, quote protection with `...`
/// splicing), the two post-hooks (placeholder restore, paren-newline
/// dropping), the factory, and the throwing and nullable one-shots.
/// Token types keep their grammar names verbatim; the final token is
/// always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// Strips `%{ … %}` block comments. MATLAB requires the block markers
/// to sit ALONE on their lines, so this is a line-wise pass: a line
/// whose trimmed content is exactly `%{` opens a block and `%}`
/// closes it; lines in between (and the marker lines) are blanked,
/// preserving newlines so line numbers stay aligned.
String stripBlockComments(String source) {
  final out = StringBuffer();
  // Mirror `split_inclusive('\n')`: Dart's split turns a trailing
  // newline into a phantom empty piece, which the inclusive split
  // never yields.
  final lines = source.split('\n');
  final pieces = (source.endsWith('\n') && lines.isNotEmpty)
      ? lines.sublist(0, lines.length - 1)
      : lines;
  final endsWithNewline = source.endsWith('\n');
  var inBlock = false;
  for (var i = 0; i < pieces.length; i++) {
    final body = pieces[i];
    final nl = (i == pieces.length - 1 && !endsWithNewline) ? '' : '\n';
    final trimmed = body.trim();
    if (!inBlock && trimmed == '%{') {
      inBlock = true;
      out.write(nl);
    } else if (inBlock && trimmed == '%}') {
      inBlock = false;
      out.write(nl);
    } else if (inBlock) {
      out.write(nl);
    } else {
      out.write(body);
      out.write(nl);
    }
  }
  return out.toString();
}

/// ASCII letters and digits. The reference uses Unicode
/// `is_alphanumeric` here, but every value token in this grammar is
/// ASCII, so a non-ASCII character is a lex error in both lanes
/// either way — protection never consumes it, and both lanes fail
/// closed on it.
bool _isAsciiAlphanumeric(int rune) =>
    (rune >= 0x30 && rune <= 0x39) ||
    (rune >= 0x41 && rune <= 0x5A) ||
    (rune >= 0x61 && rune <= 0x7A);

/// Resolves the transpose/char-array ambiguity for `'`, passing `%`
/// comments and `"..."` strings through untouched, and splices `...`
/// line continuations.
///
/// A `'` preceded (in value context) by a value stays for the grammar
/// to lex as `TRANSPOSE`; otherwise it opens a char-array literal
/// (with `''` escape) recorded in [table] and replaced by a `` `N` ``
/// placeholder. A `...` continuation and the rest of its line plus
/// the newline are dropped, joining the two physical lines (the value
/// context carries over: the expression continues).
String _protectQuotes(String source, List<String> table) {
  final chars = source.runes.toList();
  final out = StringBuffer();
  var i = 0;
  final n = chars.length;
  var prevValue = false;
  while (i < n) {
    final c = chars[i];
    // `...` line continuation: drop it and the rest of the line +
    // newline.
    if (c == 0x2E &&
        i + 2 < n &&
        chars[i + 1] == 0x2E &&
        chars[i + 2] == 0x2E) {
      i += 3;
      while (i < n && chars[i] != 0x0A) {
        i++;
      }
      if (i < n) {
        i++; // consume the newline too — the lines join
      }
      // prev_value is unchanged: the expression continues.
      continue;
    }
    // `%` line comment: copy verbatim to end of line.
    if (c == 0x25) {
      while (i < n && chars[i] != 0x0A) {
        out.writeCharCode(chars[i]);
        i++;
      }
      continue;
    }
    // Double-quoted string: copy whole, respecting the `""` escape.
    if (c == 0x22) {
      out.writeCharCode(0x22);
      i++;
      while (i < n) {
        if (chars[i] == 0x22) {
          if (i + 1 < n && chars[i + 1] == 0x22) {
            out.writeCharCode(0x22);
            out.writeCharCode(0x22);
            i += 2;
          } else {
            out.writeCharCode(0x22);
            i++;
            break;
          }
        } else {
          out.writeCharCode(chars[i]);
          i++;
        }
      }
      prevValue = true; // a string is a value
      continue;
    }
    // Transpose: value context holds, leave the quote for the grammar.
    if (c == 0x27 && prevValue) {
      out.writeCharCode(0x27);
      i++;
      prevValue = true; // the transposed result is itself a value
      continue;
    }
    // Char-array literal: consume with `''` escape, record, placeholder.
    if (c == 0x27) {
      final lit = StringBuffer("'");
      i++;
      while (true) {
        if (i >= n) break; // unterminated — emit what we have
        if (chars[i] == 0x27) {
          if (i + 1 < n && chars[i + 1] == 0x27) {
            lit.write("''");
            i += 2;
          } else {
            lit.write("'");
            i++;
            break;
          }
        } else {
          lit.writeCharCode(chars[i]);
          i++;
        }
      }
      table.add(lit.toString());
      final idx = (table.length - 1).toString();
      out.writeCharCode(0x60);
      out.write(idx);
      out.writeCharCode(0x60);
      prevValue = true; // a string is a value
      continue;
    }
    if (c == 0x20 || c == 0x09) {
      out.writeCharCode(c);
      i++;
      prevValue = false; // whitespace breaks the transpose context
      continue;
    }
    if (c == 0x0A || c == 0x0D) {
      out.writeCharCode(c);
      i++;
      prevValue = false;
      continue;
    }
    out.writeCharCode(c);
    i++;
    prevValue = _isAsciiAlphanumeric(c) ||
        c == 0x5F ||
        c == 0x29 ||
        c == 0x5D ||
        c == 0x7D ||
        c == 0x2E;
  }
  return out.toString();
}

/// Decodes a char-array lexeme to its content: drops the surrounding
/// quotes, collapses each doubled `''` to `'`.
String _decodeCharArray(String raw) {
  var inner = raw;
  if (inner.startsWith("'") && inner.endsWith("'") && inner.length >= 2) {
    inner = inner.substring(1, inner.length - 1);
  }
  return inner.replaceAll("''", "'");
}

/// Replaces each `CHARARRAY` token's value with the decoded content
/// of its recorded lexeme and re-labels it `STRING`, so downstream
/// sees char arrays and double-quoted strings uniformly.
///
/// Restoration keys off the distinct `CHARARRAY` *type*, never the
/// value. Like the reference (and unlike the scilab lane, which
/// position-checks), the `STRING` re-label is unconditional: even an
/// unresolvable placeholder reads as a `STRING` for the parser to
/// reject on shape rather than on type.
List<Token> _restoreQuotes(List<Token> tokens, List<String> table) {
  return [
    for (final token in tokens)
      if (token.type != 'CHARARRAY')
        token
      else
        Token(
          type: 'STRING',
          value: _restoreValue(token.value, table) ?? token.value,
          line: token.line,
          column: token.column,
          flags: token.flags,
        ),
  ];
}

String? _restoreValue(String value, List<String> table) {
  if (value.length < 2) return null;
  if (!(value.startsWith('`') && value.endsWith('`'))) return null;
  final idx = int.tryParse(value.substring(1, value.length - 1));
  if (idx == null || idx < 0 || idx >= table.length) return null;
  return _decodeCharArray(table[idx]);
}

/// Drops `NEWLINE` tokens inside open `( )` only. Newlines inside
/// `[ ]`/`{ }` separate matrix/cell rows and are kept.
List<Token> _dropParenNewlines(List<Token> tokens) {
  final result = <Token>[];
  var depth = 0;
  for (final token in tokens) {
    if (token.type == 'NEWLINE' && depth > 0) {
      continue;
    }
    if (token.type == 'LPAREN') {
      depth += 1;
    } else if (token.type == 'RPAREN') {
      // Saturating, mirroring the reference's `saturating_sub`.
      if (depth > 0) depth -= 1;
    }
    result.add(token);
  }
  return result;
}

/// Creates a [GrammarLexer] for [source] with the MATLAB grammar and
/// all four hooks installed in reference order: block-comment strip,
/// then quote protection (sharing one literal table with the restore
/// post-hook, like the reference's `Rc<RefCell>`), then placeholder
/// restore, then paren-newline dropping.
GrammarLexer createMatlabLexer(String source) {
  final table = <String>[];
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPreTokenize(stripBlockComments);
  lexer.addPreTokenize((src) => _protectQuotes(src, table));
  lexer.addPostTokenize((tokens) => _restoreQuotes(tokens, table));
  lexer.addPostTokenize(_dropParenNewlines);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeMatlab(String source) =>
    createMatlabLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_matlab`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeMatlab(String source) {
  try {
    return tokenizeMatlab(source);
  } on LexerError {
    return null;
  }
}
