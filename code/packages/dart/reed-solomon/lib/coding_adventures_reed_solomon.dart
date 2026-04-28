/// Reed-Solomon error-correcting codes over GF(256).
///
/// This library provides systematic RS encoding and decoding:
///
///   - [rsEncode] — encode a message with `nCheck` redundancy bytes
///   - [rsDecode] — recover the message, correcting up to `t = nCheck/2` errors
///   - [rsBuildGenerator] — build the RS generator polynomial
///   - [rsSyndromes] — compute syndrome values for a received codeword
///   - [rsErrorLocator] — run Berlekamp-Massey on syndromes
///
/// ## Usage
///
/// ```dart
/// import 'dart:typed_data';
/// import 'package:coding_adventures_reed_solomon/coding_adventures_reed_solomon.dart';
///
/// void main() {
///   final message = Uint8List.fromList([1, 2, 3, 4, 5]);
///   const nCheck = 8; // t = 4 errors correctable
///
///   final codeword = rsEncode(message, nCheck);
///
///   // Introduce 3 errors (≤ t = 4, so recoverable)
///   codeword[0] ^= 0xFF;
///   codeword[2] ^= 0xAA;
///   codeword[4] ^= 0x55;
///
///   final recovered = rsDecode(codeword, nCheck);
///   print(recovered);  // → [1, 2, 3, 4, 5]
/// }
/// ```
library coding_adventures_reed_solomon;

export 'src/reed_solomon.dart';
