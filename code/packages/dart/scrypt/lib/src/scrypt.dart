/// scrypt sequential memory-hard password hashing (RFC 7914).
///
/// ```text
/// B  = PBKDF2-HMAC-SHA256(password, salt, 1, p * 128 * r)
/// B[i] = ROMix(B[i], N) for each of p blocks  (the memory-hard step)
/// DK = PBKDF2-HMAC-SHA256(password, B, 1, dkLen)
/// ```
///
/// ROMix fills a table V with N snapshots of BlockMix-chained state
/// (`N * 128 * r` bytes), then re-mixes while randomly reading V at
/// data-dependent indices — keeping all of V in RAM simultaneously
/// is what makes the function memory-hard. V here is one flat
/// buffer rather than the reference's nested vectors; the access
/// pattern is identical.
library;

import 'dart:typed_data';

import 'package:coding_adventures_pbkdf2/pbkdf2.dart';

import 'errors.dart';
import 'salsa.dart';

/// BlockMix over 2*r 64-byte blocks: chain
/// `X = Salsa20/8(X XOR B_i)`, then emit even-indexed outputs
/// followed by odd-indexed ones (the RFC's cache-friendly
/// interleave).
Uint8List _blockMix(Uint8List blocks, int r) {
  final twoR = 2 * r;
  var x = Uint8List.fromList(blocks.sublist((twoR - 1) * 64, twoR * 64));
  final y = Uint8List(twoR * 64);
  for (var i = 0; i < twoR; i++) {
    final xored = Uint8List(64);
    for (var k = 0; k < 64; k++) {
      xored[k] = x[k] ^ blocks[i * 64 + k];
    }
    x = salsa208(xored);
    y.setRange(i * 64, (i + 1) * 64, x);
  }
  final out = Uint8List(twoR * 64);
  for (var i = 0; i < r; i++) {
    out.setRange(i * 64, (i + 1) * 64, y.sublist(2 * i * 64, (2 * i + 1) * 64));
    out.setRange((r + i) * 64, (r + i + 1) * 64,
        y.sublist((2 * i + 1) * 64, (2 * i + 2) * 64));
  }
  return out;
}

/// RFC 7914 `integerify`: the last block's first 8 bytes as a
/// little-endian integer, reduced mod [n]. The full 64-bit range
/// needs [BigInt] (Dart ints are signed 63-bit-positive).
int _integerify(Uint8List x, int n) {
  var v = BigInt.zero;
  final base = x.length - 64;
  for (var i = 7; i >= 0; i--) {
    v = (v << 8) | BigInt.from(x[base + i]);
  }
  return (v % BigInt.from(n)).toInt();
}

/// ROMix over one 128*r-byte block: fill V with N BlockMix
/// snapshots, then N passes of data-dependent V reads XORed into
/// the running state.
Uint8List _roMix(Uint8List b, int n, int r) {
  final blockLen = 128 * r;
  var x = Uint8List.fromList(b);
  final v = Uint8List(n * blockLen);
  for (var i = 0; i < n; i++) {
    v.setRange(i * blockLen, (i + 1) * blockLen, x);
    x = _blockMix(x, r);
  }
  for (var i = 0; i < n; i++) {
    final j = _integerify(x, n);
    final entry = j * blockLen;
    for (var k = 0; k < blockLen; k++) {
      x[k] ^= v[entry + k];
    }
    x = _blockMix(x, r);
  }
  return x;
}

Uint8List _pbkdf2Step(List<int> password, List<int> salt, int keyLength) {
  try {
    return pbkdf2HmacSha256(password, salt,
        iterations: 1, keyLength: keyLength, allowEmptyPassword: true);
  } on Pbkdf2Error {
    throw const ScryptHmacError();
  }
}

/// Derives [dkLen] bytes from [password] and [salt] with cost
/// parameters [n] (CPU/memory), [r] (block size), [p] (parallelism).
///
/// Memory usage is `N * 128 * r` bytes per ROMix call. Empty
/// passwords and salts are accepted (RFC 7914 vector 1 needs both
/// empty).
///
/// Throws [ScryptError] on invalid parameters. Checks run in
/// reference order: [ScryptNTooLarge] before [ScryptInvalidN] so an
/// over-cap N reports the cap even when it is also not a power of
/// two.
Uint8List scrypt(
  List<int> password,
  List<int> salt, {
  required int n,
  required int r,
  required int p,
  required int dkLen,
}) {
  if (n > (1 << 20)) {
    throw const ScryptNTooLarge();
  }
  if (n < 2 || (n & (n - 1)) != 0) {
    throw const ScryptInvalidN();
  }
  if (r < 1) {
    throw const ScryptInvalidR();
  }
  if (p < 1) {
    throw const ScryptInvalidP();
  }
  if (dkLen < 1) {
    throw const ScryptInvalidKeyLength();
  }
  if (dkLen > (1 << 20)) {
    throw const ScryptKeyLengthTooLarge();
  }
  // BigInt throughout: Dart ints wrap silently on overflow, which
  // would let absurd p*r slip under the cap (the reference uses
  // saturating/checked 64-bit arithmetic for the same reason).
  final pr = BigInt.from(p) * BigInt.from(r);
  if (pr > BigInt.from(1 << 30)) {
    throw const ScryptPRTooLarge();
  }
  final bLen = pr * BigInt.from(128);
  if (bLen > BigInt.from(1 << 30)) {
    throw const ScryptPRTooLarge();
  }

  final b = _pbkdf2Step(password, salt, bLen.toInt());
  final chunkLen = 128 * r;
  for (var i = 0; i < p; i++) {
    final chunk =
        Uint8List.fromList(b.sublist(i * chunkLen, (i + 1) * chunkLen));
    final mixed = _roMix(chunk, n, r);
    b.setRange(i * chunkLen, (i + 1) * chunkLen, mixed);
  }
  return _pbkdf2Step(password, b, dkLen);
}

/// [scrypt] as a lowercase hex string.
String scryptHex(
  List<int> password,
  List<int> salt, {
  required int n,
  required int r,
  required int p,
  required int dkLen,
}) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in scrypt(password, salt, n: n, r: r, p: p, dkLen: dkLen)) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}
