/// One Code 39 character with its narrow/wide width pattern.
library;

/// A single encoded Code 39 character.
final class EncodedCharacter {
  /// The character (after ASCII case folding).
  final String ch;

  /// Whether this is a start/stop delimiter.
  final bool isStartStop;

  /// The nine-element narrow/wide width pattern.
  final String pattern;

  /// Creates an encoded character.
  const EncodedCharacter({
    required this.ch,
    required this.isStartStop,
    required this.pattern,
  });

  @override
  bool operator ==(Object other) =>
      other is EncodedCharacter &&
      ch == other.ch &&
      isStartStop == other.isStartStop &&
      pattern == other.pattern;

  @override
  int get hashCode => Object.hash(ch, isStartStop, pattern);

  @override
  String toString() => 'EncodedCharacter($ch, $pattern)';
}
