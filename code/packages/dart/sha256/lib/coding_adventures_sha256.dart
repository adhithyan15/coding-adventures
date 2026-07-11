/// SHA-256 cryptographic hash (FIPS 180-4), implemented from scratch in pure
/// Dart.
///
/// SHA-256 maps any byte sequence to a fixed 32-byte digest. This library
/// offers a one-shot API ([sha256] / [sha256Hex]) and an incremental
/// [Sha256Hasher] for streaming data that does not fit in memory at once.
///
/// ## Usage
///
/// ```dart
/// import 'dart:convert';
/// import 'package:coding_adventures_sha256/coding_adventures_sha256.dart';
///
/// void main() {
///   print(sha256Hex(utf8.encode('abc')));
///   // ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
///
///   final h = Sha256Hasher()
///     ..update(utf8.encode('ab'))
///     ..update(utf8.encode('c'));
///   print(h.hexDigest()); // same digest, computed incrementally
/// }
/// ```
library coding_adventures_sha256;

export 'src/sha256.dart';
