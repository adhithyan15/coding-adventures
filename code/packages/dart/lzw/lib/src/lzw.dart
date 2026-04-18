import 'dart:typed_data';

/// Instructs the decoder to reset the dictionary to its initial 256-entry state.
const int clearCode = 256;

/// Marks the end of a logical LZW code stream.
const int stopCode = 257;

/// First dynamically assigned dictionary code after the two reserved control codes.
const int initialNextCode = 258;

/// Starting bit width for code packing and unpacking.
const int initialCodeSize = 9;

/// Maximum bit width for packed codes.
const int maxCodeSize = 16;

/// Default upper bound for output declared by untrusted compressed payloads.
const int defaultMaxDecompressedSize = 64 * 1024 * 1024;

/// Writes variable-width codes into a byte stream, LSB-first.
///
/// LZW's packed wire format fills the least-significant bits of each byte
/// first. That matches GIF and the Unix `compress` family.
class BitWriter {
  final List<int> _bytes = <int>[];
  int _buffer = 0;
  int _bitCount = 0;

  /// Appends [code] using exactly [codeSize] bits.
  void write(int code, int codeSize) {
    if (codeSize <= 0 || codeSize > maxCodeSize) {
      throw RangeError.value(
        codeSize,
        'codeSize',
        'Must be in 1..$maxCodeSize.',
      );
    }
    if (code < 0 || code >= (1 << codeSize)) {
      throw RangeError.value(code, 'code', 'Must fit within $codeSize bits.');
    }

    _buffer |= code << _bitCount;
    _bitCount += codeSize;

    while (_bitCount >= 8) {
      _bytes.add(_buffer & 0xff);
      _buffer >>= 8;
      _bitCount -= 8;
    }
  }

  /// Flushes the final partial byte, padding with zeros to the byte boundary.
  void flush() {
    if (_bitCount > 0) {
      _bytes.add(_buffer & 0xff);
      _buffer = 0;
      _bitCount = 0;
    }
  }

  /// Returns the emitted bytes so far.
  Uint8List bytes() => Uint8List.fromList(_bytes);
}

/// Reads variable-width codes from a byte stream, LSB-first.
class BitReader {
  /// Creates a reader over the provided packed byte payload.
  BitReader(this._data);

  final Uint8List _data;
  int _position = 0;
  int _buffer = 0;
  int _bitCount = 0;

  /// Reads the next [codeSize]-bit code.
  int read(int codeSize) {
    if (codeSize <= 0 || codeSize > maxCodeSize) {
      throw RangeError.value(
        codeSize,
        'codeSize',
        'Must be in 1..$maxCodeSize.',
      );
    }

    while (_bitCount < codeSize) {
      if (_position >= _data.length) {
        throw const FormatException(
          'Malformed LZW bit stream: unexpected end of input.',
        );
      }

      _buffer |= _data[_position] << _bitCount;
      _position += 1;
      _bitCount += 8;
    }

    final mask = (1 << codeSize) - 1;
    final code = _buffer & mask;
    _buffer >>= codeSize;
    _bitCount -= codeSize;
    return code;
  }

  /// Whether extra unread bytes remain after the current buffered bits.
  bool get hasUnreadBytes => _position < _data.length;

  /// Whether the unread buffered bits contain any non-zero padding.
  bool get hasNonZeroPaddingBits => _buffer != 0;
}

/// Encodes [data] into a logical LZW code stream including CLEAR and STOP.
List<int> encodeCodes(Uint8List data) {
  final dictionary = <String, int>{};
  for (var byte = 0; byte < 256; byte += 1) {
    dictionary[String.fromCharCode(byte)] = byte;
  }

  final codes = <int>[clearCode];
  final maxEntries = 1 << maxCodeSize;
  var nextCode = initialNextCode;
  var current = '';

  for (final byte in data) {
    final character = String.fromCharCode(byte);
    final extended = '$current$character';

    if (dictionary.containsKey(extended)) {
      current = extended;
      continue;
    }

    codes.add(dictionary[current]!);

    if (nextCode < maxEntries) {
      dictionary[extended] = nextCode;
      nextCode += 1;
    } else {
      codes.add(clearCode);
      dictionary
        ..clear()
        ..addEntries(
          Iterable<MapEntry<String, int>>.generate(
            256,
            (index) => MapEntry<String, int>(String.fromCharCode(index), index),
          ),
        );
      nextCode = initialNextCode;
    }

    current = character;
  }

  if (current.isNotEmpty) {
    codes.add(dictionary[current]!);
  }

  codes.add(stopCode);
  return List<int>.unmodifiable(codes);
}

/// Decodes a logical LZW code stream back into the original bytes.
///
/// If [originalLength] is supplied, the decoded output must match it exactly.
Uint8List decodeCodes(List<int> codes, [int originalLength = -1]) {
  if (originalLength < -1) {
    throw RangeError.value(
      originalLength,
      'originalLength',
      'Must be -1 or a non-negative byte length.',
    );
  }
  if (codes.isEmpty) {
    throw const FormatException(
      'Malformed LZW code stream: at least CLEAR_CODE and STOP_CODE are required.',
    );
  }
  if (codes.first != clearCode) {
    throw FormatException(
      'Malformed LZW code stream: expected CLEAR_CODE ($clearCode) at start, got ${codes.first}.',
    );
  }

  final dictionary = _initialDecodeDictionary();
  var nextCode = initialNextCode;
  int? previousCode;
  var sawStop = false;
  final output = <int>[];

  for (var index = 1; index < codes.length; index += 1) {
    final code = codes[index];
    _validateCodeValue(code);

    if (code == clearCode) {
      _resetDecodeDictionary(dictionary);
      nextCode = initialNextCode;
      previousCode = null;
      continue;
    }

    if (code == stopCode) {
      if (index != codes.length - 1) {
        throw const FormatException(
          'Malformed LZW code stream: codes appear after STOP_CODE.',
        );
      }
      sawStop = true;
      break;
    }

    final Uint8List entry;
    if (code < dictionary.length) {
      entry = dictionary[code];
    } else if (code == nextCode) {
      if (previousCode == null) {
        throw const FormatException(
          'Malformed LZW code stream: tricky token appeared without a previous code.',
        );
      }

      final previousEntry = dictionary[previousCode];
      entry = Uint8List(previousEntry.length + 1)
        ..setAll(0, previousEntry)
        ..[previousEntry.length] = previousEntry[0];
    } else {
      throw FormatException(
        'Malformed LZW code stream: invalid code $code for dictionary size ${dictionary.length}.',
      );
    }

    if (originalLength >= 0 && output.length + entry.length > originalLength) {
      throw FormatException(
        'Malformed LZW code stream: decoded output exceeds declared length $originalLength.',
      );
    }
    output.addAll(entry);

    if (previousCode != null && nextCode < (1 << maxCodeSize)) {
      final previousEntry = dictionary[previousCode];
      final newEntry = Uint8List(previousEntry.length + 1)
        ..setAll(0, previousEntry)
        ..[previousEntry.length] = entry[0];
      dictionary.add(newEntry);
      nextCode += 1;
    }

    previousCode = code;
  }

  if (!sawStop) {
    throw const FormatException(
      'Malformed LZW code stream: missing STOP_CODE terminator.',
    );
  }
  if (originalLength >= 0 && output.length != originalLength) {
    throw FormatException(
      'Malformed LZW code stream: decoded output length ${output.length} does not match declared length $originalLength.',
    );
  }

  return Uint8List.fromList(output);
}

/// Packs logical LZW [codes] into the CMP03 wire format.
Uint8List packCodes(List<int> codes, int originalLength) {
  _validateOriginalLength(originalLength);
  if (codes.isEmpty) {
    throw const FormatException(
      'Malformed LZW code stream: at least CLEAR_CODE and STOP_CODE are required.',
    );
  }
  if (codes.first != clearCode) {
    throw FormatException(
      'Malformed LZW code stream: expected CLEAR_CODE ($clearCode) at start, got ${codes.first}.',
    );
  }
  if (codes.last != stopCode) {
    throw FormatException(
      'Malformed LZW code stream: expected STOP_CODE ($stopCode) at end, got ${codes.last}.',
    );
  }

  final writer = BitWriter();
  var codeSize = initialCodeSize;
  var nextCode = initialNextCode;

  for (var index = 0; index < codes.length; index += 1) {
    final code = codes[index];
    _validateCodeValue(code);
    if (code >= (1 << codeSize)) {
      throw FormatException(
        'Malformed LZW code stream: code $code at index $index does not fit in $codeSize bits.',
      );
    }

    writer.write(code, codeSize);

    if (code == clearCode) {
      codeSize = initialCodeSize;
      nextCode = initialNextCode;
    } else if (code != stopCode && nextCode < (1 << maxCodeSize)) {
      nextCode += 1;
      if (nextCode > (1 << codeSize) && codeSize < maxCodeSize) {
        codeSize += 1;
      }
    }
  }

  writer.flush();
  final body = writer.bytes();
  final result = Uint8List(4 + body.length);
  final view = ByteData.sublistView(result);
  view.setUint32(0, originalLength, Endian.big);
  result.setRange(4, result.length, body);
  return result;
}

/// Unpacks CMP03 wire-format bytes into a logical LZW code stream.
({List<int> codes, int originalLength}) unpackCodes(
  Uint8List data, [
  int maxOriginalLength = defaultMaxDecompressedSize,
]) {
  _validateMaxDecompressedSize(maxOriginalLength);
  if (data.length < 4) {
    throw const FormatException(
      'Malformed LZW bit stream: header is incomplete.',
    );
  }

  final view = ByteData.view(
    data.buffer,
    data.offsetInBytes,
    data.lengthInBytes,
  );
  final originalLength = view.getUint32(0, Endian.big);
  if (originalLength > maxOriginalLength) {
    throw FormatException(
      'Malformed LZW bit stream: declared output length $originalLength exceeds limit $maxOriginalLength.',
    );
  }

  final reader = BitReader(Uint8List.sublistView(data, 4));
  final codes = <int>[];
  var codeSize = initialCodeSize;
  var nextCode = initialNextCode;

  final firstCode = _readPackedCode(
    reader,
    codeSize,
    'Malformed LZW bit stream: missing CLEAR_CODE at start.',
  );
  if (firstCode != clearCode) {
    throw FormatException(
      'Malformed LZW bit stream: expected CLEAR_CODE ($clearCode) at start, got $firstCode.',
    );
  }
  codes.add(firstCode);

  while (true) {
    final code = _readPackedCode(
      reader,
      codeSize,
      'Malformed LZW bit stream: unexpected end before STOP_CODE.',
    );
    _validateCodeValue(code);
    codes.add(code);

    if (code == stopCode) {
      break;
    }

    if (code == clearCode) {
      codeSize = initialCodeSize;
      nextCode = initialNextCode;
      continue;
    }

    if (nextCode < (1 << maxCodeSize)) {
      nextCode += 1;
      if (nextCode > (1 << codeSize) && codeSize < maxCodeSize) {
        codeSize += 1;
      }
    }
  }

  if (reader.hasUnreadBytes) {
    throw const FormatException(
      'Malformed LZW bit stream: trailing bytes remain after STOP_CODE.',
    );
  }
  if (reader.hasNonZeroPaddingBits) {
    throw const FormatException(
      'Malformed LZW bit stream: non-zero padding bits remain after STOP_CODE.',
    );
  }

  return (codes: List<int>.unmodifiable(codes), originalLength: originalLength);
}

/// Compresses [data] to the packed CMP03 wire format.
Uint8List compress(Uint8List data) => packCodes(encodeCodes(data), data.length);

/// Decompresses packed CMP03 bytes back into the original data.
Uint8List decompress(
  Uint8List data, [
  int maxDecompressedSize = defaultMaxDecompressedSize,
]) {
  final unpacked = unpackCodes(data, maxDecompressedSize);
  return decodeCodes(unpacked.codes, unpacked.originalLength);
}

int _readPackedCode(BitReader reader, int codeSize, String message) {
  try {
    return reader.read(codeSize);
  } on FormatException {
    throw FormatException(message);
  }
}

List<Uint8List> _initialDecodeDictionary() {
  final dictionary = List<Uint8List>.generate(
    256,
    (index) => Uint8List.fromList(<int>[index]),
    growable: true,
  );
  dictionary
    ..add(Uint8List(0))
    ..add(Uint8List(0));
  return dictionary;
}

void _resetDecodeDictionary(List<Uint8List> dictionary) {
  dictionary
    ..clear()
    ..addAll(_initialDecodeDictionary());
}

void _validateCodeValue(int code) {
  if (code < 0 || code >= (1 << maxCodeSize)) {
    throw FormatException(
      'Malformed LZW code stream: code must be in 0..65535, got $code.',
    );
  }
}

void _validateOriginalLength(int originalLength) {
  if (originalLength < 0) {
    throw RangeError.value(
      originalLength,
      'originalLength',
      'Must be a non-negative byte length.',
    );
  }
}

void _validateMaxDecompressedSize(int maxDecompressedSize) {
  if (maxDecompressedSize <= 0) {
    throw RangeError.value(
      maxDecompressedSize,
      'maxDecompressedSize',
      'Must be a positive byte limit.',
    );
  }
}
