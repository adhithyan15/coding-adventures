/// SHA-256 cryptographic hash function (FIPS 180-4), implemented from scratch.
///
/// SHA-256 is a member of the SHA-2 family published by NIST in 2001. It maps
/// any sequence of bytes to a fixed 32-byte (256-bit) digest. The same input
/// always yields the same digest; flipping a single input bit changes the
/// digest completely (the "avalanche effect"); and the digest cannot be
/// reversed to the input. It is the workhorse of TLS, git, Bitcoin, and code
/// signing.
///
/// ## Working in 32-bit arithmetic on a 64-bit VM
///
/// SHA-256 is defined over unsigned 32-bit words with wrap-around (mod 2³²)
/// arithmetic and rotations. Dart's `int` is 64-bit, so after every add,
/// shift, or rotate we mask with [_mask32] (`& 0xFFFFFFFF`) to stay inside 32
/// bits. Dart's `>>>` is a logical (zero-filling) right shift, which is exactly
/// what SHR and the rotations need.
library sha256;

import 'dart:typed_data';

// ─── Constants ───────────────────────────────────────────────────────────────

const int _mask32 = 0xFFFFFFFF;

/// Initial hash values: the first 32 bits of the fractional parts of the square
/// roots of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19). "Nothing up my
/// sleeve" numbers — their origin is transparent, proving no hidden backdoor.
const List<int> _init = [
  0x6A09E667, // sqrt(2)
  0xBB67AE85, // sqrt(3)
  0x3C6EF372, // sqrt(5)
  0xA54FF53A, // sqrt(7)
  0x510E527F, // sqrt(11)
  0x9B05688C, // sqrt(13)
  0x1F83D9AB, // sqrt(17)
  0x5BE0CD19, // sqrt(19)
];

/// Round constants: the first 32 bits of the fractional parts of the cube roots
/// of the first 64 primes (2, 3, 5, …, 311). 64 distinct constants give each of
/// the 64 rounds its own "flavour" of mixing.
const List<int> _k = [
  0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, //
  0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
  0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
  0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
  0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
  0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
  0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
  0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
  0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
  0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
  0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
  0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
  0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
  0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
  0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
  0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];

// ─── Auxiliary functions ─────────────────────────────────────────────────────
//
// Rotate a 32-bit word right by [n] bits: bits shifted off the bottom reappear
// at the top. Built from a zero-filling right shift OR a left shift, masked.

int _rotr(int x, int n) => ((x >>> n) | (x << (32 - n))) & _mask32;

/// Ch("choose"): where a bit of x is 1 take y, else take z. The `& _mask32`
/// clears the high bits that Dart's 64-bit `~x` sets above bit 31.
int _ch(int x, int y, int z) => ((x & y) ^ (~x & z)) & _mask32;

/// Maj("majority"): 1 where at least two of the three inputs are 1.
int _maj(int x, int y, int z) => (x & y) ^ (x & z) ^ (y & z);

int _bigSigma0(int x) => _rotr(x, 2) ^ _rotr(x, 13) ^ _rotr(x, 22);
int _bigSigma1(int x) => _rotr(x, 6) ^ _rotr(x, 11) ^ _rotr(x, 25);
int _smallSigma0(int x) => _rotr(x, 7) ^ _rotr(x, 18) ^ (x >>> 3);
int _smallSigma1(int x) => _rotr(x, 17) ^ _rotr(x, 19) ^ (x >>> 10);

// ─── Compression ─────────────────────────────────────────────────────────────
//
// Fold one 64-byte block into the eight-word [state]. First the 16 big-endian
// words of the block are expanded to a 64-word message schedule
//   W[t] = σ1(W[t-2]) + W[t-7] + σ0(W[t-15]) + W[t-16]
// then 64 rounds mix the schedule into the working variables, and finally the
// Davies–Meyer feed-forward adds the result back onto the input state.

Uint32List _compress(Uint32List state, Uint8List block, int offset) {
  final w = Uint32List(64);
  for (var i = 0; i < 16; i++) {
    final j = offset + i * 4;
    w[i] = (block[j] << 24) |
        (block[j + 1] << 16) |
        (block[j + 2] << 8) |
        block[j + 3];
  }
  for (var t = 16; t < 64; t++) {
    w[t] = (_smallSigma1(w[t - 2]) +
            w[t - 7] +
            _smallSigma0(w[t - 15]) +
            w[t - 16]) &
        _mask32;
  }

  var a = state[0],
      b = state[1],
      c = state[2],
      d = state[3],
      e = state[4],
      f = state[5],
      g = state[6],
      h = state[7];

  for (var t = 0; t < 64; t++) {
    final t1 = (h + _bigSigma1(e) + _ch(e, f, g) + _k[t] + w[t]) & _mask32;
    final t2 = (_bigSigma0(a) + _maj(a, b, c)) & _mask32;
    h = g;
    g = f;
    f = e;
    e = (d + t1) & _mask32;
    d = c;
    c = b;
    b = a;
    a = (t1 + t2) & _mask32;
  }

  final out = Uint32List(8);
  out[0] = (state[0] + a) & _mask32;
  out[1] = (state[1] + b) & _mask32;
  out[2] = (state[2] + c) & _mask32;
  out[3] = (state[3] + d) & _mask32;
  out[4] = (state[4] + e) & _mask32;
  out[5] = (state[5] + f) & _mask32;
  out[6] = (state[6] + g) & _mask32;
  out[7] = (state[7] + h) & _mask32;
  return out;
}

/// Serialise eight 32-bit state words into a big-endian 32-byte digest.
Uint8List _stateToDigest(Uint32List state) {
  final digest = Uint8List(32);
  for (var i = 0; i < 8; i++) {
    final word = state[i];
    digest[i * 4] = (word >>> 24) & 0xFF;
    digest[i * 4 + 1] = (word >>> 16) & 0xFF;
    digest[i * 4 + 2] = (word >>> 8) & 0xFF;
    digest[i * 4 + 3] = word & 0xFF;
  }
  return digest;
}

/// Build the padded tail for a message of [totalBytes] whose unprocessed
/// remainder is [buf]: append 0x80, then zeros until the length ≡ 56 (mod 64),
/// then the original bit length as a 64-bit big-endian integer (FIPS 180-4
/// §5.1.1).
Uint8List _padTail(List<int> buf, int totalBytes) {
  final bitLen = totalBytes * 8;
  final tail = <int>[...buf, 0x80];
  while (tail.length % 64 != 56) {
    tail.add(0x00);
  }
  for (var i = 7; i >= 0; i--) {
    tail.add((bitLen >>> (i * 8)) & 0xFF);
  }
  return Uint8List.fromList(tail);
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Compute the SHA-256 digest of [data] and return it as a 32-byte [Uint8List].
///
/// ```dart
/// sha256Hex(sha256(utf8.encode('abc'))); // 'ba7816bf…015ad'
/// ```
Uint8List sha256(List<int> data) {
  final padded = _padTail(data, data.length);
  var state = Uint32List.fromList(_init);
  for (var off = 0; off < padded.length; off += 64) {
    state = _compress(state, padded, off);
  }
  return _stateToDigest(state);
}

/// Compute SHA-256 and return the 64-character lowercase hex string.
///
/// ```dart
/// sha256Hex(utf8.encode('abc'));
/// // 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
/// ```
String sha256Hex(List<int> data) => _toHex(sha256(data));

String _toHex(Uint8List bytes) {
  final sb = StringBuffer();
  for (final b in bytes) {
    sb.write(b.toRadixString(16).padLeft(2, '0'));
  }
  return sb.toString();
}

/// A streaming SHA-256 hasher that accepts data in multiple [update] chunks and
/// produces the same digest as the one-shot [sha256] over the concatenation.
///
/// ```dart
/// final h = Sha256Hasher()..update(utf8.encode('ab'))..update(utf8.encode('c'));
/// h.hexDigest(); // == sha256Hex(utf8.encode('abc'))
/// ```
class Sha256Hasher {
  Uint32List _state;
  final List<int> _buf;
  int _byteCount;

  /// Create a new streaming hasher initialised with the SHA-256 constants.
  Sha256Hasher()
      : _state = Uint32List.fromList(_init),
        _buf = <int>[],
        _byteCount = 0;

  Sha256Hasher._(this._state, this._buf, this._byteCount);

  /// Feed more bytes into the hash. Complete 64-byte blocks are compressed
  /// immediately; a partial block is retained until enough bytes arrive.
  void update(List<int> data) {
    _byteCount += data.length;
    _buf.addAll(data);
    while (_buf.length >= 64) {
      final block = Uint8List.fromList(_buf.sublist(0, 64));
      _state = _compress(_state, block, 0);
      _buf.removeRange(0, 64);
    }
  }

  /// Return the 32-byte digest of all data fed so far.
  ///
  /// Non-destructive: the hasher's state is unchanged, so it can keep receiving
  /// [update]s afterwards.
  Uint8List digest() {
    final tail = _padTail(_buf, _byteCount);
    var state = Uint32List.fromList(_state);
    for (var off = 0; off < tail.length; off += 64) {
      state = _compress(state, tail, off);
    }
    return _stateToDigest(state);
  }

  /// Return the 64-character lowercase hex digest string.
  String hexDigest() => _toHex(digest());

  /// Return an independent copy of the current hasher state; hashing either the
  /// original or the copy afterwards does not affect the other.
  Sha256Hasher cloneHasher() =>
      Sha256Hasher._(Uint32List.fromList(_state), List<int>.of(_buf), _byteCount);
}
