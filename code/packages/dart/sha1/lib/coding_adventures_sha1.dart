/// SHA-1 cryptographic hash (FIPS 180-4), implemented from scratch in pure Dart.
///
/// Provides a one-shot API ([sum1] / [hexString]) and an incremental
/// [Sha1Digest] for streaming data.
///
/// **Security note:** SHA-1 is broken for collision resistance (SHAttered,
/// 2017) — do not use it for signatures. Legacy/checksum use only.
///
/// ## Usage
///
/// ```dart
/// import 'dart:convert';
/// import 'package:coding_adventures_sha1/coding_adventures_sha1.dart';
///
/// void main() {
///   print(hexString(utf8.encode('abc')));
///   // a9993e364706816aba3e25717850c26c9cd0d89d
///
///   final h = Sha1Digest()
///     ..update(utf8.encode('ab'))
///     ..update(utf8.encode('c'));
///   print(h.hexDigest()); // same digest, computed incrementally
/// }
/// ```
library coding_adventures_sha1;

export 'src/sha1.dart';
