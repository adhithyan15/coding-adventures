/// Scilab tokenization over the compiled grammar: the factory, the
/// throwing and nullable one-shots, and the four hooks — quote
/// protection (pre) plus placeholder restore, escape collapse, and
/// paren-newline dropping (post). Token types keep their grammar names
/// verbatim; the final token is always `EOF`.
library;

import 'package:coding_adventures_lexer/lexer.dart';

import '_grammar.dart';

/// A legitimately inserted string placeholder: the original lexeme plus
/// the (line, column) in the TRANSFORMED source where the `` `N` ``
/// marker was written. Restoration trusts position, never the index
/// alone — a verbatim `` `N` `` typed elsewhere must not resolve.
class _Placeholder {
  const _Placeholder(
      {required this.text, required this.line, required this.column});
  final String text;
  final int line;
  final int column;
}

/// ASCII letters and digits. The reference uses Unicode
/// `is_alphanumeric` here, but every value token in this grammar is
/// ASCII (`NAME` is `[A-Za-z_][A-Za-z0-9_]*`), so a non-ASCII
/// character is a lex error in both lanes either way — the only
/// theoretical divergence is which error fires first on exotic input
/// glued to a quote.
bool _isAsciiAlphanumeric(int rune) =>
    (rune >= 0x30 && rune <= 0x39) ||
    (rune >= 0x41 && rune <= 0x5A) ||
    (rune >= 0x61 && rune <= 0x7A);

/// Resolves the transpose/string ambiguity for `'`, passing `//`,
/// `/* */`, and `"..."` through untouched.
///
/// A `'` preceded (in value context) by a value stays for the grammar
/// to lex as `TRANSPOSE`; otherwise it opens a string literal (with
/// `''` escape) recorded in [table] and replaced by a `` `N` ``
/// placeholder. Comments and double-quoted strings never touch the
/// value tracking. Every character written to the output advances the
/// tracked (line, column) exactly as the engine's cursor will count
/// them later.
String _protectQuotes(String source, List<_Placeholder> table) {
  final chars = source.runes.toList();
  final out = StringBuffer();
  var line = 1;
  var col = 1;
  void pushTracked(int c) {
    out.writeCharCode(c);
    if (c == 0x0A) {
      line += 1;
      col = 1;
    } else {
      col += 1;
    }
  }

  var i = 0;
  final n = chars.length;
  var prevValue = false;
  while (i < n) {
    final c = chars[i];
    // `//` line comment: copy verbatim to end of line.
    if (c == 0x2F && i + 1 < n && chars[i + 1] == 0x2F) {
      while (i < n && chars[i] != 0x0A) {
        pushTracked(chars[i]);
        i++;
      }
      continue;
    }
    // `/* ... */` block comment: copy verbatim (may span lines, may
    // sit inline; unterminated runs to EOF for the parser to reject).
    if (c == 0x2F && i + 1 < n && chars[i + 1] == 0x2A) {
      pushTracked(0x2F);
      pushTracked(0x2A);
      i += 2;
      while (i < n) {
        if (chars[i] == 0x2A && i + 1 < n && chars[i + 1] == 0x2F) {
          pushTracked(0x2A);
          pushTracked(0x2F);
          i += 2;
          break;
        }
        pushTracked(chars[i]);
        i++;
      }
      continue;
    }
    // Double-quoted string: copy whole, respecting the `""` escape.
    if (c == 0x22) {
      pushTracked(0x22);
      i++;
      while (i < n) {
        if (chars[i] == 0x22) {
          if (i + 1 < n && chars[i + 1] == 0x22) {
            pushTracked(0x22);
            pushTracked(0x22);
            i += 2;
          } else {
            pushTracked(0x22);
            i++;
            break;
          }
        } else {
          pushTracked(chars[i]);
          i++;
        }
      }
      prevValue = true; // a string is a value
      continue;
    }
    // Transpose: value context holds, leave the quote for the grammar.
    if (c == 0x27 && prevValue) {
      pushTracked(0x27);
      i++;
      prevValue = true; // the transposed result is itself a value
      continue;
    }
    // String literal: consume with `''` escape, record, placeholder.
    if (c == 0x27) {
      final lit = StringBuffer("'");
      i++;
      while (true) {
        if (i >= n) break; // unterminated — parser errors later
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
      // Record BEFORE writing the opening backtick: this is the
      // (line, column) the grammar assigns the placeholder token.
      table.add(_Placeholder(text: lit.toString(), line: line, column: col));
      final idx = (table.length - 1).toString();
      pushTracked(0x60);
      for (final d in idx.runes) {
        pushTracked(d);
      }
      pushTracked(0x60);
      prevValue = true; // a string is a value
      continue;
    }
    if (c == 0x20 || c == 0x09) {
      pushTracked(c);
      i++;
      prevValue = false; // whitespace breaks the transpose context
      continue;
    }
    if (c == 0x0A || c == 0x0D) {
      pushTracked(c);
      i++;
      prevValue = false;
      continue;
    }
    pushTracked(c);
    i++;
    prevValue = _isAsciiAlphanumeric(c) ||
        c == 0x5F ||
        c == 0x29 ||
        c == 0x5D ||
        c == 0x7D ||
        c == 0x2E ||
        c == 0x24;
  }
  return out.toString();
}

/// Decodes a raw single-quoted lexeme: drops the surrounding quotes,
/// collapses each doubled `''` to `'`.
String _decodeStringLiteral(String raw) {
  var inner = raw;
  if (inner.startsWith("'") && inner.endsWith("'") && inner.length >= 2) {
    inner = inner.substring(1, inner.length - 1);
  }
  return inner.replaceAll("''", "'");
}

/// Replaces each `STRING_PLACEHOLDER` token with the decoded content
/// of its recorded lexeme and re-labels it `STRING`.
///
/// Trusts the distinct TYPE (never the value) plus the recorded
/// (line, column): a verbatim `` `N` `` typed in ordinary source, or
/// an out-of-range index, stays unresolved for the parser to reject.
List<Token> _restorePlaceholders(List<Token> tokens, List<_Placeholder> table) {
  return [
    for (final token in tokens)
      if (token.type != 'STRING_PLACEHOLDER')
        token
      else
        _restoreOne(token, table) ?? token,
  ];
}

Token? _restoreOne(Token token, List<_Placeholder> table) {
  final value = token.value;
  // Total like the reference's strip/parse chain: a lone "`" matches
  // both ends but has no inner text (unreachable via the engine,
  // whose pattern requires digits — defense in depth, never a throw).
  if (value.length < 2) return null;
  if (!(value.startsWith('`') && value.endsWith('`'))) return null;
  final inner = value.substring(1, value.length - 1);
  final idx = int.tryParse(inner);
  if (idx == null || idx < 0 || idx >= table.length) return null;
  final entry = table[idx];
  if (entry.line != token.line || entry.column != token.column) {
    return null;
  }
  return Token(
    type: 'STRING',
    value: _decodeStringLiteral(entry.text),
    line: token.line,
    column: token.column,
    flags: token.flags,
  );
}

/// Collapses the `""` escape in `DQ_STRING` tokens and re-labels them
/// `STRING`. Only ever touches `DQ_STRING` — never an already-decoded
/// placeholder value that might legitimately contain `""`.
List<Token> _collapseDqStringEscapes(List<Token> tokens) => [
      for (final token in tokens)
        if (token.type != 'DQ_STRING')
          token
        else
          Token(
            type: 'STRING',
            value: token.value.replaceAll('""', '"'),
            line: token.line,
            column: token.column,
            flags: token.flags,
          ),
    ];

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
      // Saturating, mirroring the reference's `saturating_sub`: an
      // unbalanced `)` never drives the depth negative (which would
      // wrongly KEEP a later paren-interior newline).
      if (depth > 0) depth -= 1;
    }
    result.add(token);
  }
  return result;
}

/// Creates a [GrammarLexer] for [source] with the Scilab grammar and
/// all four hooks installed, sharing one literal table between the
/// pre-hook (which fills it) and the restore post-hook (which reads
/// it) — the same sharing the reference expresses with `Rc<RefCell>`.
GrammarLexer createScilabLexer(String source) {
  final table = <_Placeholder>[];
  final lexer = GrammarLexer(source, tokenGrammar);
  lexer.addPreTokenize((src) => _protectQuotes(src, table));
  lexer.addPostTokenize((tokens) => _restorePlaceholders(tokens, table));
  lexer.addPostTokenize(_collapseDqStringEscapes);
  lexer.addPostTokenize(_dropParenNewlines);
  return lexer;
}

/// Tokenizes [source] in one shot.
/// Throws [LexerError] on unrecognized input (the reference panics;
/// Dart lanes propagate the structured error instead).
List<Token> tokenizeScilab(String source) =>
    createScilabLexer(source).tokenize();

/// Tokenizes [source], returning `null` instead of throwing.
/// Mirrors the reference's `Result`-returning `try_tokenize_scilab`
/// with idiomatic Dart nullability (catch [LexerError] for the
/// message).
List<Token>? tryTokenizeScilab(String source) {
  try {
    return tokenizeScilab(source);
  } on LexerError {
    return null;
  }
}
