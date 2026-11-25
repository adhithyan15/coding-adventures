/// One Code 128 symbol with its value and binary pattern.
library;

/// A single encoded Code 128 symbol.
final class EncodedCode128Symbol {
  /// Human-readable label: the character, `Start B`, `Stop`, or
  /// `Checksum <value>`.
  final String label;

  /// The symbol value (0-106).
  final int value;

  /// The binary module pattern for this value.
  final String pattern;

  /// Zero-based position in the framed symbol sequence.
  final int sourceIndex;

  /// The run role: `start`, `data`, `check`, or `stop`.
  final String role;

  /// Creates an encoded symbol.
  const EncodedCode128Symbol({
    required this.label,
    required this.value,
    required this.pattern,
    required this.sourceIndex,
    required this.role,
  });

  @override
  bool operator ==(Object other) =>
      other is EncodedCode128Symbol &&
      label == other.label &&
      value == other.value &&
      pattern == other.pattern &&
      sourceIndex == other.sourceIndex &&
      role == other.role;

  @override
  int get hashCode => Object.hash(label, value, pattern, sourceIndex, role);

  @override
  String toString() => 'EncodedCode128Symbol($label, $value)';
}
