/// Representation of an interleaved digit pair in an ITF barcode.
final class EncodedPair {
  final String pair;
  final String barPattern;
  final String spacePattern;
  final String binaryPattern;
  final int sourceIndex;

  const EncodedPair({
    required this.pair,
    required this.barPattern,
    required this.spacePattern,
    required this.binaryPattern,
    required this.sourceIndex,
  });
}
