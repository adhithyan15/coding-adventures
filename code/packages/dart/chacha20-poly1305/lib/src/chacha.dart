/// ChaCha20 stream cipher (RFC 8439 §2): ARX quarter-rounds over a
/// 4×4 u32 state (constants, key, counter + nonce), 20 rounds, with
/// feed-forward of the input state.
///
/// All word arithmetic is masked to 32 bits after every operation
/// (Dart ints are 64-bit; the reference relies on `wrapping_*`).
library;

import 'dart:typed_data';

import 'package:coding_adventures_zeroize/zeroize.dart';

import 'errors.dart';

/// "expand 32-byte k" nothing-up-my-sleeve constants.
const List<int> _constants = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

const int _mask32 = 0xffffffff;

int _rotl32(int x, int n) => (((x << n) | (x >> (32 - n))) & _mask32);

void _quarterRound(List<int> state, int a, int b, int c, int d) {
  state[a] = (state[a] + state[b]) & _mask32;
  state[d] ^= state[a];
  state[d] = _rotl32(state[d], 16);
  state[c] = (state[c] + state[d]) & _mask32;
  state[b] ^= state[c];
  state[b] = _rotl32(state[b], 12);
  state[a] = (state[a] + state[b]) & _mask32;
  state[d] ^= state[a];
  state[d] = _rotl32(state[d], 8);
  state[c] = (state[c] + state[d]) & _mask32;
  state[b] ^= state[c];
  state[b] = _rotl32(state[b], 7);
}

int _readU32(List<int> data, int offset) =>
    (data[offset] |
        (data[offset + 1] << 8) |
        (data[offset + 2] << 16) |
        (data[offset + 3] << 24)) &
    _mask32;

/// Runs the 20 ChaCha rounds (10 column/diagonal double-rounds) over
/// [state] in place.
void _runRounds(List<int> state) {
  for (var i = 0; i < 10; i++) {
    _quarterRound(state, 0, 4, 8, 12);
    _quarterRound(state, 1, 5, 9, 13);
    _quarterRound(state, 2, 6, 10, 14);
    _quarterRound(state, 3, 7, 11, 15);
    _quarterRound(state, 0, 5, 10, 15);
    _quarterRound(state, 1, 6, 11, 12);
    _quarterRound(state, 2, 7, 8, 13);
    _quarterRound(state, 3, 4, 9, 14);
  }
}

/// Generates one 64-byte keystream block into [output].
///
/// Public-in-source (hidden from the barrel) so the AEAD layer can
/// derive Poly1305 keys without an intermediate owner.
void chachaBlockInto(
  List<int> key,
  List<int> nonce,
  int counter,
  Uint8List output,
) {
  final state = List<int>.filled(16, 0);
  state[0] = _constants[0];
  state[1] = _constants[1];
  state[2] = _constants[2];
  state[3] = _constants[3];
  for (var i = 0; i < 8; i++) {
    state[4 + i] = _readU32(key, i * 4);
  }
  state[12] = counter & _mask32;
  state[13] = _readU32(nonce, 0);
  state[14] = _readU32(nonce, 4);
  state[15] = _readU32(nonce, 8);

  final original = List<int>.of(state);
  _runRounds(state);

  for (var i = 0; i < 16; i++) {
    final w = (state[i] + original[i]) & _mask32;
    output[i * 4] = w & 0xff;
    output[i * 4 + 1] = (w >> 8) & 0xff;
    output[i * 4 + 2] = (w >> 16) & 0xff;
    output[i * 4 + 3] = (w >> 24) & 0xff;
  }
  // Word temporaries are immutable values with no wipe equivalent
  // (see the sha384 lane); the byte keystream below is wiped by its
  // callers' owners.
}

Uint8List _chachaBlock(List<int> key, List<int> nonce, int counter) {
  final output = Uint8List(64);
  chachaBlockInto(key, nonce, counter, output);
  return output;
}

void _checkSizes(List<int> key, List<int> nonce) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (nonce.length != 12) {
    throw ArgumentError('nonce must be exactly 12 bytes');
  }
}

/// XORs [input] with the ChaCha20 keystream into caller-owned
/// [output], starting at [initialCounter].
///
/// Validates the exact output length and the complete 32-bit
/// counter range before changing [output]; keystream blocks live in
/// one reusable wipe-on-drop owner. Stream only: no authentication,
/// and key/nonce/counter positions must never repeat.
///
/// Throws [ChaCha20OutputLengthMismatch] and
/// [ChaCha20CounterExhausted], plus [ArgumentError] unless [key],
/// [nonce], and [initialCounter] fit their exact widths (the
/// reference takes fixed arrays and a u32, which admit no other
/// values).
void chacha20XorFixed(
  List<int> input,
  List<int> key,
  List<int> nonce,
  int initialCounter,
  Uint8List output,
) {
  _checkSizes(key, nonce);
  if (output.length != input.length) {
    throw const ChaCha20OutputLengthMismatch();
  }
  if (initialCounter < 0 || initialCounter > _mask32) {
    throw ArgumentError('counter must fit in 32 bits');
  }
  final counter = initialCounter;
  final blockCount = (input.length + 63) ~/ 64;
  // Available blocks from counter through u32::MAX inclusive.
  final available = 0x100000000 - counter;
  if (blockCount > available) {
    throw const ChaCha20CounterExhausted();
  }

  final keystream = ZeroizingBytes(Uint8List(64));
  try {
    var offset = 0;
    var current = counter;
    while (offset < input.length) {
      chachaBlockInto(key, nonce, current, keystream.bytes);
      final remaining = input.length - offset;
      final blockLength = remaining < 64 ? remaining : 64;
      for (var index = 0; index < blockLength; index++) {
        output[offset + index] = input[offset + index] ^ keystream.bytes[index];
      }
      offset += blockLength;
      if (offset < input.length) {
        current += 1;
      }
    }
  } finally {
    keystream.dispose();
  }
}

/// HChaCha20: derives a 32-byte subkey from ([key], 16-byte
/// [nonce16]) for XChaCha20.
///
/// Same 20 rounds over constants/key/nonce state, but with NO
/// feed-forward: the subkey is post-round rows 0 and 3 (the rows
/// not containing the key), which is what makes HChaCha20 a PRF.
///
/// Throws [ArgumentError] unless [key] and [nonce16] are exactly
/// 32/16 bytes.
Uint8List hchacha20Subkey(List<int> key, List<int> nonce16) {
  if (key.length != 32) {
    throw ArgumentError('key must be exactly 32 bytes');
  }
  if (nonce16.length != 16) {
    throw ArgumentError('nonce must be exactly 16 bytes');
  }
  final state = List<int>.filled(16, 0);
  state[0] = _constants[0];
  state[1] = _constants[1];
  state[2] = _constants[2];
  state[3] = _constants[3];
  for (var i = 0; i < 8; i++) {
    state[4 + i] = _readU32(key, i * 4);
  }
  for (var i = 0; i < 4; i++) {
    state[12 + i] = _readU32(nonce16, i * 4);
  }
  _runRounds(state);

  final subkey = Uint8List(32);
  for (var i = 0; i < 4; i++) {
    final lo = state[i], hi = state[12 + i];
    subkey[i * 4] = lo & 0xff;
    subkey[i * 4 + 1] = (lo >> 8) & 0xff;
    subkey[i * 4 + 2] = (lo >> 16) & 0xff;
    subkey[i * 4 + 3] = (lo >> 24) & 0xff;
    subkey[16 + i * 4] = hi & 0xff;
    subkey[16 + i * 4 + 1] = (hi >> 8) & 0xff;
    subkey[16 + i * 4 + 2] = (hi >> 16) & 0xff;
    subkey[16 + i * 4 + 3] = (hi >> 24) & 0xff;
  }
  return subkey;
}

/// ChaCha20 encryption (= decryption): XORs [plaintext] with the
/// keystream from [counter], 64 bytes per block.
///
/// Like the reference, the counter wraps mod 2^32 past the top
/// block instead of failing: callers streaming more than the
/// remaining counter space reuse keystream. Use [chacha20XorFixed]
/// when the request must fail instead of wrapping.
///
/// Throws [ArgumentError] unless [key], [nonce], and [counter] fit
/// their exact widths.
Uint8List chacha20Encrypt(
  List<int> plaintext,
  List<int> key,
  List<int> nonce,
  int counter,
) {
  _checkSizes(key, nonce);
  if (counter < 0 || counter > _mask32) {
    throw ArgumentError('counter must fit in 32 bits');
  }
  final output = Uint8List(plaintext.length);
  var offset = 0;
  var current = counter;
  while (offset < plaintext.length) {
    final block = ZeroizingBytes(_chachaBlock(key, nonce, current));
    try {
      current = (current + 1) & _mask32;
      final remaining = plaintext.length - offset;
      final take = remaining < 64 ? remaining : 64;
      for (var i = 0; i < take; i++) {
        output[offset + i] = plaintext[offset + i] ^ block.bytes[i];
      }
      offset += take;
    } finally {
      block.dispose();
    }
  }
  return output;
}
