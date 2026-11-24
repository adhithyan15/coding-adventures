/// Salsa20/8 core for scrypt's BlockMix (RFC 7914 §3).
///
/// Eight rounds (four double-rounds) of the Salsa quarter-round over
/// a 64-byte block, decoded as sixteen little-endian u32 words with
/// wrapping arithmetic, plus feed-forward of the input words.
///
/// Public-in-source (not barrel-exported) so tests can pin the
/// structural properties the reference tests pin, following the
/// x25519 lane's field library.
library;

import 'dart:typed_data';

int _rotl32(int x, int n) => (((x << n) | (x >> (32 - n))) & 0xffffffff);

void _quarterRound(List<int> x, int a, int b, int c, int d) {
  x[b] = (x[b] ^ _rotl32((x[a] + x[d]) & 0xffffffff, 7)) & 0xffffffff;
  x[c] = (x[c] ^ _rotl32((x[b] + x[a]) & 0xffffffff, 9)) & 0xffffffff;
  x[d] = (x[d] ^ _rotl32((x[c] + x[b]) & 0xffffffff, 13)) & 0xffffffff;
  x[a] = (x[a] ^ _rotl32((x[d] + x[c]) & 0xffffffff, 18)) & 0xffffffff;
}

/// Applies Salsa20/8 to a 64-byte block, returning a new block.
///
/// Throws [ArgumentError] unless [input] is exactly 64 bytes.
Uint8List salsa208(List<int> input) {
  if (input.length != 64) {
    throw ArgumentError('salsa20/8 input must be exactly 64 bytes');
  }
  final x = List<int>.generate(16, (i) {
    final o = i * 4;
    return (input[o] |
            (input[o + 1] << 8) |
            (input[o + 2] << 16) |
            (input[o + 3] << 24)) &
        0xffffffff;
  });
  final z = List<int>.of(x);

  for (var round = 0; round < 4; round++) {
    _quarterRound(x, 0, 4, 8, 12);
    _quarterRound(x, 5, 9, 13, 1);
    _quarterRound(x, 10, 14, 2, 6);
    _quarterRound(x, 15, 3, 7, 11);
    _quarterRound(x, 0, 1, 2, 3);
    _quarterRound(x, 5, 6, 7, 4);
    _quarterRound(x, 10, 11, 8, 9);
    _quarterRound(x, 15, 12, 13, 14);
  }

  final out = Uint8List(64);
  for (var i = 0; i < 16; i++) {
    final w = (x[i] + z[i]) & 0xffffffff;
    out[i * 4] = w & 0xff;
    out[i * 4 + 1] = (w >> 8) & 0xff;
    out[i * 4 + 2] = (w >> 16) & 0xff;
    out[i * 4 + 3] = (w >> 24) & 0xff;
  }
  return out;
}
