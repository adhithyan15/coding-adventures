/// Largest ciphertext accepted by [bruteForce].
///
/// The candidate list contains O(n²) total text, so a fixed public bound keeps
/// accidental or hostile calls from turning this educational API into an
/// unbounded allocation primitive.
const int maxBruteForceTextLength = 4096;

/// One candidate produced by [bruteForce].
final class ScytaleCandidate {
  const ScytaleCandidate({required this.key, required this.text});

  final int key;
  final String text;

  @override
  bool operator ==(Object other) =>
      other is ScytaleCandidate && other.key == key && other.text == text;

  @override
  int get hashCode => Object.hash(key, text);

  @override
  String toString() => 'ScytaleCandidate(key: $key, text: <redacted>)';
}

/// Encrypts [text] with a Scytale grid containing [key] columns.
///
/// The portable CR02 contract counts Unicode scalar values. The final row is
/// padded with literal U+0020 spaces and the grid is then read column-first.
String encrypt(String text, int key) {
  if (text.isEmpty) return '';

  final scalars = text.runes.toList(growable: true);
  _validateKey(key, scalars.length);

  final rows = _ceilingDivide(scalars.length, key);
  scalars.addAll(List.filled(rows * key - scalars.length, 0x20));

  final ciphertext = <int>[];
  for (var column = 0; column < key; column++) {
    for (var row = 0; row < rows; row++) {
      ciphertext.add(scalars[row * key + column]);
    }
  }
  return String.fromCharCodes(ciphertext);
}

/// Decrypts [text] with a Scytale grid containing [key] columns.
///
/// Uneven column lengths are supported because brute-force candidates need to
/// remain defined when the candidate key does not divide the ciphertext.
/// Every trailing U+0020 is removed; other trailing whitespace remains data.
String decrypt(String text, int key) {
  if (text.isEmpty) return '';

  final scalars = text.runes.toList(growable: false);
  _validateKey(key, scalars.length);

  final rows = _ceilingDivide(scalars.length, key);
  final remainder = scalars.length % key;
  final columnLengths = List<int>.generate(
    key,
    (column) => remainder == 0 || column < remainder ? rows : rows - 1,
    growable: false,
  );

  final columnStarts = List<int>.filled(key, 0, growable: false);
  for (var column = 1; column < key; column++) {
    columnStarts[column] = columnStarts[column - 1] + columnLengths[column - 1];
  }

  final plaintext = <int>[];
  for (var row = 0; row < rows; row++) {
    for (var column = 0; column < key; column++) {
      if (row < columnLengths[column]) {
        plaintext.add(scalars[columnStarts[column] + row]);
      }
    }
  }
  while (plaintext.isNotEmpty && plaintext.last == 0x20) {
    plaintext.removeLast();
  }
  return String.fromCharCodes(plaintext);
}

/// Tries keys 2 through half the Unicode-scalar length of [text].
List<ScytaleCandidate> bruteForce(String text) {
  final scalarLength = text.runes.length;
  if (scalarLength > maxBruteForceTextLength) {
    throw RangeError.range(
      scalarLength,
      0,
      maxBruteForceTextLength,
      'text scalar length',
    );
  }
  if (scalarLength < 4) return const [];

  return [
    for (var key = 2; key <= scalarLength ~/ 2; key++)
      ScytaleCandidate(key: key, text: decrypt(text, key)),
  ];
}

void _validateKey(int key, int scalarLength) {
  if (key < 2) {
    throw ArgumentError('key must be at least 2');
  }
  if (key > scalarLength) {
    throw ArgumentError('key must not exceed the text scalar length');
  }
}

int _ceilingDivide(int dividend, int divisor) =>
    dividend ~/ divisor + (dividend % divisor == 0 ? 0 : 1);
