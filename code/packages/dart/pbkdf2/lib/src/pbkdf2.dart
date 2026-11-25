/// PBKDF2 password hashing (RFC 8018 §5.2).
///
/// PBKDF2 stretches a password into a key by iterating a PRF
/// (HMAC-SHA-1/256/512 here) `iterations` times per output block:
///
/// ```text
/// DK  = T_1 || T_2 || ...            (first keyLength bytes)
/// T_i = U_1 XOR U_2 XOR ... XOR U_c
/// U_1 = PRF(password, salt || INT_32_BE(i))
/// U_j = PRF(password, U_{j-1})
/// ```
///
/// The iteration count is the tunable cost: every brute-force guess
/// costs the same `iterations` PRF calls.
library;

import 'dart:typed_data';

import 'package:coding_adventures_hmac/hmac.dart';
import 'package:coding_adventures_sha1/coding_adventures_sha1.dart' as sha1lane;
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart'
    as sha256lane;
import 'package:coding_adventures_sha512/coding_adventures_sha512.dart'
    as sha512lane;

import 'errors.dart';

/// Practical output cap: 2^20 bytes (1 MiB). Far below the RFC's
/// (2^32−1)×hLen hard cap; prevents memory-exhaustion DoS and keeps
/// block counting overflow-free.
const int _maxKeyLength = 1 << 20;

/// The PBKDF2 core loop over an infallible PRF closure returning
/// exactly [hLen] bytes.
Uint8List _pbkdf2Core(
  Uint8List Function(List<int> key, List<int> message) prf,
  int hLen,
  List<int> password,
  List<int> salt,
  int iterations,
  int keyLength,
  bool allowEmptyPassword,
) {
  if (password.isEmpty && !allowEmptyPassword) {
    throw const Pbkdf2EmptyPassword();
  }
  if (iterations <= 0) {
    throw const Pbkdf2InvalidIterations();
  }
  if (keyLength <= 0) {
    throw const Pbkdf2InvalidKeyLength();
  }
  if (keyLength > _maxKeyLength) {
    throw const Pbkdf2KeyLengthTooLarge();
  }

  final numBlocks = (keyLength + hLen - 1) ~/ hLen;
  final dk = <int>[];
  for (var i = 1; i <= numBlocks; i++) {
    // Seed = salt || INT_32_BE(i): the big-endian block counter
    // makes each block's first U value unique.
    final seed = [
      ...salt,
      (i >> 24) & 0xff,
      (i >> 16) & 0xff,
      (i >> 8) & 0xff,
      i & 0xff,
    ];
    final u = prf(password, seed);
    final t = Uint8List.fromList(u);
    var prev = u;
    for (var j = 1; j < iterations; j++) {
      final next = prf(password, prev);
      for (var k = 0; k < t.length; k++) {
        t[k] ^= next[k];
      }
      prev = next;
    }
    dk.addAll(t);
  }
  return Uint8List.fromList(dk.sublist(0, keyLength));
}

/// Runs [hmacFn] (a named in-lane variant) as a PBKDF2 PRF, mapping
/// any HMAC failure to [Pbkdf2PrfError].
///
/// The mapping is defensive only: the core rejects empty passwords
/// before the PRF runs (unless the low-level path below was chosen),
/// and bounded inputs cannot trip the lanes' length bounds. It
/// mirrors the reference's `map_err` at each call site.
Uint8List _namedPrf(
  Uint8List Function(List<int> key, List<int> message) hmacFn,
  List<int> key,
  List<int> message,
) {
  try {
    return hmacFn(key, message);
    // coverage:ignore-start
    // Unreachable (see above); preserved so a future HMAC lane
    // change surfaces as data rather than a new exception type.
  } on HmacError {
    throw const Pbkdf2PrfError();
    // coverage:ignore-end
  }
}

/// PBKDF2 with HMAC-SHA1 (hLen 20). Powers WPA2 (4096 iterations);
/// prefer SHA-256 for new systems.
///
/// Throws [Pbkdf2Error] on invalid inputs. An empty [password] is
/// permitted only when [allowEmptyPassword] is set, in which case
/// the guard-free generic HMAC core runs (RFC-compliant, e.g. for
/// scrypt's RFC 7914 vectors).
Uint8List pbkdf2HmacSha1(
  List<int> password,
  List<int> salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) {
  if (allowEmptyPassword && password.isEmpty) {
    return _pbkdf2Core(
      (key, message) => hmac(sha1lane.sum1, 64, key, message),
      20,
      password,
      salt,
      iterations,
      keyLength,
      true,
    );
  }
  return _pbkdf2Core(
    (key, message) => _namedPrf(hmacSha1, key, message),
    20,
    password,
    salt,
    iterations,
    keyLength,
    allowEmptyPassword,
  );
}

/// PBKDF2 with HMAC-SHA256 (hLen 32). Recommended for new systems.
///
/// Throws [Pbkdf2Error] on invalid inputs; see [pbkdf2HmacSha1] for
/// the [allowEmptyPassword] contract.
Uint8List pbkdf2HmacSha256(
  List<int> password,
  List<int> salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) {
  if (allowEmptyPassword && password.isEmpty) {
    return _pbkdf2Core(
      (key, message) => hmac(sha256lane.sha256, 64, key, message),
      32,
      password,
      salt,
      iterations,
      keyLength,
      true,
    );
  }
  return _pbkdf2Core(
    (key, message) => _namedPrf(hmacSha256, key, message),
    32,
    password,
    salt,
    iterations,
    keyLength,
    allowEmptyPassword,
  );
}

/// PBKDF2 with HMAC-SHA512 (hLen 64, 128-byte block).
///
/// Throws [Pbkdf2Error] on invalid inputs; see [pbkdf2HmacSha1] for
/// the [allowEmptyPassword] contract.
Uint8List pbkdf2HmacSha512(
  List<int> password,
  List<int> salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) {
  if (allowEmptyPassword && password.isEmpty) {
    return _pbkdf2Core(
      (key, message) => hmac(sha512lane.sha512, 128, key, message),
      64,
      password,
      salt,
      iterations,
      keyLength,
      true,
    );
  }
  return _pbkdf2Core(
    (key, message) => _namedPrf(hmacSha512, key, message),
    64,
    password,
    salt,
    iterations,
    keyLength,
    allowEmptyPassword,
  );
}

/// [pbkdf2HmacSha1] as a lowercase hex string.
String pbkdf2HmacSha1Hex(
  List<int> password,
  List<int> salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) =>
    _toHex(pbkdf2HmacSha1(password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmptyPassword));

/// [pbkdf2HmacSha256] as a lowercase hex string.
String pbkdf2HmacSha256Hex(
  List<int> password,
  List<int> salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) =>
    _toHex(pbkdf2HmacSha256(password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmptyPassword));

/// [pbkdf2HmacSha512] as a lowercase hex string.
String pbkdf2HmacSha512Hex(
  List<int> password,
  List<int> salt, {
  required int iterations,
  required int keyLength,
  bool allowEmptyPassword = false,
}) =>
    _toHex(pbkdf2HmacSha512(password, salt,
        iterations: iterations,
        keyLength: keyLength,
        allowEmptyPassword: allowEmptyPassword));

String _toHex(List<int> bytes) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in bytes) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}
