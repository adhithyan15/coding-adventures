import 'dart:typed_data';

/// One LZ78 token represented as `(dictIndex, nextChar)`.
///
/// `dictIndex` points at the longest dictionary prefix that matches the
/// current input. `nextChar` is the following byte, or `0` when a flush token
/// terminates a partial match at end of stream.
class Token {
  /// Creates a token with the given LZ78 fields.
  const Token(this.dictIndex, this.nextChar);

  /// Dictionary entry ID for the longest prefix match, or `0` for a literal.
  final int dictIndex;

  /// Byte following the matched prefix, or the flush sentinel `0`.
  final int nextChar;

  @override
  bool operator ==(Object other) {
    return other is Token &&
        other.dictIndex == dictIndex &&
        other.nextChar == nextChar;
  }

  @override
  int get hashCode => Object.hash(dictIndex, nextChar);

  @override
  String toString() {
    return 'Token(dictIndex: $dictIndex, nextChar: $nextChar)';
  }
}

/// Creates a [Token] without needing to call the constructor directly.
Token token(int dictIndex, int nextChar) => Token(dictIndex, nextChar);

/// Internal trie node used by [TrieCursor].
class _CursorNode {
  _CursorNode(this.dictId);

  final int dictId;
  final Map<int, _CursorNode> children = <int, _CursorNode>{};
}

/// A byte-at-a-time trie cursor for streaming dictionary algorithms.
///
/// The encoder keeps the cursor on the current matched prefix. When [step]
/// fails for a byte, the encoder emits a token, optionally [insert]s the new
/// sequence, and [reset]s to the trie root.
class TrieCursor {
  /// Creates an empty trie cursor positioned at the root.
  TrieCursor()
      : _root = _CursorNode(0),
        _current = _CursorNode(0) {
    _current = _root;
  }

  final _CursorNode _root;
  late _CursorNode _current;

  /// Tries to follow the edge for [byte] from the current position.
  ///
  /// Returns `true` and advances the cursor if the edge exists. Returns
  /// `false` and leaves the cursor unchanged on a miss.
  bool step(int byte) {
    final child = _current.children[byte];
    if (child == null) {
      return false;
    }

    _current = child;
    return true;
  }

  /// Inserts a child edge for [byte] with the given [dictId].
  void insert(int byte, int dictId) {
    _current.children[byte] = _CursorNode(dictId);
  }

  /// Resets the cursor back to the trie root.
  void reset() {
    _current = _root;
  }

  /// Dictionary ID at the current cursor position.
  int get dictId => _current.dictId;

  /// Whether the cursor is currently at the root.
  bool get atRoot => identical(_current, _root);
}

/// Encodes [data] into an LZ78 token stream.
///
/// The encoder walks an explicit trie one byte at a time. On a miss, it emits
/// the current dictionary prefix plus the unmatched byte, records the new
/// sequence in the dictionary if space allows, and resets to the root.
List<Token> encode(Uint8List data, [int maxDictSize = 65536]) {
  final cursor = TrieCursor();
  var nextId = 1;
  final tokens = <Token>[];

  for (final byte in data) {
    if (!cursor.step(byte)) {
      tokens.add(token(cursor.dictId, byte));

      if (nextId < maxDictSize) {
        cursor.insert(byte, nextId);
        nextId += 1;
      }

      cursor.reset();
    }
  }

  if (!cursor.atRoot) {
    tokens.add(token(cursor.dictId, 0));
  }

  return List<Token>.unmodifiable(tokens);
}

/// Decodes an LZ78 token stream back into the original bytes.
///
/// If [originalLength] is provided, the decoded output is truncated to that
/// many bytes so the flush sentinel is not returned to the caller.
Uint8List decode(List<Token> tokens, [int originalLength = -1]) {
  final table = <({int parentId, int byte})>[(parentId: 0, byte: 0)];
  final output = <int>[];

  for (final current in tokens) {
    _validateDecodedToken(current, table.length);

    final sequence = _reconstruct(table, current.dictIndex);
    output.addAll(sequence);
    if (originalLength >= 0 && output.length > originalLength) {
      throw FormatException(
        'Malformed LZ78 token stream: decoded output exceeds declared length $originalLength.',
      );
    }

    if (originalLength < 0 || output.length < originalLength) {
      output.add(current.nextChar);
    } else if (current.nextChar != 0) {
      throw FormatException(
        'Malformed LZ78 token stream: token data exceeds declared length $originalLength.',
      );
    }

    table.add((parentId: current.dictIndex, byte: current.nextChar));
  }

  if (originalLength >= 0 && output.length != originalLength) {
    throw FormatException(
      'Malformed LZ78 token stream: decoded output length ${output.length} does not match declared length $originalLength.',
    );
  }

  return Uint8List.fromList(output);
}

List<int> _reconstruct(List<({int parentId, int byte})> table, int index) {
  if (index == 0) {
    return const <int>[];
  }

  final reversed = <int>[];
  var current = index;
  while (current != 0) {
    final entry = table[current];
    reversed.add(entry.byte);
    current = entry.parentId;
  }

  return reversed.reversed.toList(growable: false);
}

/// Serialises [tokens] to the fixed-width CMP01 wire format.
///
/// Layout:
/// - 4 bytes: original length as big-endian uint32
/// - 4 bytes: token count as big-endian uint32
/// - N x 4 bytes: `(dictIndex: uint16, nextChar: uint8, reserved: uint8)`
Uint8List serialiseTokens(List<Token> tokens, int originalLength) {
  final bytes = ByteData(8 + tokens.length * 4);
  bytes.setUint32(0, originalLength, Endian.big);
  bytes.setUint32(4, tokens.length, Endian.big);

  for (var index = 0; index < tokens.length; index++) {
    final base = 8 + index * 4;
    final current = tokens[index];
    bytes.setUint16(base, current.dictIndex, Endian.big);
    bytes.setUint8(base + 2, current.nextChar);
    bytes.setUint8(base + 3, 0);
  }

  return bytes.buffer.asUint8List();
}

/// Deserialises the CMP01 wire format back into tokens and original length.
({List<Token> tokens, int originalLength}) deserialiseTokens(Uint8List data) {
  if (data.length < 8) {
    throw const FormatException(
      'Malformed LZ78 token stream: header is incomplete.',
    );
  }

  final view = ByteData.view(
    data.buffer,
    data.offsetInBytes,
    data.lengthInBytes,
  );
  final originalLength = view.getUint32(0, Endian.big);
  final tokenCount = view.getUint32(4, Endian.big);
  final expectedLength = 8 + tokenCount * 4;
  if (data.length != expectedLength) {
    throw FormatException(
      'Malformed LZ78 token stream: expected $expectedLength bytes, got ${data.length}.',
    );
  }
  final tokens = <Token>[];

  for (var index = 0; index < tokenCount; index++) {
    final base = 8 + index * 4;
    final current = token(
      view.getUint16(base, Endian.big),
      view.getUint8(base + 2),
    );
    _validateDeserialisedToken(current, originalLength);
    tokens.add(current);
  }

  return (
    tokens: List<Token>.unmodifiable(tokens),
    originalLength: originalLength,
  );
}

/// Compresses [data] using LZ78 and serialises to the CMP01 wire format.
Uint8List compress(Uint8List data, [int maxDictSize = 65536]) {
  return serialiseTokens(encode(data, maxDictSize), data.length);
}

/// Decompresses bytes produced by [compress].
Uint8List decompress(Uint8List data) {
  final decoded = deserialiseTokens(data);
  return decode(decoded.tokens, decoded.originalLength);
}

void _validateDecodedToken(Token current, int dictionaryLength) {
  if (current.dictIndex < 0) {
    throw FormatException(
      'Malformed LZ78 token: dictIndex must be non-negative, got ${current.dictIndex}.',
    );
  }
  if (current.dictIndex >= dictionaryLength) {
    throw FormatException(
      'Malformed LZ78 token: dictIndex ${current.dictIndex} exceeds dictionary size ${dictionaryLength - 1}.',
    );
  }
  if (current.nextChar < 0 || current.nextChar > 255) {
    throw FormatException(
      'Malformed LZ78 token: nextChar must be in 0..255, got ${current.nextChar}.',
    );
  }
}

void _validateDeserialisedToken(Token current, int originalLength) {
  if (originalLength == 0 && current.dictIndex != 0) {
    throw const FormatException(
      'Malformed LZ78 token stream: non-empty dictionary reference for empty output.',
    );
  }
}
