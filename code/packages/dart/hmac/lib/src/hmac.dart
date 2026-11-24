/// HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1.
///
/// HMAC takes a secret key and a message and produces a fixed-size
/// authentication tag proving both message integrity and authenticity.
/// It defeats length-extension attacks on Merkle-Damgård hashes with
/// two nested hash calls under different padded keys:
///
/// ```text
/// HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
/// ```
///
/// where `ipad = 0x36` and `opad = 0x5C`, repeated to the block size.
///
/// ## Memory hygiene
///
/// Key-derived intermediates (`K' XOR ipad`, `K' XOR opad`, the inner
/// digest, normalized key blocks) are secret material in lightly
/// disguised form — anyone reading the buffer recovers the key by
/// XORing the pad back out. This port wipes every intermediate it owns
/// via the in-lane zeroize package (explicit `dispose()` in `finally`
/// blocks — Dart has no destructors), mirroring the reference's
/// wipe-on-drop discipline. Nested inputs are built at exact capacity
/// so no reallocation abandons an un-wiped copy. The returned tag
/// belongs to the caller. The zeroize lane docs' platform limits
/// (no fences, GC copies) apply unchanged.
library;

import 'dart:typed_data';

import 'package:coding_adventures_ct_compare/coding_adventures_ct_compare.dart';
import 'package:coding_adventures_md5/coding_adventures_md5.dart';
import 'package:coding_adventures_sha1/coding_adventures_sha1.dart';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart';
import 'package:coding_adventures_sha384/sha384.dart';
import 'package:coding_adventures_sha512/coding_adventures_sha512.dart';
import 'package:coding_adventures_zeroize/zeroize.dart';

import 'errors.dart';

/// Inner pad byte (RFC 2104 §2).
const int _ipad = 0x36;

/// Outer pad byte (RFC 2104 §2).
const int _opad = 0x5c;

/// Compute HMAC using any hash function.
///
/// [hashFn] maps message bytes to a digest; [blockSize] is its
/// internal block size in bytes (64 or 128). Returns the tag, whose
/// length equals the hash output length.
Uint8List hmac(
  Uint8List Function(List<int> data) hashFn,
  int blockSize,
  List<int> key,
  List<int> message,
) {
  // Step 1 — normalize key to exactly blockSize bytes.
  final keyPrime = _normalizeKey(hashFn, blockSize, key);
  try {
    // Steps 2-4 — inner keyed input at exact capacity, then hash.
    final innerInput = ZeroizingBytes(
      Uint8List(blockSize + message.length),
    );
    try {
      for (var i = 0; i < blockSize; i++) {
        innerInput.bytes[i] = keyPrime[i] ^ _ipad;
      }
      innerInput.bytes.setRange(blockSize, blockSize + message.length, message);
      final inner = ZeroizingBytes(hashFn(innerInput.bytes));
      try {
        // Step 5 — outer keyed input at exact capacity, then hash.
        final outerInput = ZeroizingBytes(
          Uint8List(blockSize + inner.bytes.length),
        );
        try {
          for (var i = 0; i < blockSize; i++) {
            outerInput.bytes[i] = keyPrime[i] ^ _opad;
          }
          outerInput.bytes.setRange(
            blockSize,
            blockSize + inner.bytes.length,
            inner.bytes,
          );
          return hashFn(outerInput.bytes);
        } finally {
          outerInput.dispose();
        }
      } finally {
        inner.dispose();
      }
    } finally {
      innerInput.dispose();
    }
  } finally {
    zeroizeBytes(keyPrime);
  }
}

/// Normalize key to exactly [blockSize] bytes: hash long keys, pad
/// short ones. The caller owns the returned buffer and must wipe it.
Uint8List _normalizeKey(
  Uint8List Function(List<int> data) hashFn,
  int blockSize,
  List<int> key,
) {
  final Uint8List keyBytes;
  final bool owned;
  if (key.length > blockSize) {
    keyBytes = hashFn(key);
    owned = true;
  } else if (key is Uint8List) {
    // The caller's slice is only read, never retained.
    keyBytes = key;
    owned = false;
  } else {
    // Our own copy of the raw key: wipe it after normalizing.
    keyBytes = Uint8List.fromList(key);
    owned = true;
  }
  final out = Uint8List(blockSize);
  final copyLen = keyBytes.length < blockSize ? keyBytes.length : blockSize;
  out.setRange(0, copyLen, keyBytes);
  if (owned) {
    zeroizeBytes(keyBytes);
  }
  return out;
}

Uint8List _fixedTag(Uint8List Function() compute, int length) {
  final result = compute();
  final tag = Uint8List.fromList(result.sublist(0, length));
  zeroizeBytes(result);
  return tag;
}

/// HMAC-MD5: 16-byte authentication tag.
///
/// HMAC-MD5 remains secure as a MAC even though MD5 is broken for
/// collision resistance — MAC security is a different property.
///
/// Throws [HmacEmptyKey] if [key] is empty.
Uint8List hmacMd5(List<int> key, List<int> message) {
  if (key.isEmpty) {
    throw const HmacEmptyKey();
  }
  return _fixedTag(() => hmac(sumMd5, 64, key, message), 16);
}

/// HMAC-SHA1: 20-byte authentication tag.
///
/// Throws [HmacEmptyKey] if [key] is empty.
Uint8List hmacSha1(List<int> key, List<int> message) {
  if (key.isEmpty) {
    throw const HmacEmptyKey();
  }
  return _fixedTag(() => hmac(sum1, 64, key, message), 20);
}

/// Fixed-memory HMAC-SHA256: 32-byte authentication tag.
///
/// Uses two streaming passes and stack-resident key material, so the
/// HMAC core performs no heap allocation beyond the hashers'
/// bounded buffers. Wipes key blocks, pads, the long-key digest,
/// and the inner digest at their ownership boundaries. (The sha256
/// lane offers no hasher dispose, so its transient states are not
/// scrubbable — a dependency limitation.)
///
/// Throws [HmacEmptyKey] if [key] is empty. The Dart sha256 lane
/// reports no length failures, so [HmacInputTooLong] is unreachable
/// on this path (kept for contract symmetry with the reference).
Uint8List hmacSha256(List<int> key, List<int> message) =>
    hmacSha256Segments(key, [message]);

/// Fixed-memory HMAC-SHA256 over message segments.
///
/// Segment boundaries do not affect the tag: segments hash in order
/// as one logical message, with no concatenated whole-message buffer.
/// An empty segment list (and any empty segments) represent an empty
/// message.
Uint8List hmacSha256Segments(List<int> key, List<List<int>> segments) {
  if (key.isEmpty) {
    throw const HmacEmptyKey();
  }

  final keyBlock = ZeroizingBytes(Uint8List(64));
  try {
    if (key.length > 64) {
      final keyHasher = Sha256Hasher();
      keyHasher.update(key);
      final digest = ZeroizingBytes(keyHasher.digest());
      try {
        keyBlock.bytes.setRange(0, digest.bytes.length, digest.bytes);
      } finally {
        digest.dispose();
      }
    } else {
      keyBlock.bytes.setRange(0, key.length, key);
    }

    for (var i = 0; i < 64; i++) {
      keyBlock.bytes[i] ^= _ipad;
    }
    final innerHasher = Sha256Hasher();
    innerHasher.update(keyBlock.bytes);
    for (final segment in segments) {
      innerHasher.update(segment);
    }
    final innerDigest = ZeroizingBytes(innerHasher.digest());
    try {
      // Transform K' XOR ipad into K' XOR opad in place.
      for (var i = 0; i < 64; i++) {
        keyBlock.bytes[i] ^= _ipad ^ _opad;
      }
      final outerHasher = Sha256Hasher();
      outerHasher.update(keyBlock.bytes);
      outerHasher.update(innerDigest.bytes);
      return outerHasher.digest();
    } finally {
      innerDigest.dispose();
    }
  } finally {
    keyBlock.dispose();
  }
}

/// Fixed-memory HMAC-SHA384: 48-byte authentication tag.
///
/// Two streaming passes over the in-lane SHA-384 implementation.
/// Wipes key blocks, pads, the long-key digest, the inner digest,
/// and both hash states.
///
/// Throws [HmacEmptyKey] if [key] is empty, or [HmacInputTooLong] if
/// key normalization or either pass would exceed the encoded
/// message-length bound.
Uint8List hmacSha384(List<int> key, List<int> message) =>
    hmacSha384Segments(key, [message]);

/// Fixed-memory HMAC-SHA384 over message segments.
///
/// Segment boundaries do not affect the tag. An empty segment list
/// (and any empty segments) represent an empty message.
Uint8List hmacSha384Segments(List<int> key, List<List<int>> segments) {
  if (key.isEmpty) {
    throw const HmacEmptyKey();
  }

  final keyBlock = ZeroizingBytes(Uint8List(128));
  try {
    if (key.length > 128) {
      final keyHasher = Sha384Hasher();
      try {
        keyHasher.update(key);
        final digest = ZeroizingBytes(keyHasher.digest());
        try {
          keyBlock.bytes.setRange(0, digest.bytes.length, digest.bytes);
        } finally {
          digest.dispose();
        }
        // Unreachable below 2^127 key bytes; the mapping keeps the
        // reference's HmacError-only contract.
        // coverage:ignore-start
      } on Sha384MessageTooLong {
        throw const HmacInputTooLong();
        // coverage:ignore-end
      } finally {
        keyHasher.dispose();
      }
    } else {
      keyBlock.bytes.setRange(0, key.length, key);
    }

    for (var i = 0; i < 128; i++) {
      keyBlock.bytes[i] ^= _ipad;
    }
    final innerHasher = Sha384Hasher();
    try {
      innerHasher.update(keyBlock.bytes);
      for (final segment in segments) {
        innerHasher.update(segment);
      }
      final innerDigest = ZeroizingBytes(innerHasher.digest());
      try {
        for (var i = 0; i < 128; i++) {
          keyBlock.bytes[i] ^= _ipad ^ _opad;
        }
        final outerHasher = Sha384Hasher();
        try {
          outerHasher.update(keyBlock.bytes);
          outerHasher.update(innerDigest.bytes);
          return outerHasher.digest();
        } finally {
          outerHasher.dispose();
        }
      } finally {
        innerDigest.dispose();
      }
      // Unreachable below 2^127 message bytes; same contract mapping.
      // coverage:ignore-start
    } on Sha384MessageTooLong {
      throw const HmacInputTooLong();
      // coverage:ignore-end
    } finally {
      innerHasher.dispose();
    }
  } finally {
    keyBlock.dispose();
  }
}

/// HMAC-SHA512: 64-byte authentication tag.
///
/// Throws [HmacEmptyKey] if [key] is empty.
Uint8List hmacSha512(List<int> key, List<int> message) {
  if (key.isEmpty) {
    throw const HmacEmptyKey();
  }
  return _fixedTag(() => hmac(sha512, 128, key, message), 64);
}

/// HMAC-MD5 as a 32-character lowercase hex string.
///
/// Throws [HmacEmptyKey] if [key] is empty (the reference panics;
/// Dart has no panics, so the error propagates instead).
String hmacMd5Hex(List<int> key, List<int> message) =>
    _toHex(hmacMd5(key, message));

/// HMAC-SHA1 as a 40-character lowercase hex string.
String hmacSha1Hex(List<int> key, List<int> message) =>
    _toHex(hmacSha1(key, message));

/// HMAC-SHA256 as a 64-character lowercase hex string.
String hmacSha256Hex(List<int> key, List<int> message) =>
    _toHex(hmacSha256(key, message));

/// HMAC-SHA384 as a 96-character lowercase hex string.
String hmacSha384Hex(List<int> key, List<int> message) =>
    _toHex(hmacSha384(key, message));

/// HMAC-SHA512 as a 128-character lowercase hex string.
String hmacSha512Hex(List<int> key, List<int> message) =>
    _toHex(hmacSha512(key, message));

/// Compare two HMAC tags in constant time.
///
/// Delegates to the in-lane constant-time primitive for equal-length
/// inputs; mismatched lengths return `false` without scanning (lengths
/// are public). Use this instead of `==`, which short-circuits on the
/// first differing byte and leaks timing information.
bool verifyHmac(List<int> expected, List<int> actual) {
  if (expected.length != actual.length) {
    return false;
  }
  return ctEq(
    expected is Uint8List ? expected : Uint8List.fromList(expected),
    actual is Uint8List ? actual : Uint8List.fromList(actual),
  );
}

String _toHex(List<int> bytes) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in bytes) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}
