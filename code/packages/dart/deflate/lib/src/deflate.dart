import 'dart:typed_data';

import 'package:coding_adventures_huffman_tree/huffman_tree.dart';
import 'package:coding_adventures_lzss/lzss.dart' as lzss;

/// Maximum Huffman code length accepted by the CMP05 wire format.
const int maxCodeLength = 16;

/// Default upper bound for the decompressed size declared by untrusted input.
const int defaultMaxDecompressedSize = 64 * 1024 * 1024;

const List<({int symbol, int base, int extraBits})> _lengthTable =
    <({int symbol, int base, int extraBits})>[
      (symbol: 257, base: 3, extraBits: 0),
      (symbol: 258, base: 4, extraBits: 0),
      (symbol: 259, base: 5, extraBits: 0),
      (symbol: 260, base: 6, extraBits: 0),
      (symbol: 261, base: 7, extraBits: 0),
      (symbol: 262, base: 8, extraBits: 0),
      (symbol: 263, base: 9, extraBits: 0),
      (symbol: 264, base: 10, extraBits: 0),
      (symbol: 265, base: 11, extraBits: 1),
      (symbol: 266, base: 13, extraBits: 1),
      (symbol: 267, base: 15, extraBits: 1),
      (symbol: 268, base: 17, extraBits: 1),
      (symbol: 269, base: 19, extraBits: 2),
      (symbol: 270, base: 23, extraBits: 2),
      (symbol: 271, base: 27, extraBits: 2),
      (symbol: 272, base: 31, extraBits: 2),
      (symbol: 273, base: 35, extraBits: 3),
      (symbol: 274, base: 43, extraBits: 3),
      (symbol: 275, base: 51, extraBits: 3),
      (symbol: 276, base: 59, extraBits: 3),
      (symbol: 277, base: 67, extraBits: 4),
      (symbol: 278, base: 83, extraBits: 4),
      (symbol: 279, base: 99, extraBits: 4),
      (symbol: 280, base: 115, extraBits: 4),
      (symbol: 281, base: 131, extraBits: 5),
      (symbol: 282, base: 163, extraBits: 5),
      (symbol: 283, base: 195, extraBits: 5),
      (symbol: 284, base: 227, extraBits: 5),
    ];

const List<({int code, int base, int extraBits})> _distanceTable =
    <({int code, int base, int extraBits})>[
      (code: 0, base: 1, extraBits: 0),
      (code: 1, base: 2, extraBits: 0),
      (code: 2, base: 3, extraBits: 0),
      (code: 3, base: 4, extraBits: 0),
      (code: 4, base: 5, extraBits: 1),
      (code: 5, base: 7, extraBits: 1),
      (code: 6, base: 9, extraBits: 2),
      (code: 7, base: 13, extraBits: 2),
      (code: 8, base: 17, extraBits: 3),
      (code: 9, base: 25, extraBits: 3),
      (code: 10, base: 33, extraBits: 4),
      (code: 11, base: 49, extraBits: 4),
      (code: 12, base: 65, extraBits: 5),
      (code: 13, base: 97, extraBits: 5),
      (code: 14, base: 129, extraBits: 6),
      (code: 15, base: 193, extraBits: 6),
      (code: 16, base: 257, extraBits: 7),
      (code: 17, base: 385, extraBits: 7),
      (code: 18, base: 513, extraBits: 8),
      (code: 19, base: 769, extraBits: 8),
      (code: 20, base: 1025, extraBits: 9),
      (code: 21, base: 1537, extraBits: 9),
      (code: 22, base: 2049, extraBits: 10),
      (code: 23, base: 3073, extraBits: 10),
    ];

/// Compress [data] using the educational CMP05 DEFLATE wire format.
///
/// The implementation composes the earlier Dart compression layers instead of
/// rebuilding them:
///
/// 1. LZSS tokenization to find literals and sliding-window matches.
/// 2. Canonical Huffman coding over the expanded LL and distance alphabets.
Uint8List compress(
  Uint8List data, [
  int windowSize = lzss.defaultWindowSize,
  int maxMatch = lzss.defaultMaxMatch,
  int minMatch = lzss.defaultMinMatch,
]) {
  final originalLength = data.length;
  if (originalLength == 0) {
    final output = Uint8List(12);
    final header = ByteData.sublistView(output);
    header.setUint32(0, 0, Endian.big);
    header.setUint16(4, 1, Endian.big);
    header.setUint16(6, 0, Endian.big);
    header.setUint16(8, 256, Endian.big);
    output[10] = 1;
    output[11] = 0x00;
    return output;
  }

  final tokens = lzss.encode(data, windowSize, maxMatch, minMatch);
  final llFreq = <int, int>{};
  final distFreq = <int, int>{};

  for (final token in tokens) {
    if (token is lzss.Literal) {
      llFreq[token.byte] = (llFreq[token.byte] ?? 0) + 1;
      continue;
    }

    final match = token as lzss.Match;
    final symbol = lengthSymbol(match.length);
    llFreq[symbol] = (llFreq[symbol] ?? 0) + 1;
    final distance = distCode(match.offset);
    distFreq[distance] = (distFreq[distance] ?? 0) + 1;
  }
  llFreq[256] = (llFreq[256] ?? 0) + 1;

  final llTree = HuffmanTree.build(
    llFreq.entries
        .map<(int, int)>((entry) => (entry.key, entry.value))
        .toList(),
  );
  final llCodeTable = llTree.canonicalCodeTable();
  final distCodeTable = distFreq.isEmpty
      ? const <int, String>{}
      : HuffmanTree.build(
          distFreq.entries
              .map<(int, int)>((entry) => (entry.key, entry.value))
              .toList(),
        ).canonicalCodeTable();

  final llPairs =
      llCodeTable.entries
          .map<(int, int)>((entry) => (entry.key, entry.value.length))
          .toList()
        ..sort(
          (left, right) => left.$2 != right.$2
              ? left.$2.compareTo(right.$2)
              : left.$1.compareTo(right.$1),
        );
  final distPairs =
      distCodeTable.entries
          .map<(int, int)>((entry) => (entry.key, entry.value.length))
          .toList()
        ..sort(
          (left, right) => left.$2 != right.$2
              ? left.$2.compareTo(right.$2)
              : left.$1.compareTo(right.$1),
        );

  final writer = _BitWriter();
  for (final token in tokens) {
    if (token is lzss.Literal) {
      writer.writeBitString(llCodeTable[token.byte]!);
      continue;
    }

    final match = token as lzss.Match;
    final symbol = lengthSymbol(match.length);
    writer.writeBitString(llCodeTable[symbol]!);
    final lengthExtra = lengthExtraBits(symbol);
    writer.writeRawBitsLsb(match.length - lengthBase(symbol), lengthExtra);

    final distanceSymbol = distCode(match.offset);
    writer.writeBitString(distCodeTable[distanceSymbol]!);
    final distExtra = distExtraBits(distanceSymbol);
    writer.writeRawBitsLsb(match.offset - distBase(distanceSymbol), distExtra);
  }
  writer.writeBitString(llCodeTable[256]!);
  writer.flush();
  final packedBits = writer.bytes();

  final output = Uint8List(
    8 + llPairs.length * 3 + distPairs.length * 3 + packedBits.length,
  );
  final header = ByteData.sublistView(output);
  header.setUint32(0, originalLength, Endian.big);
  header.setUint16(4, llPairs.length, Endian.big);
  header.setUint16(6, distPairs.length, Endian.big);

  var offset = 8;
  for (final (symbol, length) in llPairs) {
    header.setUint16(offset, symbol, Endian.big);
    output[offset + 2] = length;
    offset += 3;
  }
  for (final (symbol, length) in distPairs) {
    header.setUint16(offset, symbol, Endian.big);
    output[offset + 2] = length;
    offset += 3;
  }
  output.setRange(offset, output.length, packedBits);
  return output;
}

/// Decompress a CMP05 payload back into the original bytes.
///
/// The decoder validates the tables before decoding so malformed codebooks fail
/// closed instead of wandering into out-of-bounds copies or unbounded output.
Uint8List decompress(
  Uint8List data, [
  int maxDecompressedSize = defaultMaxDecompressedSize,
]) {
  _validateMaxDecompressedSize(maxDecompressedSize);

  if (data.length < 8) {
    throw const FormatException(
      'Malformed DEFLATE stream: header is incomplete.',
    );
  }

  final header = ByteData.sublistView(data);
  final originalLength = header.getUint32(0, Endian.big);
  final llEntryCount = header.getUint16(4, Endian.big);
  final distEntryCount = header.getUint16(6, Endian.big);

  if (originalLength > maxDecompressedSize) {
    throw FormatException(
      'Malformed DEFLATE stream: declared output length $originalLength exceeds the configured limit $maxDecompressedSize.',
    );
  }

  var offset = 8;
  final llLengths = <(int, int)>[];
  final seenLlSymbols = <int>{};
  var previousLlLength = -1;
  var previousLlSymbol = -1;
  for (var index = 0; index < llEntryCount; index += 1) {
    if (offset + 3 > data.length) {
      throw const FormatException(
        'Malformed DEFLATE stream: LL code-length table is truncated.',
      );
    }
    final symbol = header.getUint16(offset, Endian.big);
    final codeLength = data[offset + 2];
    _validateLlTableEntry(
      symbol,
      codeLength,
      seenLlSymbols,
      previousLlLength,
      previousLlSymbol,
    );
    llLengths.add((symbol, codeLength));
    previousLlLength = codeLength;
    previousLlSymbol = symbol;
    offset += 3;
  }

  final distLengths = <(int, int)>[];
  final seenDistSymbols = <int>{};
  var previousDistLength = -1;
  var previousDistSymbol = -1;
  for (var index = 0; index < distEntryCount; index += 1) {
    if (offset + 3 > data.length) {
      throw const FormatException(
        'Malformed DEFLATE stream: distance code-length table is truncated.',
      );
    }
    final symbol = header.getUint16(offset, Endian.big);
    final codeLength = data[offset + 2];
    _validateDistTableEntry(
      symbol,
      codeLength,
      seenDistSymbols,
      previousDistLength,
      previousDistSymbol,
    );
    distLengths.add((symbol, codeLength));
    previousDistLength = codeLength;
    previousDistSymbol = symbol;
    offset += 3;
  }

  if (originalLength == 0) {
    if (llEntryCount != 1 ||
        distEntryCount != 0 ||
        llLengths.length != 1 ||
        llLengths.single != (256, 1) ||
        data.length != offset + 1 ||
        data[offset] != 0x00) {
      throw const FormatException(
        'Malformed DEFLATE stream: empty payload must use the canonical zero-length encoding.',
      );
    }
    return Uint8List(0);
  }
  if (llLengths.isEmpty) {
    throw const FormatException(
      'Malformed DEFLATE stream: non-empty output requires an LL code table.',
    );
  }

  final llDecoder = _CanonicalDecoder.fromLengths(llLengths, streamLabel: 'LL');
  final distDecoder = distLengths.isEmpty
      ? null
      : _CanonicalDecoder.fromLengths(distLengths, streamLabel: 'distance');
  final bitReader = _BitReader(data.sublist(offset));
  final output = <int>[];

  while (true) {
    final llSymbol = llDecoder.readSymbol(bitReader);
    if (llSymbol == 256) {
      break;
    }

    if (llSymbol < 256) {
      if (output.length + 1 > originalLength) {
        throw FormatException(
          'Malformed DEFLATE stream: decoded output exceeds declared length $originalLength.',
        );
      }
      output.add(llSymbol);
      continue;
    }

    if (distDecoder == null) {
      throw const FormatException(
        'Malformed DEFLATE stream: length symbol requires a distance tree.',
      );
    }

    final matchLength =
        lengthBase(llSymbol) +
        bitReader.readRawBitsLsb(
          lengthExtraBits(llSymbol),
          context: 'length extra bits',
        );
    final distSymbol = distDecoder.readSymbol(bitReader);
    final matchOffset =
        distBase(distSymbol) +
        bitReader.readRawBitsLsb(
          distExtraBits(distSymbol),
          context: 'distance extra bits',
        );

    if (matchOffset <= 0 || matchOffset > output.length) {
      throw FormatException(
        'Malformed DEFLATE stream: match offset $matchOffset is invalid for output length ${output.length}.',
      );
    }
    if (output.length + matchLength > originalLength) {
      throw FormatException(
        'Malformed DEFLATE stream: decoded output exceeds declared length $originalLength.',
      );
    }

    final start = output.length - matchOffset;
    for (var index = 0; index < matchLength; index += 1) {
      output.add(output[start + index]);
    }
  }

  if (output.length != originalLength) {
    throw FormatException(
      'Malformed DEFLATE stream: decoded output length ${output.length} does not match declared length $originalLength.',
    );
  }
  if (bitReader.hasNonZeroRemainingBits()) {
    throw const FormatException(
      'Malformed DEFLATE stream: trailing padding bits must be zero.',
    );
  }

  return Uint8List.fromList(output);
}

/// Map a literal-match length in `3..255` to its LL Huffman symbol.
int lengthSymbol(int length) {
  if (length < 3 || length > 255) {
    throw RangeError.value(length, 'length', 'Must be in 3..255.');
  }
  for (final entry in _lengthTable) {
    final maxLength = entry.base + (1 << entry.extraBits) - 1;
    if (length <= maxLength) {
      return entry.symbol;
    }
  }
  return 284;
}

/// Map a backreference distance in `1..4096` to its DEFLATE distance code.
int distCode(int offset) {
  if (offset < 1 || offset > 4096) {
    throw RangeError.value(offset, 'offset', 'Must be in 1..4096.');
  }
  for (final entry in _distanceTable) {
    final maxDistance = entry.base + (1 << entry.extraBits) - 1;
    if (offset <= maxDistance) {
      return entry.code;
    }
  }
  return 23;
}

int lengthBase(int symbol) => _findLengthEntry(symbol).base;

int lengthExtraBits(int symbol) => _findLengthEntry(symbol).extraBits;

int distBase(int code) => _findDistanceEntry(code).base;

int distExtraBits(int code) => _findDistanceEntry(code).extraBits;

({int symbol, int base, int extraBits}) _findLengthEntry(int symbol) {
  for (final entry in _lengthTable) {
    if (entry.symbol == symbol) {
      return entry;
    }
  }
  throw RangeError.value(symbol, 'symbol', 'Unknown length symbol.');
}

({int code, int base, int extraBits}) _findDistanceEntry(int code) {
  for (final entry in _distanceTable) {
    if (entry.code == code) {
      return entry;
    }
  }
  throw RangeError.value(code, 'code', 'Unknown distance code.');
}

void _validateLlTableEntry(
  int symbol,
  int codeLength,
  Set<int> seenSymbols,
  int previousLength,
  int previousSymbol,
) {
  if (symbol > 284) {
    throw FormatException(
      'Malformed DEFLATE stream: LL symbol $symbol is outside 0..284.',
    );
  }
  _validateCodeLength(codeLength, streamLabel: 'LL', symbol: symbol);
  _validateSortedUniqueEntry(
    symbol,
    codeLength,
    seenSymbols,
    previousLength,
    previousSymbol,
    streamLabel: 'LL',
  );
}

void _validateDistTableEntry(
  int symbol,
  int codeLength,
  Set<int> seenSymbols,
  int previousLength,
  int previousSymbol,
) {
  if (symbol > 23) {
    throw FormatException(
      'Malformed DEFLATE stream: distance symbol $symbol is outside 0..23.',
    );
  }
  _validateCodeLength(codeLength, streamLabel: 'distance', symbol: symbol);
  _validateSortedUniqueEntry(
    symbol,
    codeLength,
    seenSymbols,
    previousLength,
    previousSymbol,
    streamLabel: 'distance',
  );
}

void _validateCodeLength(
  int codeLength, {
  required String streamLabel,
  required int symbol,
}) {
  if (codeLength == 0) {
    throw FormatException(
      'Malformed DEFLATE stream: $streamLabel symbol $symbol has code length 0.',
    );
  }
  if (codeLength > maxCodeLength) {
    throw FormatException(
      'Malformed DEFLATE stream: $streamLabel symbol $symbol has code length $codeLength, which exceeds $maxCodeLength.',
    );
  }
}

void _validateSortedUniqueEntry(
  int symbol,
  int codeLength,
  Set<int> seenSymbols,
  int previousLength,
  int previousSymbol, {
  required String streamLabel,
}) {
  if (!seenSymbols.add(symbol)) {
    throw FormatException(
      'Malformed DEFLATE stream: duplicate $streamLabel symbol $symbol in code-length table.',
    );
  }
  if (previousLength > codeLength ||
      (previousLength == codeLength && previousSymbol > symbol)) {
    throw FormatException(
      'Malformed DEFLATE stream: $streamLabel code-length table must be sorted by (length, symbol).',
    );
  }
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
  const _CanonicalDecoder._(
    this.codesByLength,
    this.validPrefixesByLength,
    this.maxLength,
    this.streamLabel,
  );

  final Map<int, Map<int, int>> codesByLength;
  final Map<int, Set<int>> validPrefixesByLength;
  final int maxLength;
  final String streamLabel;

  static _CanonicalDecoder fromLengths(
    List<(int, int)> lengths, {
    required String streamLabel,
  }) {
    if (lengths.length == 1) {
      final (symbol, length) = lengths.single;
      if (length != 1) {
        throw FormatException(
          'Malformed DEFLATE stream: single-symbol $streamLabel tables must use a one-bit code.',
        );
      }
      return _CanonicalDecoder._(
        <int, Map<int, int>>{
          1: <int, int>{0: symbol},
        },
        <int, Set<int>>{},
        1,
        streamLabel,
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
        throw FormatException(
          'Malformed DEFLATE stream: $streamLabel code lengths oversubscribe the prefix tree.',
        );
      }
    }
    if (availableSlots != 0) {
      throw FormatException(
        'Malformed DEFLATE stream: $streamLabel code lengths do not form a complete prefix tree.',
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

      codesByLength.putIfAbsent(length, () => <int, int>{})[currentCode] =
          symbol;
      for (var prefixLength = 1; prefixLength < length; prefixLength += 1) {
        final prefix = currentCode >> (length - prefixLength);
        validPrefixesByLength
            .putIfAbsent(prefixLength, () => <int>{})
            .add(prefix);
      }

      currentCode += 1;
      previousLength = length;
    }

    return _CanonicalDecoder._(
      codesByLength,
      validPrefixesByLength,
      maxLength,
      streamLabel,
    );
  }

  int readSymbol(_BitReader reader) {
    var currentCode = 0;
    var currentLength = 0;

    while (true) {
      final bit = reader.readBit();
      if (bit == null) {
        throw FormatException(
          'Malformed DEFLATE stream: bit stream exhausted while reading a $streamLabel Huffman symbol.',
        );
      }

      currentCode = (currentCode << 1) | bit;
      currentLength += 1;

      final symbol = codesByLength[currentLength]?[currentCode];
      if (symbol != null) {
        return symbol;
      }

      final canStillGrow =
          validPrefixesByLength[currentLength]?.contains(currentCode) ?? false;
      if (!canStillGrow) {
        throw FormatException(
          'Malformed DEFLATE stream: encountered a bit prefix that matches no $streamLabel canonical code.',
        );
      }
    }
  }
}

final class _BitWriter {
  final List<int> _bytes = <int>[];
  int _buffer = 0;
  int _bitCount = 0;

  void writeBitString(String bits) {
    for (final bit in bits.codeUnits) {
      if (bit == 0x31) {
        _buffer |= 1 << _bitCount;
      }
      _bitCount += 1;
      if (_bitCount == 8) {
        _bytes.add(_buffer & 0xff);
        _buffer = 0;
        _bitCount = 0;
      }
    }
  }

  void writeRawBitsLsb(int value, int width) {
    if (width == 0) {
      return;
    }
    if (value < 0 || value >= (1 << width)) {
      throw RangeError.value(
        value,
        'value',
        'Must fit within $width raw bits.',
      );
    }

    for (var bit = 0; bit < width; bit += 1) {
      if (((value >> bit) & 1) == 1) {
        _buffer |= 1 << _bitCount;
      }
      _bitCount += 1;
      if (_bitCount == 8) {
        _bytes.add(_buffer & 0xff);
        _buffer = 0;
        _bitCount = 0;
      }
    }
  }

  void flush() {
    if (_bitCount > 0) {
      _bytes.add(_buffer & 0xff);
      _buffer = 0;
      _bitCount = 0;
    }
  }

  Uint8List bytes() => Uint8List.fromList(_bytes);
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

  int readRawBitsLsb(int width, {required String context}) {
    var value = 0;
    for (var bit = 0; bit < width; bit += 1) {
      final current = readBit();
      if (current == null) {
        throw FormatException(
          'Malformed DEFLATE stream: bit stream exhausted while reading $context.',
        );
      }
      value |= current << bit;
    }
    return value;
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
