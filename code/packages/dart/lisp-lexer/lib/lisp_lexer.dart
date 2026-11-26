/// Lisp lexer: tokenizes Lisp source code into atoms, delimiters, and
/// strings.
///
/// [tokenize] is the entry point: atoms (numbers, symbols, strings),
/// single-character delimiters (`(`, `)`, `'`, `.`), and exactly one
/// trailing [TokenType.eof]. Whitespace and `;` comments are skipped.
/// Failures throw [LexerError].
library;

export 'src/lisp_lexer.dart';
