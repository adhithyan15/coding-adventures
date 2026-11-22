/// Diagnostic and error messages for the Dart lexers, strictly payload-blind.
///
/// Standing Rule #20: Every error id, message, and message template is
/// maintained in this file so operational code carries no diagnostic string
/// literals and no diagnostic can ever echo caller source text or keywords
/// (CWE-209). Locations (line and column) are the only caller-derived data.
library;

/// Error identifier for lexical analysis failures.
const String errorIdLexicalError = 'lexical-error';

/// Message thrown when no token definition matches the current character.
const String messageUnexpectedCharacter = 'Unexpected character.';

/// Message thrown when a reserved keyword is used as an identifier.
const String messageReservedKeyword =
    'Reserved keyword cannot be used as an identifier.';

/// Message thrown when a string literal is not closed before end of input.
const String messageUnterminatedString = 'Unterminated string literal.';

/// Message thrown when a string literal ends with a dangling backslash.
const String messageUnterminatedStringBackslash =
    'Unterminated string literal (ends with backslash).';

/// Formats a lexer error with its 1-based line and column and static message.
String formatLexerError(String message, int line, int column) =>
    'Lexer error at $line:$column: $message';
