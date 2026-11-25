/// Errors thrown by Brotli compression and decompression.
library;

/// Base type for Brotli failures. Wire-format messages mirror the
/// reference texts exactly; the two compress-time guards are
/// lane-defined (the reference silently corrupts there — see the
/// package docs).
sealed class BrotliError implements Exception {
  const BrotliError();
}

/// Input shorter than the 10-byte header.
final class BrotliDataTooShort extends BrotliError {
  final int length;
  const BrotliDataTooShort(this.length);

  @override
  bool operator ==(Object other) =>
      other is BrotliDataTooShort && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'brotli: data too short ($length bytes)';
}

/// ICC table runs past the input end.
final class BrotliIccTableTruncated extends BrotliError {
  const BrotliIccTableTruncated();

  @override
  bool operator ==(Object other) => other is BrotliIccTableTruncated;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'brotli: ICC table truncated';
}

/// Distance table runs past the input end.
final class BrotliDistTableTruncated extends BrotliError {
  const BrotliDistTableTruncated();

  @override
  bool operator ==(Object other) => other is BrotliDistTableTruncated;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'brotli: dist table truncated';
}

/// Literal table [context] runs past the input end.
final class BrotliLitTableTruncated extends BrotliError {
  final int context;
  const BrotliLitTableTruncated(this.context);

  @override
  bool operator ==(Object other) =>
      other is BrotliLitTableTruncated && context == other.context;
  @override
  int get hashCode => context.hashCode;
  @override
  String toString() => 'brotli: lit table $context truncated';
}

/// Extra-bits read runs past the bit stream end.
final class BrotliBitStreamExhausted extends BrotliError {
  const BrotliBitStreamExhausted();

  @override
  bool operator ==(Object other) => other is BrotliBitStreamExhausted;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'brotli: bit stream exhausted';
}

/// Huffman symbol read runs past the bit stream end.
final class BrotliHuffmanExhausted extends BrotliError {
  const BrotliHuffmanExhausted();

  @override
  bool operator ==(Object other) => other is BrotliHuffmanExhausted;
  @override
  int get hashCode => 3;
  @override
  String toString() => 'brotli: bit stream exhausted (huffman)';
}

/// No table entry matched within 32 prefix bits.
final class BrotliHuffmanOverrun extends BrotliError {
  const BrotliHuffmanOverrun();

  @override
  bool operator ==(Object other) => other is BrotliHuffmanOverrun;
  @override
  int get hashCode => 4;
  @override
  String toString() => 'brotli: Huffman decode overrun';
}

/// Back-reference reaches before the output start.
final class BrotliInvalidCopyDistance extends BrotliError {
  final int distance;
  final int outputLength;
  const BrotliInvalidCopyDistance(this.distance, this.outputLength);

  @override
  bool operator ==(Object other) =>
      other is BrotliInvalidCopyDistance &&
      distance == other.distance &&
      outputLength == other.outputLength;
  @override
  int get hashCode => Object.hash(distance, outputLength);
  @override
  String toString() =>
      'brotli: copy distance $distance > output length $outputLength';
}

/// Output length differs from the header's original length.
final class BrotliLengthMismatch extends BrotliError {
  final int expected;
  final int actual;
  const BrotliLengthMismatch(this.expected, this.actual);

  @override
  bool operator ==(Object other) =>
      other is BrotliLengthMismatch &&
      expected == other.expected &&
      actual == other.actual;
  @override
  int get hashCode => Object.hash(expected, actual);
  @override
  String toString() =>
      'brotli: length mismatch: expected $expected, got $actual';
}

/// A literal context bucket holds all 256 symbols, which the
/// 1-byte table count cannot represent (lane-defined: the reference
/// truncates the count to 0 and emits undecodable output — proven
/// on random 2KB+ inputs).
final class BrotliTooManyLiteralSymbols extends BrotliError {
  final int bucket;
  const BrotliTooManyLiteralSymbols(this.bucket);

  @override
  bool operator ==(Object other) =>
      other is BrotliTooManyLiteralSymbols && bucket == other.bucket;
  @override
  int get hashCode => bucket.hashCode;
  @override
  String toString() =>
      'brotli: literal table $bucket exceeds the 255-symbol wire limit';
}

/// A code table names a symbol outside its valid range (lane-defined:
/// the reference panics indexing fixed tables; ICC < 64, dist ≤ 32,
/// literals < 256).
final class BrotliInvalidTableSymbol extends BrotliError {
  final String table;
  final int symbol;
  const BrotliInvalidTableSymbol(this.table, this.symbol);

  @override
  bool operator ==(Object other) =>
      other is BrotliInvalidTableSymbol &&
      table == other.table &&
      symbol == other.symbol;
  @override
  int get hashCode => Object.hash(table, symbol);
  @override
  String toString() => 'brotli: invalid symbol $symbol in $table table';
}

/// Input exceeds the 4 GiB header length field (lane-defined: the
/// reference truncates silently).
final class BrotliInputTooLarge extends BrotliError {
  final int length;
  const BrotliInputTooLarge(this.length);

  @override
  bool operator ==(Object other) =>
      other is BrotliInputTooLarge && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() =>
      'brotli: input length $length exceeds the 4 GiB wire limit';
}
