/// Errors thrown by HTTP core parsing.
library;

/// Thrown when an HTTP version marker is not `HTTP/x.y` with 16-bit
/// parts. The message mirrors the reference text exactly.
final class InvalidHttpVersion implements Exception {
  final String text;
  const InvalidHttpVersion(this.text);

  @override
  bool operator ==(Object other) =>
      other is InvalidHttpVersion && text == other.text;
  @override
  int get hashCode => text.hashCode;
  @override
  String toString() => 'invalid HTTP version: $text';
}
