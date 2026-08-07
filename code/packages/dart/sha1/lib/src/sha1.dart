/// SHA-1 cryptographic hash function (FIPS 180-4), implemented from scratch in
/// pure Dart.
///
/// SHA-1 maps any byte sequence to a fixed 20-byte (160-bit) digest. Like MD5
/// and SHA-256 it uses the Merkle–Damgård construction over 64-byte blocks, but
/// with five state words and 80 rounds.
///
/// **Security note:** SHA-1 is **broken** for collision resistance (the
/// SHAttered attack, 2017) — do not use it for signatures or certificates. It
/// remains in legacy protocols and as a non-adversarial checksum (e.g. git
/// object names), and implementing it illuminates the family shared with
/// SHA-256.
///
/// ## 32-bit arithmetic on a 64-bit VM
///
/// SHA-1 is defined over unsigned 32-bit words with wrap-around (mod 2³²)
/// arithmetic and rotations. Dart's `int` is 64-bit, so every add and rotate is
/// masked with [_mask32], and shifts use the logical (`>>>`) right shift.
library sha1;

import 'dart:typed_data';

const int _mask32 = 0xFFFFFFFF;

/// Initial state H₀..H₄ ("nothing up my sleeve" counting-sequence constants).
const List<int> _init = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

/// Per-stage round constants: floor(sqrt(2,3,5,10) × 2³⁰).
const List<int> _k = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];

/// Rotate the 32-bit word [x] left by [n] bits.
int _rotl(int x, int n) => ((x << n) | (x >>> (32 - n))) & _mask32;

/// Fold one 64-byte block into the five-word [state] over 80 rounds.
Uint32List _compress(Uint32List state, Uint8List block, int offset) {
  // Message schedule: 16 big-endian words expanded to 80.
  final w = Uint32List(80);
  for (var i = 0; i < 16; i++) {
    final j = offset + i * 4;
    w[i] = (block[j] << 24) |
        (block[j + 1] << 16) |
        (block[j + 2] << 8) |
        block[j + 3];
  }
  for (var i = 16; i < 80; i++) {
    w[i] = _rotl(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
  }

  var a = state[0], b = state[1], c = state[2], d = state[3], e = state[4];

  for (var t = 0; t < 80; t++) {
    int f, k;
    if (t < 20) {
      f = (b & c) | (~b & d);
      k = _k[0];
    } else if (t < 40) {
      f = b ^ c ^ d;
      k = _k[1];
    } else if (t < 60) {
      f = (b & c) | (b & d) | (c & d);
      k = _k[2];
    } else {
      f = b ^ c ^ d;
      k = _k[3];
    }
    f &= _mask32; // clear high bits Dart's 64-bit ~ sets above bit 31
    final temp = (_rotl(a, 5) + f + e + k + w[t]) & _mask32;
    e = d;
    d = c;
    c = _rotl(b, 30);
    b = a;
    a = temp;
  }

  final out = Uint32List(5);
  out[0] = (state[0] + a) & _mask32;
  out[1] = (state[1] + b) & _mask32;
  out[2] = (state[2] + c) & _mask32;
  out[3] = (state[3] + d) & _mask32;
  out[4] = (state[4] + e) & _mask32;
  return out;
}

/// Serialise the five state words into a 20-byte big-endian digest.
Uint8List _finalize(Uint32List state) {
  final digest = Uint8List(20);
  for (var i = 0; i < 5; i++) {
    final w = state[i];
    digest[i * 4] = (w >>> 24) & 0xFF;
    digest[i * 4 + 1] = (w >>> 16) & 0xFF;
    digest[i * 4 + 2] = (w >>> 8) & 0xFF;
    digest[i * 4 + 3] = w & 0xFF;
  }
  return digest;
}

/// Build the padded tail: append 0x80, zeros until length ≡ 56 (mod 64), then
/// the original bit length as a 64-bit **big-endian** integer (FIPS 180-4).
Uint8List _padTail(List<int> buf, int totalBytes) {
  final bitLen = totalBytes * 8;
  final tail = <int>[...buf, 0x80];
  while (tail.length % 64 != 56) {
    tail.add(0x00);
  }
  for (var i = 7; i >= 0; i--) {
    tail.add((bitLen >>> (i * 8)) & 0xFF); // big-endian length
  }
  return Uint8List.fromList(tail);
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Compute the SHA-1 digest of [data] and return it as a 20-byte [Uint8List].
///
/// Named `sum1` to mirror the reference crate (avoiding a clash with any `sum`).
Uint8List sum1(List<int> data) {
  final padded = _padTail(data, data.length);
  var state = Uint32List.fromList(_init);
  for (var off = 0; off < padded.length; off += 64) {
    state = _compress(state, padded, off);
  }
  return _finalize(state);
}

/// Compute SHA-1 and return the 40-character lowercase hex string.
///
/// ```dart
/// hexString(utf8.encode('abc')); // 'a9993e364706816aba3e25717850c26c9cd0d89d'
/// ```
String hexString(List<int> data) => _toHex(sum1(data));

String _toHex(Uint8List bytes) {
  final sb = StringBuffer();
  for (final b in bytes) {
    sb.write(b.toRadixString(16).padLeft(2, '0'));
  }
  return sb.toString();
}

/// A streaming SHA-1 hasher that accepts data in multiple [update] chunks and
/// produces the same digest as the one-shot [sum1] over the concatenation.
class Sha1Digest {
  Uint32List _state;
  final List<int> _buf;
  int _byteCount;

  /// Create a new streaming hasher initialised with the SHA-1 constants.
  Sha1Digest()
      : _state = Uint32List.fromList(_init),
        _buf = <int>[],
        _byteCount = 0;

  Sha1Digest._(this._state, this._buf, this._byteCount);

  /// Feed more bytes into the hash. Complete 64-byte blocks are compressed
  /// immediately; a partial block is retained until [sum1] is called.
  void update(List<int> data) {
    _byteCount += data.length;
    _buf.addAll(data);
    while (_buf.length >= 64) {
      final block = Uint8List.fromList(_buf.sublist(0, 64));
      _state = _compress(_state, block, 0);
      _buf.removeRange(0, 64);
    }
  }

  /// Return the 20-byte digest of all data fed so far. Non-destructive.
  Uint8List sum1() {
    final tail = _padTail(_buf, _byteCount);
    var state = Uint32List.fromList(_state);
    for (var off = 0; off < tail.length; off += 64) {
      state = _compress(state, tail, off);
    }
    return _finalize(state);
  }

  /// Return the 40-character lowercase hex digest string.
  String hexDigest() => _toHex(sum1());

  /// Return an independent copy of the current hasher.
  Sha1Digest cloneDigest() =>
      Sha1Digest._(Uint32List.fromList(_state), List<int>.of(_buf), _byteCount);
}
