/// Diagnostic and error messages for Lattice parsing, strictly payload-blind.
///
/// Standing Rule #20: Every error id, message, and placeholder is maintained
/// in this file so operational code carries no string literals and no diagnostic
/// can ever echo caller Lattice stylesheets or property values.
library;

/// Error identifier for unsupported Lattice version requests.
const String errorIdInvalidVersion = 'invalid-version';

/// Error identifier when token sequence length exceeds allowed bounds.
const String errorIdTokenCountExceeded = 'token-count-exceeded';

/// Error identifier when delimiter nesting exceeds safe recursion depth.
const String errorIdNestingTooDeep = 'nesting-too-deep';

/// Error identifier for grammar parse syntax errors.
const String errorIdSyntaxError = 'syntax-error';

/// Message thrown when an unknown or unsupported Lattice version string is provided.
const String messageInvalidVersion = 'Unknown or unsupported Lattice version.';

/// Message thrown when the token count exceeds the maximum allowed ceiling.
const String messageTokenCountExceeded =
    'Lattice token count exceeds maximum allowed limit.';

/// Message thrown when delimiter nesting exceeds the maximum recursion limit.
const String messageNestingTooDeep =
    'Lattice block or expression nesting depth exceeds maximum allowed limit.';

/// Message thrown when parsing fails due to unexpected syntax.
const String messageSyntaxError = 'Lattice syntax error.';

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
