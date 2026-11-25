/// One Codabar symbol with its binary module pattern.
library;

/// A single encoded Codabar symbol.
final class EncodedCodabarSymbol {
  /// The character (after ASCII case folding).
  final String ch;

  /// The binary module pattern for this character.
  final String pattern;

  /// Zero-based position in the normalized guarded string.
  final int sourceIndex;

  /// The run role: `start`, `stop`, or `data`.
  final String role;

  /// Creates an encoded symbol.
  const EncodedCodabarSymbol({
    required this.ch,
    required this.pattern,
    required this.sourceIndex,
    required this.role,
  });

  @override
  bool operator ==(Object other) =>
      other is EncodedCodabarSymbol &&
      ch == other.ch &&
      pattern == other.pattern &&
      sourceIndex == other.sourceIndex &&
      role == other.role;

  @override
  int get hashCode => Object.hash(ch, pattern, sourceIndex, role);

  @override
  String toString() => 'EncodedCodabarSymbol($ch, $pattern)';
}
