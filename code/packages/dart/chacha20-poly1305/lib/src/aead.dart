/// ChaCha20-Poly1305 AEAD (RFC 8439 §2.8) and XChaCha20-Poly1305
/// (draft-irtf-cfrg-xchacha).
///
/// AEAD composition: one-time Poly1305 key from ChaCha counter 0,
/// CTR encryption from counter 1, tag over padded AAD/ciphertext
/// plus 64-bit lengths. XChaCha20 stretches the nonce to 192 bits
/// via HChaCha20 subkey derivation.
library;

import 'dart:typed_data';

import 'package:coding_adventures_zeroize/zeroize.dart';

import 'chacha.dart';
import 'errors.dart';
import 'poly1305.dart';

/// Validates fixed-AEAD public lengths, returning (aadLength,
/// inputLength) for the length block.
///
/// Public-in-source (hidden from the barrel) so tests can pin the
/// counter-exhaustion boundary with plain integers — no allocation
/// is needed to check lengths no caller could ever buffer —
/// following the reference's white-box test.
(BigInt, BigInt) validateAeadRequestLengths(
  int inputLength,
  int outputLength,
  int aadLength,
) {
  if (outputLength != inputLength) {
    throw const AeadOutputLengthMismatch();
  }
  final maxU64 = (BigInt.one << 64) - BigInt.one;
  if (BigInt.from(aadLength) > maxU64 || BigInt.from(inputLength) > maxU64) {
    throw const AeadLengthOverflow();
  }
  final blockCount =
      (BigInt.from(inputLength) + BigInt.from(63)) ~/ BigInt.from(64);
  if (blockCount > BigInt.from(0xffffffff)) {
    throw const AeadCounterExhausted();
  }
  return (BigInt.from(aadLength), BigInt.from(inputLength));
}

/// Maps fixed-ChaCha20 failures onto fixed-AEAD failures.
///
/// Public-in-source for the same white-box test.
FixedAeadError mapFixedChacha20Error(FixedChaCha20Error error) {
  switch (error) {
    case ChaCha20OutputLengthMismatch():
      return const AeadOutputLengthMismatch();
    case ChaCha20CounterExhausted():
      return const AeadCounterExhausted();
  }
}

void _checkKeyNonce(List<int> key, List<int> nonce) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (nonce.length != 12) {
    throw ArgumentError('nonce must be exactly 12 bytes');
  }
}

Uint8List _derivePolyKey(List<int> key, List<int> nonce) {
  final block = ZeroizingBytes(Uint8List(64));
  try {
    chachaBlockInto(key, nonce, 0, block.bytes);
    return Uint8List.fromList(block.bytes.sublist(0, 32));
  } finally {
    block.dispose();
  }
}

/// Encrypts and authenticates into exact caller-owned [ciphertext]
/// and [tag].
///
/// Lengths and the counter range validate before either output
/// changes; the one-time key, its source block, and MAC state wipe
/// on return. Callers must guarantee unique key/nonce pairs.
///
/// Throws [FixedAeadError] on violations, and [ArgumentError]
/// unless [key], [nonce], and [tag] are exactly 32/12/16 bytes.
void aeadEncryptFixed(
  List<int> plaintext,
  List<int> key,
  List<int> nonce,
  List<int> aad,
  Uint8List ciphertext,
  Uint8List tag,
) {
  _checkKeyNonce(key, nonce);
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final lengths = validateAeadRequestLengths(
      plaintext.length, ciphertext.length, aad.length);

  final polyKey = ZeroizingBytes(_derivePolyKey(key, nonce));
  final computedTag = ZeroizingBytes(Uint8List(16));
  try {
    try {
      chacha20XorFixed(plaintext, key, nonce, 1, ciphertext);
      // coverage:ignore-start
      // Unreachable: the preflight above already proved matching
      // lengths and counter space. Preserved so a future ChaCha20
      // lane change surfaces as data rather than a new type.
    } on FixedChaCha20Error catch (e) {
      throw mapFixedChacha20Error(e);
      // coverage:ignore-end
    }
    poly1305AeadTagFixed(aad, ciphertext, lengths.$1, lengths.$2, polyKey.bytes,
        computedTag.bytes);
    tag.setRange(0, 16, computedTag.bytes);
  } finally {
    polyKey.dispose();
    computedTag.dispose();
  }
}

/// Authenticates and decrypts into exact caller-owned [plaintext].
///
/// The complete tag recomputes and compares on a fixed schedule
/// before anything releases; failures leave [plaintext] unchanged.
///
/// Throws [FixedAeadError] (including [AeadAuthenticationFailed]),
/// and [ArgumentError] unless [key], [nonce], and [tag] are exactly
/// 32/12/16 bytes.
void aeadDecryptFixed(
  List<int> ciphertext,
  List<int> key,
  List<int> nonce,
  List<int> aad,
  List<int> tag,
  Uint8List plaintext,
) {
  _checkKeyNonce(key, nonce);
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final lengths = validateAeadRequestLengths(
      ciphertext.length, plaintext.length, aad.length);

  final polyKey = ZeroizingBytes(_derivePolyKey(key, nonce));
  final computedTag = ZeroizingBytes(Uint8List(16));
  try {
    poly1305AeadTagFixed(aad, ciphertext, lengths.$1, lengths.$2, polyKey.bytes,
        computedTag.bytes);
    var difference = 0;
    for (var index = 0; index < 16; index++) {
      difference |= computedTag.bytes[index] ^ tag[index];
    }
    if (difference != 0) {
      throw const AeadAuthenticationFailed();
    }
    try {
      chacha20XorFixed(ciphertext, key, nonce, 1, plaintext);
      // coverage:ignore-start
      // Unreachable: same preflight proof as the encrypt path.
    } on FixedChaCha20Error catch (e) {
      throw mapFixedChacha20Error(e);
      // coverage:ignore-end
    }
  } finally {
    polyKey.dispose();
    computedTag.dispose();
  }
}

Uint8List _pad16(int length) {
  final remainder = length % 16;
  return remainder == 0 ? Uint8List(0) : Uint8List(16 - remainder);
}

/// Encrypts with ChaCha20-Poly1305, returning `(ciphertext, tag)`.
///
/// Throws [ArgumentError] unless [key] and [nonce] are exactly
/// 32/12 bytes.
(Uint8List, Uint8List) aeadEncrypt(
  List<int> plaintext,
  List<int> key,
  List<int> nonce,
  List<int> aad,
) {
  _checkKeyNonce(key, nonce);
  final polyKey = ZeroizingBytes(_derivePolyKey(key, nonce));
  try {
    final ciphertext = chacha20Encrypt(plaintext, key, nonce, 1);
    final macData = [
      ...aad,
      ..._pad16(aad.length),
      ...ciphertext,
      ..._pad16(ciphertext.length),
      ..._le64(aad.length),
      ..._le64(ciphertext.length),
    ];
    final tag = poly1305Mac(macData, polyKey.bytes);
    return (ciphertext, tag);
  } finally {
    polyKey.dispose();
  }
}

/// Decrypts ChaCha20-Poly1305, returning the plaintext, or null when
/// authentication fails (constant-time tag compare first — never
/// releases unauthenticated bytes).
///
/// Throws [ArgumentError] unless [key], [nonce], and [tag] are
/// exactly 32/12/16 bytes.
Uint8List? aeadDecrypt(
  List<int> ciphertext,
  List<int> key,
  List<int> nonce,
  List<int> aad,
  List<int> tag,
) {
  _checkKeyNonce(key, nonce);
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final polyKey = ZeroizingBytes(_derivePolyKey(key, nonce));
  try {
    final macData = [
      ...aad,
      ..._pad16(aad.length),
      ...ciphertext,
      ..._pad16(ciphertext.length),
      ..._le64(aad.length),
      ..._le64(ciphertext.length),
    ];
    final computedTag = poly1305Mac(macData, polyKey.bytes);
    var diff = 0;
    for (var i = 0; i < 16; i++) {
      diff |= computedTag[i] ^ tag[i];
    }
    if (diff != 0) {
      return null;
    }
    return chacha20Encrypt(ciphertext, key, nonce, 1);
  } finally {
    polyKey.dispose();
  }
}

List<int> _le64(int value) => [
      value & 0xff,
      (value >> 8) & 0xff,
      (value >> 16) & 0xff,
      (value >> 24) & 0xff,
      (value >> 32) & 0xff,
      (value >> 40) & 0xff,
      (value >> 48) & 0xff,
      (value >> 56) & 0xff,
    ];

/// XChaCha20 stream cipher: subkey from the nonce's first 16 bytes,
/// then ChaCha20 under `[0,0,0,0] || nonce[16..24]`.
///
/// The counter wraps mod 2^32 like [chacha20Encrypt]; out-of-range
/// counters are rejected.
///
/// Throws [ArgumentError] unless [key], [nonce24], and [counter]
/// fit their exact widths.
Uint8List xchacha20Encrypt(
  List<int> plaintext,
  List<int> key,
  List<int> nonce24,
  int counter,
) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (nonce24.length != 24) {
    throw ArgumentError('nonce must be exactly 24 bytes');
  }
  final subkey = ZeroizingBytes(hchacha20Subkey(key, nonce24.sublist(0, 16)));
  try {
    final nonce12 = Uint8List(12)..setRange(4, 12, nonce24.sublist(16, 24));
    return chacha20Encrypt(plaintext, subkey.bytes, nonce12, counter);
  } finally {
    subkey.dispose();
  }
}

/// XChaCha20-Poly1305 AEAD encryption, returning
/// `(ciphertext, tag)`.
///
/// Throws [ArgumentError] unless [key] and [nonce24] are exactly
/// 32/24 bytes.
(Uint8List, Uint8List) xchacha20Poly1305AeadEncrypt(
  List<int> plaintext,
  List<int> key,
  List<int> nonce24,
  List<int> aad,
) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (nonce24.length != 24) {
    throw ArgumentError('nonce must be exactly 24 bytes');
  }
  final subkey = ZeroizingBytes(hchacha20Subkey(key, nonce24.sublist(0, 16)));
  try {
    final nonce12 = Uint8List(12)..setRange(4, 12, nonce24.sublist(16, 24));
    return aeadEncrypt(plaintext, subkey.bytes, nonce12, aad);
  } finally {
    subkey.dispose();
  }
}

/// XChaCha20-Poly1305 AEAD decryption: plaintext on a valid tag,
/// null otherwise.
///
/// Throws [ArgumentError] unless [key], [nonce24], and [tag] are
/// exactly 32/24/16 bytes.
Uint8List? xchacha20Poly1305AeadDecrypt(
  List<int> ciphertext,
  List<int> key,
  List<int> nonce24,
  List<int> aad,
  List<int> tag,
) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (nonce24.length != 24) {
    throw ArgumentError('nonce must be exactly 24 bytes');
  }
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final subkey = ZeroizingBytes(hchacha20Subkey(key, nonce24.sublist(0, 16)));
  try {
    final nonce12 = Uint8List(12)..setRange(4, 12, nonce24.sublist(16, 24));
    return aeadDecrypt(ciphertext, subkey.bytes, nonce12, aad, tag);
  } finally {
    subkey.dispose();
  }
}
