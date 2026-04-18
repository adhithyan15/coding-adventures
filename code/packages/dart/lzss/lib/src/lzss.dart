import 'dart:typed_data';

/// Default sliding-window size used by the CMP02 specification.
const int defaultWindowSize = 4096;

/// Default maximum match length used by the CMP02 specification.
const int defaultMaxMatch = 255;

/// Default minimum profitable match length used by the CMP02 specification.
const int defaultMinMatch = 3;

/// Default upper bound for output declared by untrusted compressed payloads.
///
/// LZSS is cheap to parse but potentially expensive to expand. Capping the
/// declared output size keeps a hostile header from forcing the decoder to
/// allocate an arbitrarily large result.
const int defaultMaxDecompressedSize = 64 * 1024 * 1024;

/// A single LZSS token in the teaching-friendly token stream.
///
/// LZSS alternates between two ideas:
/// - emit a [Literal] when the encoder cannot find a worthwhile match
/// - emit a [Match] when it can copy bytes from the already-emitted prefix
sealed class Token {
  const Token();
}

/// A single literal byte copied directly into the output.
class Literal extends Token {
  /// Creates a literal token that carries one byte from the source stream.
  const Literal(this.byte);

  /// The literal byte value in the range `0..255`.
  final int byte;

  @override
  bool operator ==(Object other) => other is Literal && other.byte == byte;

  @override
  int get hashCode => byte.hashCode;

  @override
  String toString() => 'Literal(byte: $byte)';
}

/// A backreference that copies bytes from the already-decoded prefix.
class Match extends Token {
  /// Creates a match token that copies [length] bytes from [offset] bytes back.
  const Match(this.offset, this.length);

  /// Distance backward from the current output position.
  final int offset;

  /// Number of bytes to copy, one byte at a time for overlap safety.
  final int length;

  @override
  bool operator ==(Object other) =>
      other is Match && other.offset == offset && other.length == length;

  @override
  int get hashCode => Object.hash(offset, length);

  @override
  String toString() => 'Match(offset: $offset, length: $length)';
}

/// Creates a [Literal] without needing to call the constructor directly.
Literal literal(int byte) => Literal(byte);

/// Creates a [Match] without needing to call the constructor directly.
Match match(int offset, int length) => Match(offset, length);

/// Finds the longest match for `data[cursor:]` inside the current search window.
///
/// The byte-by-byte comparison intentionally allows overlap. That means an
/// input such as `AAAAAAA` can be described as one literal plus a match that
/// keeps reading from bytes that were themselves just copied.
({int offset, int length}) _findLongestMatch(
  Uint8List data,
  int cursor,
  int windowStart,
  int maxMatch,
) {
  var bestOffset = 0;
  var bestLength = 0;
  final lookaheadEnd = cursor + maxMatch < data.length
      ? cursor + maxMatch
      : data.length;

  for (var position = windowStart; position < cursor; position += 1) {
    var length = 0;
    while (cursor + length < lookaheadEnd &&
        data[position + length] == data[cursor + length]) {
      length += 1;
    }

    if (length > bestLength) {
      bestLength = length;
      bestOffset = cursor - position;
    }
  }

  return (offset: bestOffset, length: bestLength);
}

/// Encodes [data] into an LZSS token stream.
///
/// At each cursor position the encoder looks backward up to [windowSize]
/// bytes, finds the longest match, and compares it against [minMatch]. Short
/// matches are not worth the three-byte match payload, so they stay literals.
List<Token> encode(
  Uint8List data, [
  int windowSize = defaultWindowSize,
  int maxMatch = defaultMaxMatch,
  int minMatch = defaultMinMatch,
]) {
  _validateEncoderParameters(windowSize, maxMatch, minMatch);

  final tokens = <Token>[];
  var cursor = 0;

  while (cursor < data.length) {
    final windowStart = cursor - windowSize > 0 ? cursor - windowSize : 0;
    final current = _findLongestMatch(data, cursor, windowStart, maxMatch);

    if (current.length >= minMatch) {
      tokens.add(match(current.offset, current.length));
      cursor += current.length;
    } else {
      tokens.add(literal(data[cursor]));
      cursor += 1;
    }
  }

  return List<Token>.unmodifiable(tokens);
}

/// Decodes an LZSS token stream back into the original bytes.
///
/// The decoder copies match bytes one at a time so overlapping backreferences
/// behave exactly like the textbook algorithm. That is the trick that turns
/// `Literal('A') + Match(1, 6)` into `AAAAAAA`.
Uint8List decode(List<Token> tokens, [int originalLength = -1]) {
  if (originalLength < -1) {
    throw RangeError.value(
      originalLength,
      'originalLength',
      'Must be -1 or a non-negative byte length.',
    );
  }

  final output = <int>[];

  for (final current in tokens) {
    if (current is Literal) {
      _validateLiteral(current);
      if (originalLength >= 0 && output.length + 1 > originalLength) {
        throw FormatException(
          'Malformed LZSS token stream: literal exceeds declared length $originalLength.',
        );
      }

      output.add(current.byte);
      continue;
    }

    final currentMatch = current as Match;
    _validateDecodedMatch(currentMatch, output.length);
    if (originalLength >= 0 &&
        output.length + currentMatch.length > originalLength) {
      throw FormatException(
        'Malformed LZSS token stream: match exceeds declared length $originalLength.',
      );
    }

    final start = output.length - currentMatch.offset;
    for (var index = 0; index < currentMatch.length; index += 1) {
      output.add(output[start + index]);
    }
  }

  if (originalLength >= 0 && output.length != originalLength) {
    throw FormatException(
      'Malformed LZSS token stream: decoded output length ${output.length} does not match declared length $originalLength.',
    );
  }

  return Uint8List.fromList(output);
}

/// Serialises [tokens] to the CMP02 wire format.
///
/// Layout:
/// - 4 bytes: original length as big-endian uint32
/// - 4 bytes: block count as big-endian uint32
/// - repeated blocks:
///   - 1 byte flag field where bit `0` describes the first symbol
///   - 1 byte for each literal or 3 bytes for each match
Uint8List serialiseTokens(List<Token> tokens, int originalLength) {
  _validateOriginalLength(originalLength);

  final blocks = <Uint8List>[];
  for (var start = 0; start < tokens.length; start += 8) {
    final end = start + 8 < tokens.length ? start + 8 : tokens.length;
    final chunk = tokens.sublist(start, end);
    var flag = 0;
    final symbolBytes = <int>[];

    for (var bit = 0; bit < chunk.length; bit += 1) {
      final current = chunk[bit];
      if (current is Literal) {
        _validateLiteral(current);
        symbolBytes.add(current.byte);
        continue;
      }

      final currentMatch = current as Match;
      _validateSerialisedMatch(currentMatch);
      flag |= 1 << bit;
      symbolBytes.addAll(<int>[
        (currentMatch.offset >> 8) & 0xff,
        currentMatch.offset & 0xff,
        currentMatch.length & 0xff,
      ]);
    }

    blocks.add(Uint8List.fromList(<int>[flag, ...symbolBytes]));
  }

  final bodyLength = blocks.fold<int>(0, (sum, block) => sum + block.length);
  final bytes = Uint8List(8 + bodyLength);
  final view = ByteData.sublistView(bytes);
  view.setUint32(0, originalLength, Endian.big);
  view.setUint32(4, blocks.length, Endian.big);

  var cursor = 8;
  for (final block in blocks) {
    bytes.setRange(cursor, cursor + block.length, block);
    cursor += block.length;
  }

  return bytes;
}

/// Deserialises CMP02 bytes back into tokens and the declared output length.
///
/// The header tells us how many blocks to expect, while [originalLength] tells
/// us when the final partial block has described enough output. Together these
/// checks let the parser fail closed on truncated, padded, or overlong data.
({List<Token> tokens, int originalLength}) deserialiseTokens(
  Uint8List data, [
  int maxOriginalLength = defaultMaxDecompressedSize,
]) {
  _validateMaxDecompressedSize(maxOriginalLength);

  if (data.length < 8) {
    throw const FormatException(
      'Malformed LZSS token stream: header is incomplete.',
    );
  }

  final view = ByteData.view(
    data.buffer,
    data.offsetInBytes,
    data.lengthInBytes,
  );
  final originalLength = view.getUint32(0, Endian.big);
  final blockCount = view.getUint32(4, Endian.big);
  if (originalLength > maxOriginalLength) {
    throw FormatException(
      'Malformed LZSS token stream: declared output length $originalLength exceeds limit $maxOriginalLength.',
    );
  }

  if (originalLength == 0) {
    if (blockCount != 0 || data.length != 8) {
      throw const FormatException(
        'Malformed LZSS token stream: empty output must not declare blocks or payload bytes.',
      );
    }

    return (tokens: const <Token>[], originalLength: 0);
  }

  final tokens = <Token>[];
  var cursor = 8;
  var producedLength = 0;

  for (var blockIndex = 0; blockIndex < blockCount; blockIndex += 1) {
    if (cursor >= data.length) {
      throw FormatException(
        'Malformed LZSS token stream: missing flag byte for block $blockIndex.',
      );
    }

    final flag = data[cursor];
    cursor += 1;

    for (var bit = 0; bit < 8; bit += 1) {
      if (producedLength == originalLength) {
        if (blockIndex != blockCount - 1 || cursor != data.length) {
          throw const FormatException(
            'Malformed LZSS token stream: trailing data appears after the declared output length.',
          );
        }

        return (
          tokens: List<Token>.unmodifiable(tokens),
          originalLength: originalLength,
        );
      }

      if ((flag & (1 << bit)) == 0) {
        if (cursor >= data.length) {
          throw FormatException(
            'Malformed LZSS token stream: block $blockIndex ends before literal $bit.',
          );
        }

        tokens.add(literal(data[cursor]));
        cursor += 1;
        producedLength += 1;
      } else {
        if (cursor + 3 > data.length) {
          throw FormatException(
            'Malformed LZSS token stream: block $blockIndex ends in the middle of a match token.',
          );
        }

        final current = match(
          view.getUint16(cursor, Endian.big),
          view.getUint8(cursor + 2),
        );
        _validateSerialisedMatch(current);

        tokens.add(current);
        cursor += 3;
        producedLength += current.length;
      }

      if (producedLength > originalLength) {
        throw FormatException(
          'Malformed LZSS token stream: decoded length would exceed declared length $originalLength.',
        );
      }
    }
  }

  if (producedLength != originalLength) {
    throw FormatException(
      'Malformed LZSS token stream: decoded length $producedLength does not match declared length $originalLength.',
    );
  }
  if (cursor != data.length) {
    throw const FormatException(
      'Malformed LZSS token stream: trailing bytes remain after the declared blocks.',
    );
  }

  return (
    tokens: List<Token>.unmodifiable(tokens),
    originalLength: originalLength,
  );
}

/// Compresses [data] using LZSS and serialises the result to CMP02 bytes.
Uint8List compress(
  Uint8List data, [
  int windowSize = defaultWindowSize,
  int maxMatch = defaultMaxMatch,
  int minMatch = defaultMinMatch,
]) {
  return serialiseTokens(
    encode(data, windowSize, maxMatch, minMatch),
    data.length,
  );
}

/// Decompresses bytes produced by [compress].
Uint8List decompress(
  Uint8List data, [
  int maxDecompressedSize = defaultMaxDecompressedSize,
]) {
  final decoded = deserialiseTokens(data, maxDecompressedSize);
  return decode(decoded.tokens, decoded.originalLength);
}

void _validateEncoderParameters(int windowSize, int maxMatch, int minMatch) {
  if (windowSize <= 0) {
    throw RangeError.value(
      windowSize,
      'windowSize',
      'Must be a positive number of bytes.',
    );
  }
  if (maxMatch <= 0) {
    throw RangeError.value(
      maxMatch,
      'maxMatch',
      'Must be a positive number of bytes.',
    );
  }
  if (minMatch <= 0) {
    throw RangeError.value(
      minMatch,
      'minMatch',
      'Must be a positive number of bytes.',
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

void _validateLiteral(Literal current) {
  if (current.byte < 0 || current.byte > 255) {
    throw FormatException(
      'Malformed LZSS literal: byte must be in 0..255, got ${current.byte}.',
    );
  }
}

void _validateDecodedMatch(Match current, int outputLength) {
  if (current.offset <= 0) {
    throw FormatException(
      'Malformed LZSS match: offset must be positive, got ${current.offset}.',
    );
  }
  if (current.offset > outputLength) {
    throw FormatException(
      'Malformed LZSS match: offset ${current.offset} exceeds output length $outputLength.',
    );
  }
  if (current.length <= 0 || current.length > 255) {
    throw FormatException(
      'Malformed LZSS match: length must be in 1..255, got ${current.length}.',
    );
  }
}

void _validateSerialisedMatch(Match current) {
  if (current.offset <= 0 || current.offset > 65535) {
    throw FormatException(
      'Malformed LZSS match: offset must be in 1..65535, got ${current.offset}.',
    );
  }
  if (current.length <= 0 || current.length > 255) {
    throw FormatException(
      'Malformed LZSS match: length must be in 1..255, got ${current.length}.',
    );
  }
}
