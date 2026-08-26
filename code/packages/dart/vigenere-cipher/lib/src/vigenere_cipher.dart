import 'dart:math' as math;

/// Highest key length accepted by the classical-analysis helpers.
const int maxAnalysisKeyLength = 40;

/// Expected English frequencies for ASCII letters A through Z.
const List<double> englishFrequencies = [
  0.08167,
  0.01492,
  0.02782,
  0.04253,
  0.12702,
  0.02228,
  0.02015,
  0.06094,
  0.06966,
  0.00153,
  0.00772,
  0.04025,
  0.02406,
  0.06749,
  0.07507,
  0.01929,
  0.00095,
  0.05987,
  0.06327,
  0.09056,
  0.02758,
  0.00978,
  0.02360,
  0.00150,
  0.01974,
  0.00074,
];

/// Result returned by [breakCipher].
final class BreakResult {
  const BreakResult({required this.key, required this.plaintext});

  final String key;
  final String plaintext;

  @override
  bool operator ==(Object other) =>
      other is BreakResult && other.key == key && other.plaintext == plaintext;

  @override
  int get hashCode => Object.hash(key, plaintext);

  @override
  String toString() => 'BreakResult(key: <redacted>, plaintext: <redacted>)';
}

/// Encrypts ASCII letters in [plaintext] with a repeating ASCII [key].
///
/// Case is preserved. Non-ASCII letters and all other scalar values pass
/// through without consuming a key position.
String encrypt(String plaintext, String key) =>
    _applyCipher(plaintext, _keyShifts(key), 1);

/// Reverses Vigenere encryption with [key].
String decrypt(String ciphertext, String key) =>
    _applyCipher(ciphertext, _keyShifts(key), -1);

/// Estimates the Vigenere key length using average index of coincidence.
///
/// Candidates are considered from 2 through [maxLength]. A result of 1 is the
/// deterministic fallback when the ciphertext has insufficient ASCII signal.
int findKeyLength(String ciphertext, {int maxLength = 20}) {
  if (maxLength > maxAnalysisKeyLength) {
    throw RangeError.range(
      maxLength,
      0,
      maxAnalysisKeyLength,
      'maxLength',
    );
  }

  final letters = _extractAsciiUpper(ciphertext);
  final limit = math.min(maxLength, letters.length ~/ 2);
  if (letters.length < 2 || limit < 2) return 1;

  final scores = <(int, double)>[
    for (var keyLength = 2; keyLength <= limit; keyLength++)
      (keyLength, _averageIndexOfCoincidence(letters, keyLength)),
  ];
  final bestScore = scores.map((candidate) => candidate.$2).reduce(math.max);
  if (bestScore <= 0) return 1;

  // Multiples of the real key also have a strong IC. Choosing the first
  // candidate within ten percent of the best favors the shortest period.
  // The neutral fixture owner records this provisional historical heuristic
  // so every established lane can later converge in one reviewed change.
  final threshold = bestScore * 0.90;
  return scores.firstWhere((candidate) => candidate.$2 >= threshold).$1;
}

/// Recovers an uppercase key of [keyLength] with chi-squared analysis.
String findKey(String ciphertext, int keyLength) {
  if (keyLength <= 0) return '';
  if (keyLength > maxAnalysisKeyLength) {
    throw RangeError.range(
      keyLength,
      1,
      maxAnalysisKeyLength,
      'keyLength',
    );
  }

  final letters = _extractAsciiUpper(ciphertext);
  return String.fromCharCodes([
    for (var position = 0; position < keyLength; position++)
      0x41 + _bestShift(_positionGroup(letters, keyLength, position)),
  ]);
}

/// Estimates the key, decrypts [ciphertext], and returns both values.
BreakResult breakCipher(String ciphertext) {
  final key = findKey(ciphertext, findKeyLength(ciphertext));
  return BreakResult(key: key, plaintext: decrypt(ciphertext, key));
}

List<int> _keyShifts(String key) {
  if (key.isEmpty) {
    throw ArgumentError('key must not be empty');
  }

  final shifts = <int>[];
  for (final codePoint in key.runes) {
    if (!_isAsciiLetter(codePoint)) {
      throw ArgumentError('key must contain only ASCII letters');
    }
    shifts.add(_toAsciiUpper(codePoint) - 0x41);
  }
  return shifts;
}

String _applyCipher(String text, List<int> shifts, int direction) {
  var keyIndex = 0;
  final output = <int>[];
  for (final codePoint in text.runes) {
    if (_isAsciiLetter(codePoint)) {
      final base = codePoint >= 0x61 ? 0x61 : 0x41;
      final offset = codePoint - base;
      final shift = direction * shifts[keyIndex % shifts.length];
      output.add(base + (offset + shift) % 26);
      keyIndex++;
    } else {
      output.add(codePoint);
    }
  }
  return String.fromCharCodes(output);
}

List<int> _extractAsciiUpper(String text) => [
      for (final codePoint in text.runes)
        if (_isAsciiLetter(codePoint)) _toAsciiUpper(codePoint),
    ];

bool _isAsciiLetter(int codePoint) =>
    (codePoint >= 0x41 && codePoint <= 0x5a) ||
    (codePoint >= 0x61 && codePoint <= 0x7a);

int _toAsciiUpper(int codePoint) =>
    codePoint >= 0x61 && codePoint <= 0x7a ? codePoint - 0x20 : codePoint;

double _averageIndexOfCoincidence(List<int> letters, int keyLength) {
  var total = 0.0;
  var groups = 0;
  for (var position = 0; position < keyLength; position++) {
    final group = _positionGroup(letters, keyLength, position);
    if (group.length > 1) {
      total += _indexOfCoincidence(group);
      groups++;
    }
  }
  return groups == 0 ? 0 : total / groups;
}

List<int> _positionGroup(List<int> letters, int keyLength, int position) => [
      for (var index = position; index < letters.length; index += keyLength)
        letters[index],
    ];

double _indexOfCoincidence(List<int> letters) {
  final counts = _letterCounts(letters);
  final numerator = counts.fold<int>(
    0,
    (sum, count) => sum + count * (count - 1),
  );
  return numerator / (letters.length * (letters.length - 1));
}

int _bestShift(List<int> group) {
  if (group.isEmpty) return 0;

  var bestShift = 0;
  var bestScore = double.infinity;
  for (var shift = 0; shift < 26; shift++) {
    final decrypted = [
      for (final letter in group) 0x41 + (letter - 0x41 - shift) % 26,
    ];
    final score = _chiSquared(_letterCounts(decrypted));
    if (score < bestScore) {
      bestScore = score;
      bestShift = shift;
    }
  }
  return bestShift;
}

List<int> _letterCounts(List<int> letters) {
  final counts = List<int>.filled(26, 0);
  for (final letter in letters) {
    counts[letter - 0x41]++;
  }
  return counts;
}

double _chiSquared(List<int> counts) {
  final total = counts.fold<int>(0, (sum, count) => sum + count);
  var score = 0.0;
  for (var index = 0; index < 26; index++) {
    final expected = total * englishFrequencies[index];
    final difference = counts[index] - expected;
    score += difference * difference / expected;
  }
  return score;
}
