/// The Caesar cipher — encrypt, decrypt, and break a classic shift cipher.
///
/// The Caesar cipher is the oldest and simplest substitution cipher: each
/// letter of the alphabet is shifted forward by a fixed number of positions.
/// Julius Caesar reportedly used a shift of 3 for his private correspondence.
///
/// ## How the shift works
///
/// We number letters A=0, B=1, …, Z=25. Encryption with shift `s` maps each
/// letter at position `p` to position `(p + s) mod 26`. Decryption maps it to
/// `(p − s) mod 26`, which is the same as `(p + 26 − s) mod 26`.
///
/// ### Truth table for shift = 3
///
/// ```
/// Input  | Position | + Shift | mod 26 | Output
/// -------|----------|---------|--------|-------
///   A    |    0     |    3    |    3   |   D
///   B    |    1     |    4    |    4   |   E
///   H    |    7     |   10    |   10   |   K
///   X    |   23     |   26    |    0   |   A
///   Y    |   24     |   27    |    1   |   B
///   Z    |   25     |   28    |    2   |   C
/// ```
///
/// ### Non-alphabetic characters
///
/// Digits, spaces, punctuation, and other non-letter characters pass through
/// unchanged. This matches the historical usage of the cipher, which was only
/// applied to letters. Case is preserved.
///
/// ### Negative and large shifts
///
/// A negative shift moves letters backwards through the alphabet (shift = −1
/// maps B to A and A to Z). We normalise every shift into the range 0..25
/// using modular arithmetic, so shift −1 and shift 25 produce the same result,
/// and shift 26 is the identity.
library caesar_cipher;

// ===========================================================================
// Core transformation: encrypt / decrypt / rot13
// ===========================================================================

/// Encrypt [text] using the Caesar cipher with the given [shift].
///
/// Each ASCII letter is shifted forward through the alphabet by [shift]
/// positions. Non-alphabetic characters are left unchanged and case is
/// preserved.
///
/// ```
/// encrypt("HELLO", 3)         → "KHOOR"
/// encrypt("hello", 3)         → "khoor"
/// encrypt("Hello, World!", 3) → "Khoor, Zruog!"
/// encrypt("abc", 0)           → "abc"   (identity)
/// encrypt("ABC", 26)          → "ABC"   (full rotation)
/// encrypt("ABC", -1)          → "ZAB"   (negative shift)
/// ```
String encrypt(String text, int shift) {
  // -----------------------------------------------------------------------
  // Step 1: Normalise the shift into the range 0..25.
  //
  // Dart's `%` operator, unlike C or Rust, always returns a non-negative
  // result for a positive divisor — `(-1) % 26 == 25`. So a single `%` is
  // already enough, but we keep the `+ 26` then `% 26` form to make the
  // intent obvious and to mirror the reference implementations exactly.
  //
  //   shift = -1  →  ((-1 % 26) + 26) % 26 = (25 + 26) % 26 = 25
  //   shift =  3  →  (( 3 % 26) + 26) % 26 = (3  + 26) % 26 =  3
  //   shift = 29  →  ((29 % 26) + 26) % 26 = (3  + 26) % 26 =  3
  // -----------------------------------------------------------------------
  final normalisedShift = ((shift % 26) + 26) % 26;

  // Step 2: Transform each UTF-16 code unit. We operate on code units rather
  // than runes because every ASCII letter is a single code unit, and any
  // multi-unit character (emoji, accented letters) is non-ASCII and therefore
  // passes through unchanged anyway.
  final buffer = StringBuffer();
  for (final unit in text.codeUnits) {
    buffer.writeCharCode(_shiftCodeUnit(unit, normalisedShift));
  }
  return buffer.toString();
}

/// Decrypt [text] that was encrypted with the Caesar cipher using [shift].
///
/// Decryption is the inverse of encryption: we shift each letter *backwards*
/// by [shift] positions. Because [encrypt] already normalises negative
/// shifts, decryption is simply encryption with the negated shift.
///
/// For any text `t` and shift `s`: `decrypt(encrypt(t, s), s) == t`.
///
/// ```
/// decrypt("KHOOR", 3) → "HELLO"
/// ```
String decrypt(String text, int shift) => encrypt(text, -shift);

/// Apply ROT13 — a special Caesar cipher with shift 13.
///
/// ROT13 is its own inverse because 13 + 13 = 26, a full rotation:
/// `rot13(rot13(text)) == text`. It was historically popular on Usenet for
/// hiding spoilers and punchlines. It provides no real security.
///
/// ```
/// rot13("Hello") → "Uryyb"
/// rot13("Uryyb") → "Hello"   (self-inverse)
/// rot13("123!")  → "123!"    (non-alpha unchanged)
/// ```
String rot13(String text) => encrypt(text, 13);

/// Shift a single UTF-16 code [unit] by [normalisedShift] positions (0..25).
///
/// Handles uppercase letters, lowercase letters, and everything else
/// separately.
///
/// ```
///  unit = 'H' (72), normalisedShift = 3
///  base = 'A' = 65
///  position = 72 - 65 = 7
///  newPosition = (7 + 3) % 26 = 10
///  result = 65 + 10 = 75 = 'K'
/// ```
int _shiftCodeUnit(int unit, int normalisedShift) {
  const upperA = 65; // 'A'
  const upperZ = 90; // 'Z'
  const lowerA = 97; // 'a'
  const lowerZ = 122; // 'z'

  if (unit >= upperA && unit <= upperZ) {
    return upperA + (unit - upperA + normalisedShift) % 26;
  }
  if (unit >= lowerA && unit <= lowerZ) {
    return lowerA + (unit - lowerA + normalisedShift) % 26;
  }
  // Non-alphabetic: digits, spaces, punctuation, non-ASCII — pass through.
  return unit;
}

// ===========================================================================
// Analysis: breaking the cipher
// ===========================================================================

/// Expected frequency of each letter in English text, as a fraction (not a
/// percentage). Index 0 is A, index 25 is Z. E is by far the most common at
/// ~12.7%; Z is the rarest at ~0.07%. These values come from large-corpus
/// analysis and are widely cited in cryptography literature.
const List<double> englishFrequencies = [
  0.08167, // A
  0.01492, // B
  0.02782, // C
  0.04253, // D
  0.12702, // E
  0.02228, // F
  0.02015, // G
  0.06094, // H
  0.06966, // I
  0.00153, // J
  0.00772, // K
  0.04025, // L
  0.02406, // M
  0.06749, // N
  0.07507, // O
  0.01929, // P
  0.00095, // Q
  0.05987, // R
  0.06327, // S
  0.09056, // T
  0.02758, // U
  0.00978, // V
  0.02360, // W
  0.00150, // X
  0.01974, // Y
  0.00074, // Z
];

/// One candidate result from a brute-force attack: the [shift] that was tried
/// and the resulting [plaintext].
class BruteForceResult {
  /// The shift value that was applied to decrypt.
  final int shift;

  /// The plaintext produced by decrypting with this shift.
  final String plaintext;

  const BruteForceResult(this.shift, this.plaintext);

  @override
  bool operator ==(Object other) =>
      other is BruteForceResult &&
      other.shift == shift &&
      other.plaintext == plaintext;

  @override
  int get hashCode => Object.hash(shift, plaintext);

  @override
  String toString() => 'BruteForceResult(shift: $shift, plaintext: "$plaintext")';
}

/// Try all 25 non-trivial shifts and return the candidate plaintexts.
///
/// Shift 0 is excluded because it is the identity. Results are returned in
/// order from shift 1 to shift 25, so `bruteForce(ct)[i]` holds shift `i + 1`.
/// With only 25 candidates a human analyst can quickly scan the list; for
/// automated detection see [frequencyAnalysis].
///
/// ```
/// final results = bruteForce("KHOOR");
/// results[2].shift     → 3
/// results[2].plaintext → "HELLO"
/// results.length       → 25
/// ```
List<BruteForceResult> bruteForce(String ciphertext) => [
      for (var shift = 1; shift <= 25; shift++)
        BruteForceResult(shift, decrypt(ciphertext, shift)),
    ];

/// The outcome of a frequency-analysis attack: the most likely [shift] and the
/// [plaintext] it produces.
typedef FrequencyAnalysisResult = ({int shift, String plaintext});

/// Use chi-squared frequency analysis to find the most likely shift.
///
/// Tries all 25 non-trivial shifts, scores each candidate plaintext against
/// the known English letter-frequency distribution, and returns the one with
/// the lowest chi-squared value (a *lower* value means a closer fit):
///
/// ```
/// chi² = Σ (observedᵢ − expectedᵢ)² / expectedᵢ
/// ```
///
/// Works best on longer texts (50+ characters); short texts may lack
/// statistical signal. Assumes English plaintext. If the ciphertext contains
/// no alphabetic characters there is no signal, so it falls back to shift 1.
///
/// ```
/// final r = frequencyAnalysis("WKH TXLFN EURZQ IRA MXPSV RYHU WKH ODCB GRJ");
/// r.shift     → 3
/// r.plaintext → "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
/// ```
FrequencyAnalysisResult frequencyAnalysis(String ciphertext) {
  // Start with shift 1 as the default so that even when every candidate ties
  // (e.g. no alphabetic characters, all scoring `double.infinity`), we return
  // a valid shift and its correctly "decrypted" text.
  final firstCandidate = decrypt(ciphertext, 1);
  var bestShift = 1;
  var bestScore = _chiSquared(firstCandidate);
  var bestPlaintext = firstCandidate;

  for (var shift = 2; shift <= 25; shift++) {
    final candidate = decrypt(ciphertext, shift);
    final score = _chiSquared(candidate);
    if (score < bestScore) {
      bestScore = score;
      bestShift = shift;
      bestPlaintext = candidate;
    }
  }

  return (shift: bestShift, plaintext: bestPlaintext);
}

/// Count occurrences of each letter A..Z in [text] (case-insensitive).
/// Returns 26 counts where index 0 is A and index 25 is Z.
List<int> _letterCounts(String text) {
  final counts = List<int>.filled(26, 0);
  const upperA = 65, upperZ = 90, lowerA = 97, lowerZ = 122;
  for (final unit in text.codeUnits) {
    if (unit >= upperA && unit <= upperZ) {
      counts[unit - upperA]++;
    } else if (unit >= lowerA && unit <= lowerZ) {
      counts[unit - lowerA]++;
    }
  }
  return counts;
}

/// Compute the chi-squared statistic comparing the letter distribution of
/// [text] against expected English frequencies. A lower value is a closer fit.
/// If the text has no letters, returns [double.infinity] so this candidate is
/// never chosen.
double _chiSquared(String text) {
  final counts = _letterCounts(text);
  final total = counts.fold<int>(0, (sum, c) => sum + c);
  if (total == 0) return double.infinity;

  final totalF = total.toDouble();
  var chi = 0.0;
  for (var i = 0; i < 26; i++) {
    final expected = totalF * englishFrequencies[i];
    if (expected < 1e-10) continue; // guard; all English freqs are non-zero
    final diff = counts[i] - expected;
    chi += diff * diff / expected;
  }
  return chi;
}
