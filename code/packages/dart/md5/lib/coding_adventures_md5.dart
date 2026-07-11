/// MD5 message-digest algorithm (RFC 1321), implemented from scratch in pure
/// Dart.
///
/// Provides a one-shot API ([sumMd5] / [hexString]) and an incremental
/// [Md5Digest] for streaming data.
///
/// **Security note:** MD5 is cryptographically broken (practical collisions
/// exist) — do not use it for signatures or passwords. It remains a useful fast
/// checksum for non-adversarial integrity checks.
///
/// ## Usage
///
/// ```dart
/// import 'dart:convert';
/// import 'package:coding_adventures_md5/coding_adventures_md5.dart';
///
/// void main() {
///   print(hexString(utf8.encode('abc'))); // 900150983cd24fb0d6963f7d28e17f72
///
///   final h = Md5Digest()
///     ..update(utf8.encode('ab'))
///     ..update(utf8.encode('c'));
///   print(h.hexDigest()); // same digest, computed incrementally
/// }
/// ```
library coding_adventures_md5;

export 'src/md5.dart';
