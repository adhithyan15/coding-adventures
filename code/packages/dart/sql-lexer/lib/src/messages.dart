/// Diagnostic and error messages for SQL lexical analysis, strictly payload-blind.
///
/// Standing Rule #20: Every error id, message, and placeholder is maintained
/// in this file so operational code carries no string literals and no diagnostic
/// can ever echo caller SQL source code or database payloads.
library;

/// Error identifier for unsupported SQL version requests.
const String errorIdInvalidVersion = 'invalid-version';

/// Error identifier for source code violating bounds or encoding rules.
const String errorIdInvalidSource = 'invalid-source';

/// Error identifier for lexical scanning failures.
const String errorIdLexicalError = 'lexical-error';

/// Message thrown when an unknown or unsupported SQL version string is provided.
const String messageInvalidVersion = 'Unknown or unsupported SQL version.';

/// Message thrown when SQL source exceeds the maximum allowed size ceiling.
const String messageSourceTooLarge = 'SQL source exceeds maximum allowed size.';

/// Message thrown when SQL source contains lone or reversed UTF-16 surrogates.
const String messageInvalidSurrogates =
    'SQL source contains invalid Unicode surrogates.';

/// Message thrown when lexical analysis fails due to an unrecognized character.
const String messageLexicalError =
    'Lexical analysis failed due to unrecognized token.';

/// Redacted placeholder for payload confidentiality (CWE-209).
const String redactedPlaceholder = '<redacted>';

/// Placeholder for invalid or unparseable elements.
const String invalidPlaceholder = '<invalid>';

/// Formats an error message with 1-based line and column offsets if available.
String formatLocation(String message, int? line, int? column) {
  if (line != null && column != null) {
    return '$message ($line:$column)';
  }
  return message;
}
