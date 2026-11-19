part of coding_adventures_hash_map;

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
      : _bytes = List<int>.unmodifiable(_copy(bytes)),
        super._();

  final List<int> _bytes;

  @override
  int get _tag => 0;

  @override
  List<int> get _payload => _bytes;

  static List<int> _copy(List<int> bytes) {
    for (var index = 0; index < bytes.length; index++) {
      final value = bytes[index];
      if (value < 0 || value > 255) throw InvalidByte(index, value);
    }
    return List<int>.of(bytes);
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
