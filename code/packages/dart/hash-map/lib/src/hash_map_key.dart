part of coding_adventures_hash_map;

/// A key's identity includes its type tag as well as its payload bytes.
///
/// The serialized form is `[tag][u64 little-endian payload length][payload]`.
/// This makes string and raw-byte keys distinct even when their payloads match.
sealed class HashMapKey {
  HashMapKey._();

  factory HashMapKey.bytes(List<int> bytes) => _ByteHashMapKey(bytes);
  factory HashMapKey.string(String value) => _StringHashMapKey(value);

  List<int> get encodedBytes {
    final payload = _payload;
    final length = BigInt.from(payload.length);
    final encoded = <int>[_tag];
    for (var shift = 0; shift < 64; shift += 8) {
      encoded.add(((length >> shift) & BigInt.from(0xff)).toInt());
    }
    encoded.addAll(payload);
    return encoded;
  }

  int get _tag;
  List<int> get _payload;
  int get _payloadByteCount => _payload.length;

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other is! HashMapKey || _tag != other._tag) return false;
    final left = _payload;
    final right = other._payload;
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (left[index] != right[index]) return false;
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(_tag, Object.hashAll(_payload));
}

final class _ByteHashMapKey extends HashMapKey {
  _ByteHashMapKey(List<int> bytes)
      : _bytes = List<int>.unmodifiable(_snapshotAndValidate(bytes)),
        super._();

  final List<int> _bytes;

  @override
  int get _tag => 0;

  @override
  List<int> get _payload => _bytes;

  static List<int> _snapshotAndValidate(List<int> bytes) {
    // Validate the exact owned snapshot. A caller-supplied List implementation
    // may compute values when read, so validating it and copying it separately
    // could admit different bytes on the second pass.
    final snapshot = List<int>.of(bytes);
    for (var index = 0; index < snapshot.length; index++) {
      final value = snapshot[index];
      if (value < 0 || value > 255) throw InvalidByte(index, value);
    }
    return snapshot;
  }
}

final class _StringHashMapKey extends HashMapKey {
  _StringHashMapKey(String value)
      : _bytes = List<int>.unmodifiable(_encode(value)),
        super._();

  final List<int> _bytes;

  @override
  int get _tag => 1;

  @override
  List<int> get _payload => _bytes;

  // Validate UTF-16 before asking the UTF-8 encoder to avoid replacement of
  // malformed surrogate code units with the same bytes as a different string.
  static List<int> _encode(String value) {
    final units = value.codeUnits;
    for (var index = 0; index < units.length; index++) {
      final unit = units[index];
      if (unit >= 0xd800 && unit <= 0xdbff) {
        if (index + 1 >= units.length ||
            units[index + 1] < 0xdc00 ||
            units[index + 1] > 0xdfff) {
          throw MalformedUtf16(index);
        }
        index++;
      } else if (unit >= 0xdc00 && unit <= 0xdfff) {
        throw MalformedUtf16(index);
      }
    }
    return utf8.encode(value);
  }
}
