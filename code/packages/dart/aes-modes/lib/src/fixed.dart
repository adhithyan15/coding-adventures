/// Fixed-memory GHASH and AES-128-GCM (NIST SP 800-38D).
///
/// The allocation-free boundaries: GF(2^128) multiplication with
/// mask-selected (branch-free coded) decisions, GHASH absorption
/// into caller-owned storage, and AES-128-GCM encrypt/decrypt with
/// complete preflight validation before either output changes.
///
/// Secret-derived temporaries live in [ZeroizingBytes] owners
/// disposed in `finally` blocks (Dart has no destructors). The
/// zeroize lane docs' platform limits apply unchanged; in
/// particular, Dart offers no optimizer barrier like the
/// reference's `black_box`, so the mask discipline is best-effort
/// rather than a verified constant-time promise.
library;

import 'dart:typed_data';

import 'package:coding_adventures_aes/aes.dart' as aes;
import 'package:coding_adventures_zeroize/zeroize.dart';

import 'errors.dart';

/// AES block size in bytes.
const int _blockSize = 16;

/// Maximum GCM payload in blocks: 2^32 - 2 (counter 0 is J0, and
/// the top value is reserved).
final BigInt _maxPayloadBlocks = (BigInt.one << 32) - BigInt.two;

/// Multiplies two 128-bit values in GF(2^128) under GCM's reducing
/// polynomial (R high byte 0xE1), writing into caller-owned
/// [output].
///
/// NIST SP 800-38D Algorithm 1 over 128 fixed rounds: each input
/// bit selects the multiplier by mask, and the right shift reduces
/// by mask on carry. Product and working multiplier are wiped on
/// return.
///
/// Throws [ArgumentError] unless all three buffers are exactly 16
/// bytes (the reference takes fixed-size arrays, which admit no
/// other lengths).
void ghashMultiplyFixed(List<int> x, List<int> h, Uint8List output) {
  if (x.length != 16) {
    throw ArgumentError('x must be exactly 16 bytes');
  }
  if (h.length != 16) {
    throw ArgumentError('h must be exactly 16 bytes');
  }
  if (output.length != 16) {
    throw ArgumentError('output must be exactly 16 bytes');
  }
  final z = ZeroizingBytes(Uint8List(16));
  final v = ZeroizingBytes(Uint8List.fromList(h));
  try {
    for (var i = 0; i < 128; i++) {
      final bit = (x[i ~/ 8] >> (7 - (i % 8))) & 1;
      final bitMask = (-bit) & 0xff;
      for (var j = 0; j < 16; j++) {
        z.bytes[j] ^= v.bytes[j] & bitMask;
      }
      final carry = v.bytes[15] & 1;
      for (var j = 15; j >= 1; j--) {
        v.bytes[j] = (v.bytes[j] >> 1) | ((v.bytes[j - 1] & 1) << 7);
      }
      v.bytes[0] >>= 1;
      v.bytes[0] ^= 0xe1 & ((-carry) & 0xff);
    }
    output.setRange(0, 16, z.bytes);
  } finally {
    z.dispose();
    v.dispose();
  }
}

/// The bit length of [byteLength], throwing [GhashInputTooLong]
/// when it cannot be represented in 64 bits.
BigInt _ghashBitLength(int byteLength) {
  final bits = BigInt.from(byteLength) * BigInt.from(8);
  if (bits > (BigInt.one << 64) - BigInt.one) {
    throw const GhashInputTooLong();
  }
  return bits;
}

void _ghashAbsorb(
  List<int> h,
  List<int> data,
  Uint8List accumulator,
  Uint8List block,
) {
  for (var offset = 0; offset < data.length; offset += 16) {
    final end = offset + 16 < data.length ? offset + 16 : data.length;
    zeroizeBytes(block);
    block.setRange(0, end - offset, data.sublist(offset, end));
    for (var index = 0; index < 16; index++) {
      block[index] ^= accumulator[index];
    }
    final next = Uint8List(16);
    ghashMultiplyFixed(block, h, next);
    accumulator.setRange(0, 16, next);
    zeroizeBytes(next);
  }
}

/// Computes GHASH into caller-owned fixed [output]:
/// `Y[i] = (Y[i-1] XOR block[i]) * H` over AAD blocks, ciphertext
/// blocks, then the 128-bit big-endian length block.
///
/// Lengths validate before [output] changes; accumulator and block
/// are wiped on all exits.
///
/// Throws [GhashInputTooLong] when an input exceeds the 64-bit
/// bit-length field, and [ArgumentError] unless [h] and [output]
/// are exactly 16 bytes.
void ghashFixed(
  List<int> h,
  List<int> aad,
  List<int> ciphertext,
  Uint8List output,
) {
  if (h.length != 16) {
    throw ArgumentError('h must be exactly 16 bytes');
  }
  if (output.length != 16) {
    throw ArgumentError('output must be exactly 16 bytes');
  }
  final aadBits = _ghashBitLength(aad.length);
  final ciphertextBits = _ghashBitLength(ciphertext.length);
  _ghashFixedValidated(h, aad, ciphertext, aadBits, ciphertextBits, output);
}

void _ghashFixedValidated(
  List<int> h,
  List<int> aad,
  List<int> ciphertext,
  BigInt aadBits,
  BigInt ciphertextBits,
  Uint8List output,
) {
  final accumulator = ZeroizingBytes(Uint8List(16));
  final block = ZeroizingBytes(Uint8List(16));
  try {
    _ghashAbsorb(h, aad, accumulator.bytes, block.bytes);
    _ghashAbsorb(h, ciphertext, accumulator.bytes, block.bytes);
    zeroizeBytes(block.bytes);
    for (var i = 0; i < 8; i++) {
      block.bytes[i] = ((aadBits >> (8 * (7 - i))) & BigInt.from(0xff)).toInt();
      block.bytes[8 + i] =
          ((ciphertextBits >> (8 * (7 - i))) & BigInt.from(0xff)).toInt();
    }
    for (var index = 0; index < 16; index++) {
      block.bytes[index] ^= accumulator.bytes[index];
    }
    final next = Uint8List(16);
    ghashMultiplyFixed(block.bytes, h, next);
    accumulator.bytes.setRange(0, 16, next);
    zeroizeBytes(next);
    output.setRange(0, 16, accumulator.bytes);
  } finally {
    accumulator.dispose();
    block.dispose();
  }
}

/// Validates fixed-GCM public lengths, returning (aadBits,
/// inputBits) for the length block.
///
/// Public-in-source (hidden from the barrel) so tests can pin the
/// counter-exhaustion boundary with plain integers, following the
/// reference's white-box test: no allocation is needed to check
/// lengths that no caller could ever buffer.
(BigInt, BigInt) validateFixedAes128GcmLengths(
  int inputLength,
  int outputLength,
  int aadLength,
) {
  if (outputLength != inputLength) {
    throw const GcmOutputLengthMismatch();
  }
  BigInt aadBits;
  BigInt inputBits;
  try {
    aadBits = _ghashBitLength(aadLength);
    inputBits = _ghashBitLength(inputLength);
  } on GhashInputTooLong {
    throw const GcmLengthOverflow();
  }
  final payloadBlocks =
      (BigInt.from(inputLength) + BigInt.from(_blockSize - 1)) ~/
          BigInt.from(_blockSize);
  if (payloadBlocks > _maxPayloadBlocks) {
    throw const GcmCounterExhausted();
  }
  return (aadBits, inputBits);
}

void _ctrXorFixed(
  List<int> input,
  Uint8List key,
  Uint8List nonce,
  Uint8List output,
) {
  final counterBlock = ZeroizingBytes(Uint8List(16));
  final keystream = ZeroizingBytes(Uint8List(16));
  try {
    counterBlock.bytes.setRange(0, 12, nonce);
    var offset = 0;
    var counter = 2;
    while (offset < input.length) {
      counterBlock.bytes[12] = (counter >> 24) & 0xff;
      counterBlock.bytes[13] = (counter >> 16) & 0xff;
      counterBlock.bytes[14] = (counter >> 8) & 0xff;
      counterBlock.bytes[15] = counter & 0xff;
      aes.aes128EncryptBlockFixed(counterBlock.bytes, key, keystream.bytes);
      final remaining = input.length - offset;
      final blockLength = remaining < _blockSize ? remaining : _blockSize;
      for (var index = 0; index < blockLength; index++) {
        output[offset + index] = input[offset + index] ^ keystream.bytes[index];
      }
      offset += blockLength;
      if (offset < input.length) {
        counter = (counter + 1) & 0xffffffff;
      }
    }
  } finally {
    counterBlock.dispose();
    keystream.dispose();
  }
}

void _tagFixed(
  Uint8List key,
  Uint8List nonce,
  Uint8List hashSubkey,
  List<int> aad,
  List<int> ciphertext,
  (BigInt, BigInt) lengthBits,
  Uint8List output,
) {
  final ghashResult = ZeroizingBytes(Uint8List(16));
  final j0 = ZeroizingBytes(Uint8List(16));
  final encryptedJ0 = ZeroizingBytes(Uint8List(16));
  try {
    _ghashFixedValidated(hashSubkey, aad, ciphertext, lengthBits.$1,
        lengthBits.$2, ghashResult.bytes);
    j0.bytes.setRange(0, 12, nonce);
    j0.bytes[15] = 1;
    aes.aes128EncryptBlockFixed(j0.bytes, key, encryptedJ0.bytes);
    for (var index = 0; index < 16; index++) {
      output[index] = ghashResult.bytes[index] ^ encryptedJ0.bytes[index];
    }
  } finally {
    ghashResult.dispose();
    j0.dispose();
    encryptedJ0.dispose();
  }
}

bool _tagsEqualFixed(List<int> computed, List<int> supplied) {
  var difference = 0;
  for (var index = 0; index < 16; index++) {
    difference |= computed[index] ^ supplied[index];
  }
  return difference == 0;
}

/// Encrypts and authenticates with AES-128-GCM into exact
/// caller-owned [ciphertext] and [tag] (RFC 5116 96-bit nonce
/// profile).
///
/// All public lengths and the complete counter range validate
/// before either output changes; encryption runs at `inc32(J0)`
/// and above over the exact produced ciphertext. Key-derived
/// temporaries stay in wipe-on-drop fixed storage.
///
/// Callers must guarantee unique key/nonce pairs. This primitive
/// does not reserve nonces, own traffic keys, enforce TLS record
/// limits, frame records, or provide a transport.
///
/// Throws [FixedAes128GcmError] on length/counter violations, and
/// [ArgumentError] unless [key], [nonce], and [tag] are exactly
/// 16/12/16 bytes.
void aes128GcmEncryptFixed(
  List<int> plaintext,
  List<int> key,
  List<int> nonce,
  List<int> aad,
  Uint8List ciphertext,
  Uint8List tag,
) {
  if (key.length != 16) {
    throw ArgumentError('key must be exactly 16 bytes');
  }
  if (nonce.length != 12) {
    throw ArgumentError('nonce must be exactly 12 bytes');
  }
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final keyBytes = key is Uint8List ? key : Uint8List.fromList(key);
  final nonceBytes = nonce is Uint8List ? nonce : Uint8List.fromList(nonce);
  final lengths = validateFixedAes128GcmLengths(
      plaintext.length, ciphertext.length, aad.length);

  final hashSubkey = ZeroizingBytes(Uint8List(16));
  final computedTag = ZeroizingBytes(Uint8List(16));
  try {
    aes.aes128EncryptBlockFixed(Uint8List(16), keyBytes, hashSubkey.bytes);
    _ctrXorFixed(plaintext, keyBytes, nonceBytes, ciphertext);
    _tagFixed(keyBytes, nonceBytes, hashSubkey.bytes, aad, ciphertext, lengths,
        computedTag.bytes);
    tag.setRange(0, 16, computedTag.bytes);
  } finally {
    hashSubkey.dispose();
    computedTag.dispose();
  }
}

/// Authenticates and decrypts AES-128-GCM into exact caller-owned
/// [plaintext].
///
/// The complete tag recomputes and compares on a fixed schedule
/// before any plaintext releases; authentication failure and every
/// preflight failure leave [plaintext] unchanged.
///
/// Throws [FixedAes128GcmError] (including
/// [GcmFixedAuthenticationFailed] on tag mismatch), and
/// [ArgumentError] unless [key], [nonce], and [tag] are exactly
/// 16/12/16 bytes.
void aes128GcmDecryptFixed(
  List<int> ciphertext,
  List<int> key,
  List<int> nonce,
  List<int> aad,
  List<int> tag,
  Uint8List plaintext,
) {
  if (key.length != 16) {
    throw ArgumentError('key must be exactly 16 bytes');
  }
  if (nonce.length != 12) {
    throw ArgumentError('nonce must be exactly 12 bytes');
  }
  if (tag.length != 16) {
    throw ArgumentError('tag must be exactly 16 bytes');
  }
  final keyBytes = key is Uint8List ? key : Uint8List.fromList(key);
  final nonceBytes = nonce is Uint8List ? nonce : Uint8List.fromList(nonce);
  final lengths = validateFixedAes128GcmLengths(
      ciphertext.length, plaintext.length, aad.length);

  final hashSubkey = ZeroizingBytes(Uint8List(16));
  final computedTag = ZeroizingBytes(Uint8List(16));
  try {
    aes.aes128EncryptBlockFixed(Uint8List(16), keyBytes, hashSubkey.bytes);
    _tagFixed(keyBytes, nonceBytes, hashSubkey.bytes, aad, ciphertext, lengths,
        computedTag.bytes);
    if (!_tagsEqualFixed(computedTag.bytes, tag)) {
      throw const GcmFixedAuthenticationFailed();
    }
    _ctrXorFixed(ciphertext, keyBytes, nonceBytes, plaintext);
  } finally {
    hashSubkey.dispose();
    computedTag.dispose();
  }
}
