import 'dart:typed_data';

class Leb128Exception implements Exception {
  Leb128Exception(this.message);

  final String message;

  @override
  String toString() => 'Leb128Exception: $message';
}

class DecodedInteger {
  const DecodedInteger(this.value, this.bytesRead);

  final int value;
  final int bytesRead;
}

DecodedInteger decodeUnsigned(List<int> data, {int offset = 0, int maxBytes = 10}) {
  var result = 0;
  var shift = 0;
  var index = offset;

  while (index < data.length) {
    final byte = data[index];
    result |= (byte & 0x7f) << shift;
    index += 1;

    if ((byte & 0x80) == 0) {
      return DecodedInteger(result, index - offset);
    }

    shift += 7;
    if (index - offset >= maxBytes) {
      throw Leb128Exception('Unsigned LEB128 exceeds $maxBytes bytes.');
    }
  }

  throw Leb128Exception('Unterminated unsigned LEB128 sequence.');
}

DecodedInteger decodeSigned(List<int> data, {int offset = 0, int maxBytes = 10, int bitWidth = 64}) {
  var result = 0;
  var shift = 0;
  var index = offset;
  var byte = 0;

  while (index < data.length) {
    byte = data[index];
    result |= (byte & 0x7f) << shift;
    shift += 7;
    index += 1;

    if ((byte & 0x80) == 0) {
      if (shift < bitWidth && (byte & 0x40) != 0) {
        result |= (-1) << shift;
      }
      return DecodedInteger(result, index - offset);
    }

    if (index - offset >= maxBytes) {
      throw Leb128Exception('Signed LEB128 exceeds $maxBytes bytes.');
    }
  }

  throw Leb128Exception('Unterminated signed LEB128 sequence.');
}

Uint8List encodeUnsigned(int value) {
  if (value < 0) {
    throw Leb128Exception('Unsigned LEB128 cannot encode negative values.');
  }

  final bytes = <int>[];
  var remaining = value;
  do {
    var byte = remaining & 0x7f;
    remaining >>= 7;
    if (remaining != 0) {
      byte |= 0x80;
    }
    bytes.add(byte);
  } while (remaining != 0);

  return Uint8List.fromList(bytes);
}

Uint8List encodeSigned(int value) {
  final bytes = <int>[];
  var remaining = value;
  var more = true;

  while (more) {
    var byte = remaining & 0x7f;
    remaining >>= 7;

    final signBitSet = (byte & 0x40) != 0;
    more = !((remaining == 0 && !signBitSet) || (remaining == -1 && signBitSet));
    if (more) {
      byte |= 0x80;
    }
    bytes.add(byte);
  }

  return Uint8List.fromList(bytes);
}
