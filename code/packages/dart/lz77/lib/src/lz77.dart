import 'dart:typed_data';

/// A single LZ77 token represented as `(offset, length, nextChar)`.
///
/// The encoder emits either a literal token `(0, 0, byte)` or a backreference
/// token where `offset` points backwards into the sliding search buffer,
/// `length` says how many bytes to copy, and `nextChar` finishes the phrase.
class Token {
  /// Creates a token with the given LZ77 fields.
  const Token(this.offset, this.length, this.nextChar);

  /// Distance back from the cursor where the match starts.
  final int offset;

  /// Number of bytes covered by the backreference match.
  final int length;

  /// Literal byte appended after the match is copied.
  final int nextChar;

  @override
  bool operator ==(Object other) {
    return other is Token &&
        other.offset == offset &&
        other.length == length &&
        other.nextChar == nextChar;
  }

  @override
  int get hashCode => Object.hash(offset, length, nextChar);

  @override
  String toString() {
    return 'Token(offset: $offset, length: $length, nextChar: $nextChar)';
  }
}

/// Creates a [Token] without needing to call the constructor directly.
Token token(int offset, int length, int nextChar) {
  return Token(offset, length, nextChar);
}

/// Finds the longest match in the sliding search buffer.
///
/// The encoder compares every candidate start in the previous `windowSize`
/// bytes and keeps the longest prefix match against the current cursor.
({int offset, int length}) _findLongestMatch(
  Uint8List data,
  int cursor,
  int windowSize,
  int maxMatch,
) {
  var bestOffset = 0;
  var bestLength = 0;

  final searchStart = cursor - windowSize < 0 ? 0 : cursor - windowSize;
  final lookaheadEnd = _min(cursor + maxMatch, data.length - 1);

  for (var position = searchStart; position < cursor; position++) {
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

/// Encodes [data] into LZ77 tokens.
///
/// The encoder walks left to right. At each cursor position it finds the
/// longest match in the previous `windowSize` bytes. If that match is at least
/// `minMatch` bytes long it becomes a backreference token; otherwise the byte
/// is emitted as a literal token.
List<Token> encode(
  Uint8List data, [
  int windowSize = 4096,
  int maxMatch = 255,
  int minMatch = 3,
]) {
  final tokens = <Token>[];
  var cursor = 0;

  while (cursor < data.length) {
    // The last byte cannot start a backreference because there would be no
    // room left to store the required trailing literal `nextChar`.
    if (cursor == data.length - 1) {
      tokens.add(token(0, 0, data[cursor]));
      cursor += 1;
      continue;
    }

    final match = _findLongestMatch(data, cursor, windowSize, maxMatch);
    if (match.length >= minMatch) {
      final nextChar = data[cursor + match.length];
      tokens.add(token(match.offset, match.length, nextChar));
      cursor += match.length + 1;
    } else {
      tokens.add(token(0, 0, data[cursor]));
      cursor += 1;
    }
  }

  return List<Token>.unmodifiable(tokens);
}

/// Decodes LZ77 [tokens] back into the original bytes.
///
/// Backreferences are copied byte by byte instead of in bulk so overlapping
/// matches like `(offset: 1, length: 5)` expand correctly.
Uint8List decode(List<Token> tokens, [Uint8List? initialBuffer]) {
  final output = (initialBuffer ?? Uint8List(0)).toList(growable: true);

  for (final current in tokens) {
    _validateDecodedToken(current, output.length);

    if (current.length > 0) {
      final start = output.length - current.offset;
      for (var index = 0; index < current.length; index++) {
        output.add(output[start + index]);
      }
    }

    output.add(current.nextChar);
  }

  return Uint8List.fromList(output);
}

/// Serialises a token stream to a fixed-width teaching format.
///
/// Layout:
/// - 4 bytes: token count as big-endian uint32
/// - N x 4 bytes: `(offset: uint16, length: uint8, nextChar: uint8)`
Uint8List serialiseTokens(List<Token> tokens) {
  final bytes = ByteData(4 + tokens.length * 4);
  bytes.setUint32(0, tokens.length, Endian.big);

  for (var index = 0; index < tokens.length; index++) {
    final base = 4 + index * 4;
    final current = tokens[index];
    bytes.setUint16(base, current.offset, Endian.big);
    bytes.setUint8(base + 2, current.length);
    bytes.setUint8(base + 3, current.nextChar);
  }

  return bytes.buffer.asUint8List();
}

/// Deserialises the fixed-width teaching format back into tokens.
List<Token> deserialiseTokens(Uint8List data) {
  if (data.length < 4) {
    return const <Token>[];
  }

  final view = ByteData.view(
    data.buffer,
    data.offsetInBytes,
    data.lengthInBytes,
  );
  final count = view.getUint32(0, Endian.big);
  final tokens = <Token>[];

  for (var index = 0; index < count; index++) {
    final base = 4 + index * 4;
    if (base + 4 > data.length) {
      throw const FormatException(
        'Malformed LZ77 token stream: truncated data.',
      );
    }

    final current = token(
      view.getUint16(base, Endian.big),
      view.getUint8(base + 2),
      view.getUint8(base + 3),
    );
    _validateDeserialisedToken(current);
    tokens.add(current);
  }

  return List<Token>.unmodifiable(tokens);
}

/// Compresses [data] using the one-shot LZ77 API.
Uint8List compress(
  Uint8List data, [
  int windowSize = 4096,
  int maxMatch = 255,
  int minMatch = 3,
]) {
  return serialiseTokens(encode(data, windowSize, maxMatch, minMatch));
}

/// Decompresses data produced by [compress].
Uint8List decompress(Uint8List data) {
  return decode(deserialiseTokens(data));
}

int _min(int left, int right) => left < right ? left : right;

void _validateDecodedToken(Token current, int outputLength) {
  if (current.length < 0) {
    throw FormatException(
      'Malformed LZ77 token: length must be non-negative, got ${current.length}.',
    );
  }
  if (current.nextChar < 0 || current.nextChar > 255) {
    throw FormatException(
      'Malformed LZ77 token: nextChar must be in 0..255, got ${current.nextChar}.',
    );
  }
  if (current.length == 0) {
    if (current.offset != 0) {
      throw FormatException(
        'Malformed LZ77 token: literal tokens must use offset 0, got ${current.offset}.',
      );
    }
    return;
  }
  if (current.offset <= 0) {
    throw FormatException(
      'Malformed LZ77 token: backreferences must use a positive offset, got ${current.offset}.',
    );
  }
  if (current.offset > outputLength) {
    throw FormatException(
      'Malformed LZ77 token: offset ${current.offset} exceeds decoded prefix length $outputLength.',
    );
  }
}

void _validateDeserialisedToken(Token current) {
  if (current.length > 0 && current.offset == 0) {
    throw const FormatException(
      'Malformed LZ77 token stream: backreferences must use a positive offset.',
    );
  }
}
