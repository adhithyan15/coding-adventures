/// Centralized, localizable error messages and diagnostic string constants.
///
/// Complies with Standing Rule #20: 100% of error descriptions, codes,
/// and placeholder strings are centralized here. Exceptions use closed
/// static diagnostic strings and never reflect caller-supplied payloads,
/// preventing sensitive information leakage through exception channels.
library;

abstract final class VerilogLexerMessages {
  /// Error message when an unrecognized Verilog version string is provided.
  static const String invalidVersion =
      'Unknown or unsupported Verilog grammar version. Valid values: 1995, 2001, 2005.';

  /// Error message when source text exceeds the maximum allowable length.
  static const String sourceTooLarge =
      'Verilog source code exceeds maximum allowed size limit of 1 MB (1048576 code units).';

  /// Error message when source text contains malformed UTF-16 surrogate pairs.
  static const String invalidSurrogates =
      'Verilog source code contains unpaired or ill-formed UTF-16 surrogates.';

  /// Error message when preprocessor conditional nesting depth exceeds maximum limit.
  static const String conditionalDepthExceeded =
      'Verilog preprocessor conditional nesting depth exceeds maximum limit of 64.';

  /// Error message when lexical analysis fails on an unexpected character sequence.
  static const String lexicalError =
      'Lexical analysis failed due to unrecognized token or syntax error.';

  /// Redacted placeholder for sensitive or suppressed payload data.
  static const String redacted = '<redacted>';

  /// Generic invalid placeholder.
  static const String invalid = '<invalid>';
}
