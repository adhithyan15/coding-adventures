/// Diagnostic and error messages for TypeScript lexical analysis, strictly payload-blind.
///
/// Standing Rule #20: Every error id, message, and placeholder is maintained
/// in this file so operational code carries no string literals and no diagnostic
/// can ever echo caller TypeScript source code or version strings.
library;

/// Error identifier for unsupported TypeScript version requests.
const String errorIdInvalidVersion = 'invalid-version';

/// Error identifier when source violates length or surrogate encoding bounds.
const String errorIdInvalidSource = 'invalid-source';

/// Error identifier for lexical analysis syntax errors.
const String errorIdLexicalError = 'lexical-error';

/// Message thrown when an unknown or unsupported TypeScript version is requested.
const String messageInvalidVersion =
    'Unknown or unsupported TypeScript version.';

/// Message thrown when source text exceeds the 1 MB ceiling.
const String messageSourceTooLarge =
    'TypeScript source exceeds maximum allowed size.';

/// Message thrown when source text contains lone or reversed UTF-16 surrogates.
const String messageInvalidSurrogates =
    'TypeScript source contains invalid UTF-16 surrogates.';

/// Message thrown when tokenization fails due to unrecognized characters.
const String messageLexicalError = 'TypeScript lexical analysis failed.';

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
