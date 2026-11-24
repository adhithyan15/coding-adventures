/// One EAN-13 digit with its L/G/R encoding and 7-module pattern.
library;

/// A single encoded EAN-13 digit.
///
/// The leading digit (index 0) is never bar-encoded, so encoded entries
/// cover indexes 1-12 only.
final class EncodedDigit {
  /// The digit character (`0`-`9`).
  final String digit;

  /// The encoding side: `L` or `G` for the left half, `R` for the right.
  final String encoding;

  /// The 7-module binary pattern for this digit and side.
  final String pattern;

  /// Zero-based position in the normalized 13-digit string.
  final int sourceIndex;

  /// The run role: `data`, or `check` for the final check digit.
  final String role;

  /// Creates an encoded digit.
  const EncodedDigit({
    required this.digit,
    required this.encoding,
    required this.pattern,
    required this.sourceIndex,
    required this.role,
  });

  @override
  bool operator ==(Object other) =>
      other is EncodedDigit &&
      digit == other.digit &&
      encoding == other.encoding &&
      pattern == other.pattern &&
      sourceIndex == other.sourceIndex &&
      role == other.role;

  @override
  int get hashCode => Object.hash(digit, encoding, pattern, sourceIndex, role);

  @override
  String toString() =>
      'EncodedDigit($digit, $encoding, $pattern, index $sourceIndex, $role)';
}
