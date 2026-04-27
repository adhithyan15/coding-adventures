import 'dart:typed_data';

import 'package:coding_adventures_huffman_tree/huffman_tree.dart';

/// Maximum code length allowed by the CMP04 wire format.
const int maxCodeLength = 16;

/// Default cap for output length declared by untrusted compressed payloads.
const int defaultMaxDecompressedSize = 64 * 1024 * 1024;

/// Compress [data] into the CMP04 canonical Huffman wire format.
///
/// The output layout is:
///
/// - 4 bytes: original length as big-endian uint32
/// - 4 bytes: distinct symbol count as big-endian uint32
/// - `2 * symbol_count` bytes: `(symbol, code_length)` pairs sorted by
///   `(code_length, symbol)` ascending
/// - variable bytes: bit stream packed LSB-first and zero-padded to the byte
///   boundary
Uint8List compress(Uint8List data) {
  if (data.isEmpty) {
    return Uint8List(8);
  }

  final frequencies = <int, int>{};
  for (final byte in data) {
    frequencies[byte] = (frequencies[byte] ?? 0) + 1;
  }

  final tree = HuffmanTree.build(
    frequencies.entries
        .map<(int, int)>((entry) => (entry.key, entry.value))
        .toList(),
  );
  final canonicalTable = tree.canonicalCodeTable();
  final lengths = canonicalTable.entries
      .map<(int, int)>((entry) => (entry.key, entry.value.length))
      .toList()
    ..sort(
      (left, right) => left.$2 != right.$2
          ? left.$2.compareTo(right.$2)
          : left.$1.compareTo(right.$1),
    );

  for (final (_, length) in lengths) {
    if (length > maxCodeLength)
      throw StateError(
          'Huffman tree produced a code longer than $maxCodeLength bits.');
  }

  final bits = StringBuffer();
  for (final byte in data) {
    bits.write(canonicalTable[byte]!);
  }

  final bitBytes = _packBitsLsbFirst(bits.toString());
  final symbolCount = lengths.length;
  final output = Uint8List(8 + symbolCount * 2 + bitBytes.length);
  final header = ByteData.sublistView(output);
  header.setUint32(0, data.length, Endian.big);
  header.setUint32(4, symbolCount, Endian.big);

  for (var index = 0; index < symbolCount; index += 1) {
    final (symbol, length) = lengths[index];
    output[8 + index * 2] = symbol;
    output[8 + index * 2 + 1] = length;
  }
  output.setRange(8 + symbolCount * 2, output.length, bitBytes);
  return output;
}

/// Decompress a CMP04 canonical Huffman payload.
///
/// The decoder validates the header, the code-length table, the canonical code
/// space, truncated bit streams, invalid prefixes, and non-zero padding bits.
Uint8List decompress(
  Uint8List data, [
  int maxDecompressedSize = defaultMaxDecompressedSize,
]) {
  _validateMaxDecompressedSize(maxDecompressedSize);

  if (data.isEmpty) {
    return Uint8List(0);
  }
  if (data.length < 8) {
    throw const FormatException(
      'Malformed Huffman stream: header is incomplete.',
    );
  }

  final header = ByteData.sublistView(data);
  final originalLength = header.getUint32(0, Endian.big);
  final symbolCount = header.getUint32(4, Endian.big);

  if (originalLength == 0) {
    if (symbolCount != 0) {
      throw const FormatException(
        'Malformed Huffman stream: empty output cannot declare symbols.',
      );
    }
    if (data.length != 8) {
      throw const FormatException(
        'Malformed Huffman stream: empty output must not include table or bit data.',
      );
    }
    return Uint8List(0);
  }

  if (originalLength > maxDecompressedSize) {
    throw FormatException(
      'Malformed Huffman stream: declared output length $originalLength exceeds the configured limit $maxDecompressedSize.',
    );
  }
  if (symbolCount == 0) {
    throw const FormatException(
      'Malformed Huffman stream: non-empty output requires at least one symbol.',
    );
  }
  if (symbolCount > 256) {
    throw FormatException(
      'Malformed Huffman stream: symbol count $symbolCount exceeds the byte alphabet.',
    );
  }

  final tableOffset = 8;
  final tableSize = symbolCount * 2;
  final bitOffset = tableOffset + tableSize;
  if (data.length < bitOffset) {
    throw const FormatException(
      'Malformed Huffman stream: code-length table is truncated.',
    );
  }

  final lengths = <(int, int)>[];
  final seenSymbols = <int>{};
  var previousLength = -1;
  var previousSymbol = -1;
  for (var index = 0; index < symbolCount; index += 1) {
    final symbol = data[tableOffset + index * 2];
    final length = data[tableOffset + index * 2 + 1];

    if (!seenSymbols.add(symbol)) {
      throw FormatException(
        'Malformed Huffman stream: duplicate symbol $symbol in code-length table.',
      );
    }
    if (length == 0) {
      throw FormatException(
        'Malformed Huffman stream: symbol $symbol has invalid code length 0.',
      );
    }
    if (length > maxCodeLength) {
      throw FormatException(
        'Malformed Huffman stream: symbol $symbol has code length $length, which exceeds $maxCodeLength.',
      );
    }
    if (previousLength > length ||
        (previousLength == length && previousSymbol > symbol)) {
      throw const FormatException(
        'Malformed Huffman stream: code-length table must be sorted by (length, symbol).',
      );
    }

    previousLength = length;
    previousSymbol = symbol;
    lengths.add((symbol, length));
  }

  final decoder = _canonicalDecoderFromLengths(lengths);
  final bitReader = _BitReader(data.sublist(bitOffset));
  final output = Uint8List(originalLength);
  var decoded = 0;
  var currentCode = 0;
  var currentLength = 0;

  while (decoded < originalLength) {
    final bit = bitReader.readBit();
    if (bit == null) {
      throw FormatException(
        'Malformed Huffman stream: bit stream exhausted after $decoded symbols; expected $originalLength.',
      );
    }

    currentCode = (currentCode << 1) | bit;
    currentLength += 1;

    if (currentLength > decoder.maxLength) {
      throw const FormatException(
        'Malformed Huffman stream: encountered a bit prefix that matches no canonical code.',
      );
    }

    final symbol = decoder.codesByLength[currentLength]?[currentCode];
    if (symbol != null) {
      output[decoded] = symbol;
      decoded += 1;
      currentCode = 0;
      currentLength = 0;
      continue;
    }

    final canStillGrow =
        decoder.validPrefixesByLength[currentLength]?.contains(currentCode) ??
            false;
    if (!canStillGrow) {
      throw const FormatException(
        'Malformed Huffman stream: encountered a bit prefix that matches no canonical code.',
      );
    }
  }

  if (currentLength != 0) {
    throw const FormatException(
      'Malformed Huffman stream: decoding ended in the middle of a code word.',
    );
  }
  if (bitReader.hasNonZeroRemainingBits()) {
    throw const FormatException(
      'Malformed Huffman stream: trailing padding bits must be zero.',
    );
  }

  return output;
}

Uint8List _packBitsLsbFirst(String bits) {
  final output = Uint8List((bits.length + 7) ~/ 8);
  for (var index = 0; index < bits.length; index += 1) {
    final bit = bits[index];
    if (bit == '1') {
      output[index ~/ 8] |= 1 << (index % 8);
    }
  }
  return output;
}

_CanonicalDecoder _canonicalDecoderFromLengths(List<(int, int)> lengths) {
  if (lengths.isEmpty) {
    throw const FormatException(
      'Malformed Huffman stream: code-length table must not be empty.',
    );
  }

  if (lengths.length == 1) {
    final (symbol, length) = lengths.single;
    if (length != 1) {
      throw const FormatException(
        'Malformed Huffman stream: single-symbol tables must use a one-bit code.',
      );
    }
    return _CanonicalDecoder(
      <int, Map<int, int>>{
        1: <int, int>{0: symbol}
      },
      <int, Set<int>>{},
      1,
    );
  }

  final countsByLength = <int, int>{};
  var maxLength = 0;
  for (final (_, length) in lengths) {
    countsByLength[length] = (countsByLength[length] ?? 0) + 1;
    if (length > maxLength) {
      maxLength = length;
    }
  }

  var availableSlots = 1;
  for (var depth = 1; depth <= maxLength; depth += 1) {
    availableSlots <<= 1;
    availableSlots -= countsByLength[depth] ?? 0;
    if (availableSlots < 0) {
      throw const FormatException(
        'Malformed Huffman stream: canonical code lengths oversubscribe the prefix tree.',
      );
    }
  }
  if (availableSlots != 0) {
    throw const FormatException(
      'Malformed Huffman stream: canonical code lengths do not form a complete prefix tree.',
    );
  }

  final codesByLength = <int, Map<int, int>>{};
  final validPrefixesByLength = <int, Set<int>>{};
  var currentCode = 0;
  var previousLength = lengths.first.$2;
  for (final (symbol, length) in lengths) {
    if (length > previousLength) {
      currentCode <<= length - previousLength;
    }
    if (currentCode >= (1 << length)) {
      throw const FormatException(
        'Malformed Huffman stream: canonical code value overflowed its declared length.',
      );
    }
    codesByLength.putIfAbsent(length, () => <int, int>{})[currentCode] = symbol;
    for (var prefixLength = 1; prefixLength < length; prefixLength += 1) {
      final prefix = currentCode >> (length - prefixLength);
      validPrefixesByLength
          .putIfAbsent(prefixLength, () => <int>{})
          .add(prefix);
    }
    currentCode += 1;
    previousLength = length;
  }

  return _CanonicalDecoder(codesByLength, validPrefixesByLength, maxLength);
}

void _validateMaxDecompressedSize(int maxDecompressedSize) {
  if (maxDecompressedSize <= 0) {
    throw RangeError.value(
      maxDecompressedSize,
      'maxDecompressedSize',
      'Must be positive.',
    );
  }
}

final class _CanonicalDecoder {
  const _CanonicalDecoder(
    this.codesByLength,
    this.validPrefixesByLength,
    this.maxLength,
  );

  final Map<int, Map<int, int>> codesByLength;
  final Map<int, Set<int>> validPrefixesByLength;
  final int maxLength;
}

final class _BitReader {
  _BitReader(this._data);

  final Uint8List _data;
  int _byteIndex = 0;
  int _bitIndex = 0;

  int? readBit() {
    if (_byteIndex >= _data.length) {
      return null;
    }

    final bit = (_data[_byteIndex] >> _bitIndex) & 1;
    _bitIndex += 1;
    if (_bitIndex == 8) {
      _bitIndex = 0;
      _byteIndex += 1;
    }
    return bit;
  }

  bool hasNonZeroRemainingBits() {
    if (_byteIndex >= _data.length) {
      return false;
    }

    if (_bitIndex > 0) {
      final mask = ((1 << (8 - _bitIndex)) - 1) << _bitIndex;
      if ((_data[_byteIndex] & mask) != 0) {
        return true;
      }
      for (var index = _byteIndex + 1; index < _data.length; index += 1) {
        if (_data[index] != 0) {
          return true;
        }
      }
      return false;
    }

    for (var index = _byteIndex; index < _data.length; index += 1) {
      if (_data[index] != 0) {
        return true;
      }
    }
    return false;
  }
}
