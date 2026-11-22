/// Centralized, localizable error messages and diagnostic string constants.
///
/// Complies with Standing Rule #20: 100% of error descriptions, codes,
/// and placeholder strings are centralized here. Exceptions use closed
/// static diagnostic strings and never reflect caller-supplied payloads,
/// preventing sensitive information leakage through exception channels.
library;

abstract final class StarlarkCompilerMessages {
  /// Error message when an integer is outside the unsigned 8-bit range [0, 255].
  static const String invalidByte =
      'Invalid byte value: must be an unsigned 8-bit integer (0x00 to 0xFF).';

  /// Error message when an opcode byte does not correspond to a known opcode.
  static const String unknownOpcode =
      'Unknown or unassigned Starlark opcode byte.';

  /// Error message when an operator string does not map to any opcode.
  static const String unknownOperator = 'Unknown or unmapped operator symbol.';

  /// Error message when incoming corpus exceeds the maximum byte limit.
  static const String corpusByteLimitExceeded =
      'Corpus exceeds maximum allowed byte limit of 131072 bytes.';

  /// Error message when corpus transport format is malformed or invalid UTF-8.
  static const String corpusMalformedTransport =
      'Corpus transport encoding is malformed or contains invalid Unicode escapes.';

  /// Error message when corpus schema structure or row invariants are violated.
  static const String corpusSchemaViolation =
      'Corpus schema or structural invariant violation.';

  /// Redacted placeholder for sensitive or suppressed payload data.
  static const String redacted = '<redacted>';

  /// Generic invalid placeholder.
  static const String invalid = '<invalid>';
}
