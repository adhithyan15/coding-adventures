/// Poly1305 one-time MAC (RFC 8439 §2.5): polynomial evaluation
/// mod p = 2^130 - 5 with clamped multiplier `r`, plus `s` mod
/// 2^128.
///
/// Two implementations mirror the reference: the compatibility
/// [poly1305Mac] over exact [BigInt] integers (following the
/// ed25519 lane instead of the reference's u128 limb pairs), and
/// the fixed-memory [poly1305MacFixed] as a direct transliteration
/// of the five 26-bit-limb construction, cross-checked against
/// each other like the reference does.
library;

import 'dart:typed_data';

import 'package:coding_adventures_zeroize/zeroize.dart';

final BigInt _p1305 = (BigInt.one << 130) - BigInt.from(5);
final BigInt _mod128 = BigInt.one << 128;

const int _mask26 = (1 << 26) - 1;
const int _hibit = 1 << 24;

BigInt _loadLe(List<int> bytes) {
  var v = BigInt.zero;
  for (var i = bytes.length - 1; i >= 0; i--) {
    v = (v << 8) | BigInt.from(bytes[i]);
  }
  return v;
}

Uint8List _clampR(List<int> key) {
  final r = Uint8List.fromList(key.sublist(0, 16));
  r[3] &= 0x0f;
  r[7] &= 0x0f;
  r[11] &= 0x0f;
  r[15] &= 0x0f;
  r[4] &= 0xfc;
  r[8] &= 0xfc;
  r[12] &= 0xfc;
  return r;
}

void _checkKey(List<int> key) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
}

/// Computes the Poly1305 tag for [message] under the 32-byte
/// one-time [key] (`r || s` with clamped `r`).
///
/// Keys must never repeat across messages.
///
/// Throws [ArgumentError] unless [key] is exactly 32 bytes.
Uint8List poly1305Mac(List<int> message, List<int> key) {
  _checkKey(key);
  final rBytes = _clampR(key);
  final BigInt r;
  try {
    r = _loadLe(rBytes);
  } finally {
    zeroizeBytes(rBytes);
  }
  final s = _loadLe(key.sublist(16, 32));
  var acc = BigInt.zero;
  for (var offset = 0; offset < message.length; offset += 16) {
    final end = offset + 16 < message.length ? offset + 16 : message.length;
    final chunk = message.sublist(offset, end);
    final n = _loadLe(chunk) + (BigInt.one << (8 * chunk.length));
    acc = ((acc + n) * r) % _p1305;
  }
  final tag = (acc + s) % _mod128;
  final out = Uint8List(16);
  for (var i = 0; i < 16; i++) {
    out[i] = ((tag >> (8 * i)) & BigInt.from(0xff)).toInt();
  }
  return out;
}

// ─── 26-bit-limb fixed implementation ─────────────────────────────
//
// Direct transliteration of the reference's limb schedule. All
// values stay far below 2^63 (limbs < 2^27, five-term products <
// 2^56), so plain Dart arithmetic is exact; only the final
// borrow-select needs care (see below).

/// Wipe-on-drop limb workspace: clamped r limbs + s multiples,
/// accumulator, pad words, products, carry.
class _PolyState {
  final List<int> w = List<int>.filled(24, 0);
}

void _initState(_PolyState state, List<int> key) {
  int readU32(int offset) =>
      key[offset] |
      (key[offset + 1] << 8) |
      (key[offset + 2] << 16) |
      (key[offset + 3] << 24);
  state.w[0] = readU32(0) & 0x3ffffff;
  state.w[1] = ((readU32(3) >> 2) & 0x3ffff03);
  state.w[2] = ((readU32(6) >> 4) & 0x3ffc0ff);
  state.w[3] = ((readU32(9) >> 6) & 0x3f03fff);
  state.w[4] = ((readU32(12) >> 8) & 0x00fffff);
  state.w[5] = state.w[1] * 5;
  state.w[6] = state.w[2] * 5;
  state.w[7] = state.w[3] * 5;
  state.w[8] = state.w[4] * 5;
  state.w[14] = readU32(16);
  state.w[15] = readU32(20);
  state.w[16] = readU32(24);
  state.w[17] = readU32(28);
}

int _blockU32(List<int> block, int offset) =>
    block[offset] |
    (block[offset + 1] << 8) |
    (block[offset + 2] << 16) |
    (block[offset + 3] << 24);

void _processBlock(_PolyState state, List<int> block, int hibit) {
  final w = state.w;
  w[9] += _blockU32(block, 0) & _mask26;
  w[10] += (_blockU32(block, 3) >> 2) & _mask26;
  w[11] += (_blockU32(block, 6) >> 4) & _mask26;
  w[12] += (_blockU32(block, 9) >> 6) & _mask26;
  w[13] += (_blockU32(block, 12) >> 8) | hibit;

  w[18] =
      w[9] * w[0] + w[10] * w[8] + w[11] * w[7] + w[12] * w[6] + w[13] * w[5];
  w[19] =
      w[9] * w[1] + w[10] * w[0] + w[11] * w[8] + w[12] * w[7] + w[13] * w[6];
  w[20] =
      w[9] * w[2] + w[10] * w[1] + w[11] * w[0] + w[12] * w[8] + w[13] * w[7];
  w[21] =
      w[9] * w[3] + w[10] * w[2] + w[11] * w[1] + w[12] * w[0] + w[13] * w[8];
  w[22] =
      w[9] * w[4] + w[10] * w[3] + w[11] * w[2] + w[12] * w[1] + w[13] * w[0];

  w[23] = w[18] >> 26;
  w[9] = w[18] & _mask26;
  w[19] += w[23];
  w[23] = w[19] >> 26;
  w[10] = w[19] & _mask26;
  w[20] += w[23];
  w[23] = w[20] >> 26;
  w[11] = w[20] & _mask26;
  w[21] += w[23];
  w[23] = w[21] >> 26;
  w[12] = w[21] & _mask26;
  w[22] += w[23];
  w[23] = w[22] >> 26;
  w[13] = w[22] & _mask26;
  w[9] += w[23] * 5;
  w[23] = w[9] >> 26;
  w[9] &= _mask26;
  w[10] += w[23];
}

void _finalize(_PolyState state, Uint8List output) {
  final w = state.w;
  w[23] = w[10] >> 26;
  w[10] &= _mask26;
  w[11] += w[23];
  w[23] = w[11] >> 26;
  w[11] &= _mask26;
  w[12] += w[23];
  w[23] = w[12] >> 26;
  w[12] &= _mask26;
  w[13] += w[23];
  w[23] = w[13] >> 26;
  w[13] &= _mask26;
  w[9] += w[23] * 5;
  w[23] = w[9] >> 26;
  w[9] &= _mask26;
  w[10] += w[23];

  w[18] = w[9] + 5;
  w[23] = w[18] >> 26;
  w[18] &= _mask26;
  w[19] = w[10] + w[23];
  w[23] = w[19] >> 26;
  w[19] &= _mask26;
  w[20] = w[11] + w[23];
  w[23] = w[20] >> 26;
  w[20] &= _mask26;
  w[21] = w[12] + w[23];
  w[23] = w[21] >> 26;
  w[21] &= _mask26;
  // Borrow-select without a secret branch: (d4 >> 63) & 1 is 0/1 in
  // both u64-wrapping and signed Dart arithmetic, and subtracting 1
  // under a 64-bit mask yields all-ones/zero respectively.
  w[22] = w[13] + w[23] - (1 << 26);
  const mask64 = 0xffffffffffffffff;
  final selectReduced = (((w[22] >> 63) & 1) - 1) & mask64;
  final selectOriginal = mask64 ^ selectReduced;
  w[9] = (w[9] & selectOriginal) | (w[18] & selectReduced);
  w[10] = (w[10] & selectOriginal) | (w[19] & selectReduced);
  w[11] = (w[11] & selectOriginal) | (w[20] & selectReduced);
  w[12] = (w[12] & selectOriginal) | (w[21] & selectReduced);
  w[13] = (w[13] & selectOriginal) | (w[22] & selectReduced);

  w[18] = (((w[9] | (w[10] << 26)) & 0xffffffff) + w[14]) & 0xffffffffffffffff;
  w[19] =
      ((((w[10] >> 6) | (w[11] << 20)) & 0xffffffff) + w[15] + (w[18] >> 32)) &
          0xffffffffffffffff;
  w[20] =
      ((((w[11] >> 12) | (w[12] << 14)) & 0xffffffff) + w[16] + (w[19] >> 32)) &
          0xffffffffffffffff;
  w[21] =
      ((((w[12] >> 18) | (w[13] << 8)) & 0xffffffff) + w[17] + (w[20] >> 32)) &
          0xffffffffffffffff;
  for (var i = 0; i < 4; i++) {
    final word = w[18 + i] & 0xffffffff;
    output[i * 4] = word & 0xff;
    output[i * 4 + 1] = (word >> 8) & 0xff;
    output[i * 4 + 2] = (word >> 16) & 0xff;
    output[i * 4 + 3] = (word >> 24) & 0xff;
  }
  // Scrub the limb workspace on the way out (word lists are
  // mutable, unlike BigInts, so wiping is meaningful here).
  for (var i = 0; i < w.length; i++) {
    w[i] = 0;
  }
}

void _absorbBlocks(
  _PolyState state,
  ZeroizingBytes block,
  List<int> message,
  bool macTail,
) {
  var offset = 0;
  while (message.length - offset >= 16) {
    block.bytes.setRange(0, 16, message.sublist(offset, offset + 16));
    _processBlock(state, block.bytes, _hibit);
    offset += 16;
  }
  final remaining = message.length - offset;
  if (remaining == 0) {
    return;
  }
  zeroizeBytes(block.bytes);
  block.bytes.setRange(0, remaining, message.sublist(offset));
  if (macTail) {
    // Standalone MAC: explicit 0x01 byte, no high bit.
    block.bytes[remaining] = 1;
    _processBlock(state, block.bytes, 0);
  } else {
    // AEAD segments: zero padding, high bit set.
    _processBlock(state, block.bytes, _hibit);
  }
}

/// Computes a Poly1305 tag into caller-owned fixed [output] over the
/// 26-bit-limb schedule.
///
/// Throws [ArgumentError] unless [key] and [output] are exactly
/// 32/16 bytes.
void poly1305MacFixed(List<int> message, List<int> key, Uint8List output) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (output.length != 16) {
    throw ArgumentError('output must be exactly 16 bytes');
  }
  final state = _PolyState();
  final block = ZeroizingBytes(Uint8List(16));
  try {
    _initState(state, key);
    _absorbBlocks(state, block, message, true);
    _finalize(state, output);
  } finally {
    block.dispose();
  }
}

/// Fixed AEAD tag computation over AAD, ciphertext, and 64-bit
/// LITTLE-endian lengths (RFC 8439 §2.8 — unlike GCM's big-endian
/// length block). Public-in-source for the AEAD layer; hidden from
/// the barrel.
void poly1305AeadTagFixed(
  List<int> aad,
  List<int> ciphertext,
  BigInt aadLength,
  BigInt ciphertextLength,
  List<int> key,
  Uint8List output,
) {
  final state = _PolyState();
  final block = ZeroizingBytes(Uint8List(16));
  try {
    _initState(state, key);
    _absorbBlocks(state, block, aad, false);
    _absorbBlocks(state, block, ciphertext, false);
    zeroizeBytes(block.bytes);
    for (var i = 0; i < 8; i++) {
      block.bytes[i] = ((aadLength >> (8 * i)) & BigInt.from(0xff)).toInt();
      block.bytes[8 + i] =
          ((ciphertextLength >> (8 * i)) & BigInt.from(0xff)).toInt();
    }
    _processBlock(state, block.bytes, _hibit);
    _finalize(state, output);
  } finally {
    block.dispose();
  }
}
