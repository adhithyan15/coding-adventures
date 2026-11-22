/// Diagnostic and error messages for TypeScript parsing, strictly payload-blind.
///
/// Standing Rule #20: Every error id, message, and placeholder is maintained
/// in this file so operational code carries no string literals and no diagnostic
/// can ever echo caller TypeScript code or version strings.
library;

/// Error identifier for unsupported TypeScript version requests.
const String errorIdInvalidVersion = 'invalid-version';

/// Error identifier when token count ceiling is exceeded.
const String errorIdTokenCountExceeded = 'token-count-exceeded';

/// Error identifier when delimiter or prefix operator recursion ceiling is exceeded.
const String errorIdNestingTooDeep = 'nesting-too-deep';

/// Error identifier for syntax parse errors.
const String errorIdSyntaxError = 'syntax-error';

/// Message thrown when an unknown or unsupported TypeScript version is requested.
const String messageInvalidVersion =
    'Unknown or unsupported TypeScript version.';

/// Message thrown when source exceeds the token count ceiling.
const String messageTokenCountExceeded =
    'TypeScript token stream exceeds maximum allowed token count.';

/// Message thrown when delimiter or prefix nesting exceeds the safe limit.
const String messageNestingTooDeep =
    'TypeScript delimiter nesting depth exceeds maximum allowed limit.';

/// Message thrown when a syntax violation occurs during parsing.
const String messageSyntaxError = 'TypeScript syntax parse error.';

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
