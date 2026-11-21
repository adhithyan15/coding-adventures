import 'messages.dart';

/// A key-value endpoint stored in a [RadixTree].
///
/// Implements value equality and pattern matching compatibility. Diagnostic
/// string formatting is strictly payload-blind to prevent credential or payload
/// leakage (CWE-209).
final class RadixEntry<V> {
  /// Create a new entry mapping [key] to [value].
  const RadixEntry(this.key, this.value);

  /// The exact string key.
  final String key;

  /// The associated value.
  final V value;

  /// Positional getter matching Dart record pattern `(key, value).$1`.
  String get $1 => key;

  /// Positional getter matching Dart record pattern `(key, value).$2`.
  V get $2 => value;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RadixEntry<V> && key == other.key && value == other.value;

  @override
  int get hashCode => Object.hash(key, value);

  @override
  String toString() =>
      '$radixEntryStringPrefix$redactedPlaceholder$radixEntryStringSeparator$redactedPlaceholder$radixEntryStringSuffix';
}
