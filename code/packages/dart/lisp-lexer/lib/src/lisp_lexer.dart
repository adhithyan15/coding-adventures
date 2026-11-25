/// Hand-written Lisp tokenizer: [TokenType], [Token], [LexerError],
/// and the [tokenize] entry point.
///
/// Lisp has one of the simplest token sets of any language: everything is
/// an atom (number, symbol, string) or a list (stuff in parentheses).
/// The one tricky part is symbols — `+`, `-`, `<=`, `set!`, `null?` are
/// all symbols — with a single disambiguation: `-` followed by a digit
/// is a negative number, otherwise it is a symbol.
///
/// The scanner works on Unicode scalar values (Dart runes), matching the
/// reference's `char` walk; positions count runes, which coincide with
/// the reference's indices on all scalar input. (The reference documents
/// byte offsets but counts `chars` — the code governs.)
library;

/// The kinds of tokens in Lisp source code.
enum TokenType {
  /// An integer literal, possibly negative (`42`, `-7`, `0`).
  number,

  /// An identifier or operator name (`define`, `+`, `null?`).
  symbol,

  /// A double-quoted string literal, quotes included (`"hello"`).
  string,

  /// Opening parenthesis `(`.
  lParen,

  /// Closing parenthesis `)`.
  rParen,

  /// Single quote `'`, sugar for `(quote ...)`.
  quote,

  /// Dot `.`, for dotted pairs (`(a . b)`).
  dot,

  /// End of input. Every token stream ends with exactly one EOF token.
  eof;

  /// The reference's display name (`NUMBER`, `SYMBOL`, ...).
  String get displayName {
    switch (this) {
      case TokenType.number:
        return 'NUMBER';
      case TokenType.symbol:
        return 'SYMBOL';
      case TokenType.string:
        return 'STRING';
      case TokenType.lParen:
        return 'LPAREN';
      case TokenType.rParen:
        return 'RPAREN';
      case TokenType.quote:
        return 'QUOTE';
      case TokenType.dot:
        return 'DOT';
      case TokenType.eof:
        return 'EOF';
    }
  }
}

/// A single token extracted from Lisp source code.
final class Token {
  const Token(this.tokenType, this.value);

  /// What kind of token this is.
  final TokenType tokenType;

  /// The original source text: digits for numbers, identifier text for
  /// symbols, quotes included for strings, the single character for
  /// delimiters, and empty for EOF.
  final String value;

  @override
  bool operator ==(Object other) =>
      other is Token && other.tokenType == tokenType && other.value == value;

  @override
  int get hashCode => Object.hash(tokenType, value);

  @override
  String toString() => 'Token(${tokenType.displayName}, "$value")';
}

/// An error during tokenization: an unrecognized character or an
/// unterminated string.
final class LexerError implements Exception {
  const LexerError({required this.message, required this.position});

  /// Human-readable description.
  final String message;

  /// Rune offset in the source where the error occurred.
  final int position;

  @override
  bool operator ==(Object other) =>
      other is LexerError &&
      other.message == message &&
      other.position == position;

  @override
  int get hashCode => Object.hash(message, position);

  @override
  String toString() => 'LexerError at position $position: $message';
}

/// Whether [rune] can start a Lisp symbol: ASCII letters, `_`, and
/// `+ - * / = < > ! ? &`. Digits cannot start one.
bool _isSymbolStart(int rune) =>
    (rune >= 0x41 && rune <= 0x5A) ||
    (rune >= 0x61 && rune <= 0x7A) ||
    rune == 0x5F ||
    rune == 0x2B ||
    rune == 0x2D ||
    rune == 0x2A ||
    rune == 0x2F ||
    rune == 0x3D ||
    rune == 0x3C ||
    rune == 0x3E ||
    rune == 0x21 ||
    rune == 0x3F ||
    rune == 0x26;

/// Whether [rune] can continue a symbol: start characters plus digits.
bool _isSymbolContinue(int rune) =>
    _isSymbolStart(rune) || (rune >= 0x30 && rune <= 0x39);

/// Whether [rune] is ASCII whitespace: space, tab, LF, FF, CR.
bool _isAsciiWhitespace(int rune) =>
    rune == 0x20 ||
    rune == 0x09 ||
    rune == 0x0A ||
    rune == 0x0C ||
    rune == 0x0D;

/// Renders [rune] the way Rust's `{:?}` renders a `char`: simple escapes
/// for the controls, ASCII printables as-is, and `\u{hex}` otherwise.
///
/// (Rust keeps non-ASCII printables unescaped; without Unicode tables
/// this port escapes them. ASCII — every realistic error character — is
/// exact.)
String _debugChar(int rune) {
  const simple = {
    0x00: r'\0',
    0x09: r'\t',
    0x0A: r'\n',
    0x0D: r'\r',
    0x27: r"\'",
    0x5C: r'\\',
  };
  if (simple.containsKey(rune)) return "'${simple[rune]}'";
  if (rune >= 0x20 && rune <= 0x7E) {
    return "'${String.fromCharCode(rune)}'";
  }
  return "'\\u{${rune.toRadixString(16)}}'";
}

/// Tokenizes Lisp [source] into tokens ending with exactly one EOF.
///
/// Whitespace and `;` comments are skipped. Throws [LexerError] on
/// unrecognized characters and unterminated strings.
List<Token> tokenize(String source) {
  final runes = source.runes.toList();
  final tokens = <Token>[];
  var pos = 0;

  while (pos < runes.length) {
    // Skip whitespace and `;`-to-end-of-line comments silently.
    if (_isAsciiWhitespace(runes[pos])) {
      pos++;
      continue;
    }
    if (runes[pos] == 0x3B) {
      while (pos < runes.length && runes[pos] != 0x0A) {
        pos++;
      }
      continue;
    }

    // Single-character delimiters, no lookahead. Note `.` is always DOT
    // — the reference muses about `.5` but never checks.
    switch (runes[pos]) {
      case 0x28:
        tokens.add(const Token(TokenType.lParen, '('));
        pos++;
        continue;
      case 0x29:
        tokens.add(const Token(TokenType.rParen, ')'));
        pos++;
        continue;
      case 0x27:
        tokens.add(const Token(TokenType.quote, "'"));
        pos++;
        continue;
      case 0x2E:
        tokens.add(const Token(TokenType.dot, '.'));
        pos++;
        continue;
    }

    // Strings: backslash escapes any next character; the value keeps the
    // quotes.
    if (runes[pos] == 0x22) {
      final start = pos;
      pos++;
      while (pos < runes.length && runes[pos] != 0x22) {
        if (runes[pos] == 0x5C) {
          pos++;
        }
        pos++;
      }
      if (pos >= runes.length) {
        throw LexerError(
            message: 'Unterminated string literal', position: start);
      }
      pos++;
      tokens.add(Token(
          TokenType.string, String.fromCharCodes(runes.sublist(start, pos))));
      continue;
    }

    // Numbers: digits, or `-` followed by a digit. A lone `-` falls
    // through to symbols.
    if (_isDigit(runes[pos]) ||
        (runes[pos] == 0x2D &&
            pos + 1 < runes.length &&
            _isDigit(runes[pos + 1]))) {
      final start = pos;
      if (runes[pos] == 0x2D) {
        pos++;
      }
      while (pos < runes.length && _isDigit(runes[pos])) {
        pos++;
      }
      tokens.add(Token(
          TokenType.number, String.fromCharCodes(runes.sublist(start, pos))));
      continue;
    }

    // Symbols: the catch-all for identifiers and operator names.
    if (_isSymbolStart(runes[pos])) {
      final start = pos;
      pos++;
      while (pos < runes.length && _isSymbolContinue(runes[pos])) {
        pos++;
      }
      tokens.add(Token(
          TokenType.symbol, String.fromCharCodes(runes.sublist(start, pos))));
      continue;
    }

    throw LexerError(
        message: 'Unexpected character: ${_debugChar(runes[pos])}',
        position: pos);
  }

  tokens.add(const Token(TokenType.eof, ''));
  return tokens;
}

/// Whether [rune] is an ASCII digit.
bool _isDigit(int rune) => rune >= 0x30 && rune <= 0x39;
