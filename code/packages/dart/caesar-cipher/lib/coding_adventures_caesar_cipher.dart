/// The Caesar cipher — a classic shift substitution cipher.
///
/// Each letter of the alphabet is shifted forward by a fixed number of
/// positions; non-letters pass through unchanged and case is preserved. This
/// library also includes the two classic attacks that break the cipher:
/// brute force (only 25 keys) and chi-squared frequency analysis.
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_caesar_cipher/coding_adventures_caesar_cipher.dart';
///
/// void main() {
///   final ct = encrypt('Attack at dawn!', 3);
///   print(ct);                    // → 'Dwwdfn dw gdzq!'
///   print(decrypt(ct, 3));        // → 'Attack at dawn!'
///   print(rot13('Hello'));        // → 'Uryyb'
///
///   // Break it without knowing the key:
///   final r = frequencyAnalysis(encrypt('THE QUICK BROWN FOX', 7));
///   print(r.shift);               // → 7
///   print(r.plaintext);           // → 'THE QUICK BROWN FOX'
/// }
/// ```
library coding_adventures_caesar_cipher;

export 'src/caesar_cipher.dart';
