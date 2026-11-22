/// Centralized, localizable error messages and diagnostic string constants.
///
/// Complies with Standing Rule #20: 100% of error descriptions, codes,
/// and placeholder strings are centralized here. Exceptions use closed
/// static diagnostic strings and never reflect caller-supplied payloads,
/// preventing sensitive information leakage through exception channels.
library;

abstract final class StarlarkParserMessages {
  /// Error message when an unrecognized Starlark parser edition is provided.
  static const String invalidVersion =
      'Unknown or unsupported Starlark parser grammar version. Valid value: starlark.';

  /// Error message when token count exceeds the maximum allowable limit.
  static const String tokenCountExceeded =
      'Starlark token count exceeds maximum allowed limit of 65,536 tokens.';

  /// Error message when syntax nesting depth exceeds the maximum ceiling.
  static const String nestingTooDeep =
      'Starlark syntax nesting depth exceeds maximum allowed limit of 64.';

  /// Error message when syntax analysis fails on invalid grammar constructs.
  static const String syntaxError =
      'Syntax analysis failed due to invalid Starlark grammar construct.';

  /// Redacted placeholder for sensitive or suppressed payload data.
  static const String redacted = '<redacted>';

  /// Generic invalid placeholder.
  static const String invalid = '<invalid>';
}
