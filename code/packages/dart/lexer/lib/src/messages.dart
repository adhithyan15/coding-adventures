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

/// Message thrown when no group pattern matches the current sequence.
///
/// The reference echoes the offending character; this lane stays
/// payload-blind per Standing Rule #20, keeping the assertion-compatible
/// prefix.
const String messageUnexpectedSequence = 'Unexpected sequence.';

/// Message thrown when a reserved keyword is used as an identifier.
const String messageReservedKeyword =
    'Reserved keyword cannot be used as an identifier.';

/// Message thrown when a string literal is not closed before end of input.
const String messageUnterminatedString = 'Unterminated string literal.';

/// Message thrown when a string literal ends with a dangling backslash.
const String messageUnterminatedStringBackslash =
    'Unterminated string literal (ends with backslash).';

/// Message thrown when a tab character is used in indentation.
const String messageTabInIndentation =
    'Tab character in indentation (use spaces only).';

/// Message thrown when dedentation does not match any outer indentation level.
const String messageInconsistentDedent = 'Inconsistent dedent.';

/// Message thrown when a `refuse` transition fires: the token is not
/// supported in the active mode.
///
/// The reference echoes the token key, value, and mode; this lane
/// stays payload-blind per Standing Rule #20.
const String messageTokenRefusedInMode =
    'Token is not supported in this mode by this grammar.';

/// Formats a lexer error with its 1-based line and column and static message.
String formatLexerError(String message, int line, int column) =>
    'Lexer error at $line:$column: $message';
