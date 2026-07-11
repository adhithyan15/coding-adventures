/// The Caesar cipher — **native-through-Rust** Dart bindings.
///
/// This package exposes the *same* API as the pure-Dart
/// `coding_adventures_caesar_cipher` package, but every call is executed by the
/// Rust `caesar-cipher` crate through a C ABI (`dart:ffi`). The pure port is
/// the readable reference implementation; this one shares a single Rust source
/// of truth with the Rust, Python, and other native bindings.
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_caesar_cipher_native/coding_adventures_caesar_cipher_native.dart';
///
/// void main() {
///   final ct = encrypt('Attack at dawn!', 3); // → 'Dwwdfn dw gdzq!'  (in Rust)
///   print(decrypt(ct, 3));                     // → 'Attack at dawn!'
///   print(rot13('Hello'));                     // → 'Uryyb'
///
///   final r = frequencyAnalysis(encrypt('THE QUICK BROWN FOX', 7));
///   print('${r.shift}: ${r.plaintext}');       // → '7: THE QUICK BROWN FOX'
/// }
/// ```
///
/// The shared library is located via the `CAESAR_CIPHER_NATIVE_PATH`
/// environment variable (an absolute path) or the platform default name on the
/// loader search path. `tools/run-tests.sh` builds the cdylib with cargo and
/// sets that variable before running the tests.
library coding_adventures_caesar_cipher_native;

import 'src/ffi.dart' as ffi;

/// Encrypt [text] with the Caesar cipher using [shift] (executed in Rust).
///
/// Letters shift forward by [shift]; case is preserved; non-alphabetic and
/// non-ASCII characters pass through unchanged. Negative and out-of-range
/// shifts are normalised into `0..25`.
String encrypt(String text, int shift) => ffi.nativeEncrypt(text, shift);

/// Decrypt [text] (inverse of [encrypt]) using [shift] (executed in Rust).
String decrypt(String text, int shift) => ffi.nativeDecrypt(text, shift);

/// Apply ROT13 to [text] — the shift-13 self-inverse special case.
String rot13(String text) => ffi.nativeRot13(text);

/// The outcome of a frequency-analysis attack: the most likely [shift] and the
/// [plaintext] it produces.
typedef FrequencyAnalysisResult = ({int shift, String plaintext});

/// Recover the most likely [shift] for [ciphertext] via chi-squared frequency
/// analysis (executed in Rust), returning it with the decrypted plaintext.
/// Falls back to shift 1 when the ciphertext has no alphabetic characters.
FrequencyAnalysisResult frequencyAnalysis(String ciphertext) {
  final (shift, plaintext) = ffi.nativeFrequencyAnalysis(ciphertext);
  return (shift: shift, plaintext: plaintext);
}

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
  String toString() =>
      'BruteForceResult(shift: $shift, plaintext: "$plaintext")';
}

/// Try all 25 non-trivial shifts and return the candidate plaintexts, in order
/// from shift 1 to shift 25.
///
/// Each candidate is produced by a native [decrypt] call (executed in Rust), so
/// the whole result is correct for *any* input — including ciphertext that
/// contains tabs or newlines, which a single serialised C string could not
/// represent unambiguously.
List<BruteForceResult> bruteForce(String ciphertext) => [
      for (var shift = 1; shift <= 25; shift++)
        BruteForceResult(shift, decrypt(ciphertext, shift)),
    ];
