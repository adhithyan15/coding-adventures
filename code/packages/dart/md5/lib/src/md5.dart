/// MD5 message-digest algorithm (RFC 1321), implemented from scratch in pure
/// Dart.
///
/// MD5 maps any byte sequence to a fixed 16-byte (128-bit) digest. The same
/// input always yields the same digest; a one-bit change avalanches through the
/// whole output; and the digest cannot be reversed to the input.
///
/// **Security note:** MD5 is broken for cryptographic purposes — practical
/// collision attacks exist — so it must not be used for signatures or password
/// storage. It remains useful as a fast, non-adversarial integrity checksum,
/// and implementing it teaches the Merkle–Damgård construction shared by SHA-1
/// and SHA-2.
///
/// ## Little-endian — the defining quirk
///
/// Unlike SHA-1/SHA-256, MD5 is **little-endian**: block words are read with the
/// *first* byte as the least-significant, the length is appended as a 64-bit
/// little-endian integer, and the digest words are emitted least-significant
/// byte first. Getting the byte order wrong yields a completely different (and
/// wrong) digest.
///
/// ## 32-bit arithmetic on a 64-bit VM
///
/// MD5 is defined over unsigned 32-bit words with wrap-around (mod 2³²)
/// arithmetic and rotations. Dart's `int` is 64-bit, so every add and rotate is
/// masked with [_mask32], and rotations use the logical (`>>>`) right shift.
library md5;

import 'dart:typed_data';

// ─── Constants ───────────────────────────────────────────────────────────────

const int _mask32 = 0xFFFFFFFF;

/// Initial state (A, B, C, D). In little-endian bytes these spell the counting
/// sequence 01 23 45 67 / 89 AB CD EF / … — transparently not a backdoor.
const List<int> _init = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476];

/// The 64 sine-derived round constants: `T[i] = floor(abs(sin(i+1)) × 2³²)`.
/// Hardcoded from RFC 1321 (Dart, like Rust `const fn`, can't call `sin` at
/// compile time).
const List<int> _t = [
  0xD76AA478, 0xE8C7B756, 0x242070DB, 0xC1BDCEEE, //
  0xF57C0FAF, 0x4787C62A, 0xA8304613, 0xFD469501,
  0x698098D8, 0x8B44F7AF, 0xFFFF5BB1, 0x895CD7BE,
  0x6B901122, 0xFD987193, 0xA679438E, 0x49B40821,
  0xF61E2562, 0xC040B340, 0x265E5A51, 0xE9B6C7AA,
  0xD62F105D, 0x02441453, 0xD8A1E681, 0xE7D3FBC8,
  0x21E1CDE6, 0xC33707D6, 0xF4D50D87, 0x455A14ED,
  0xA9E3E905, 0xFCEFA3F8, 0x676F02D9, 0x8D2A4C8A,
  0xFFFA3942, 0x8771F681, 0x6D9D6122, 0xFDE5380C,
  0xA4BEEA44, 0x4BDECFA9, 0xF6BB4B60, 0xBEBFBC70,
  0x289B7EC6, 0xEAA127FA, 0xD4EF3085, 0x04881D05,
  0xD9D4D039, 0xE6DB99E5, 0x1FA27CF8, 0xC4AC5665,
  0xF4292244, 0x432AFF97, 0xAB9423A7, 0xFC93A039,
  0x655B59C3, 0x8F0CCC92, 0xFFEFF47D, 0x85845DD1,
  0x6FA87E4F, 0xFE2CE6E0, 0xA3014314, 0x4E0811A1,
  0xF7537E82, 0xBD3AF235, 0x2AD7D2BB, 0xEB86D391,
];

/// Per-round left-rotation amounts, in four repeating groups of four.
const List<int> _s = [
  7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, //
  5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
  4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
  6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Rotate the 32-bit word [x] left by [s] bits.
int _rotl(int x, int s) => ((x << s) | (x >>> (32 - s))) & _mask32;

// ─── Compression ─────────────────────────────────────────────────────────────
//
// Fold one 64-byte block into the four-word [state] over 64 rounds. Four stages
// use different auxiliary functions and message-word schedules:
//   1 (FF): (b&c)|(~b&d)     g = i
//   2 (GG): (d&b)|(~d&c)     g = (5i+1) % 16
//   3 (HH): b^c^d            g = (3i+5) % 16
//   4 (II): c^(b|~d)         g = (7i)   % 16
// Each round: temp = b + ROTL(s[i], a + f + M[g] + T[i]); (a,b,c,d) ← (d,temp,b,c).

Uint32List _compress(Uint32List state, Uint8List block, int offset) {
  final m = Uint32List(16);
  for (var i = 0; i < 16; i++) {
    final j = offset + i * 4;
    // Little-endian: block[j] is the least-significant byte of word i.
    m[i] = block[j] |
        (block[j + 1] << 8) |
        (block[j + 2] << 16) |
        (block[j + 3] << 24);
  }

  var a = state[0], b = state[1], c = state[2], d = state[3];

  for (var i = 0; i < 64; i++) {
    int f, g;
    if (i < 16) {
      f = (b & c) | (~b & d);
      g = i;
    } else if (i < 32) {
      f = (d & b) | (~d & c);
      g = (5 * i + 1) % 16;
    } else if (i < 48) {
      f = b ^ c ^ d;
      g = (3 * i + 5) % 16;
    } else {
      f = c ^ (b | (~d & _mask32));
      g = (7 * i) % 16;
    }
    f &= _mask32; // clear high bits Dart's 64-bit ~ sets above bit 31
    final sum = (a + f + m[g] + _t[i]) & _mask32;
    final temp = (b + _rotl(sum, _s[i])) & _mask32;
    a = d;
    d = c;
    c = b;
    b = temp;
  }

  final out = Uint32List(4);
  out[0] = (state[0] + a) & _mask32;
  out[1] = (state[1] + b) & _mask32;
  out[2] = (state[2] + c) & _mask32;
  out[3] = (state[3] + d) & _mask32;
  return out;
}

/// Serialise the four state words into a 16-byte little-endian digest.
Uint8List _finalize(Uint32List state) {
  final digest = Uint8List(16);
  for (var i = 0; i < 4; i++) {
    final w = state[i];
    digest[i * 4] = w & 0xFF;
    digest[i * 4 + 1] = (w >>> 8) & 0xFF;
    digest[i * 4 + 2] = (w >>> 16) & 0xFF;
    digest[i * 4 + 3] = (w >>> 24) & 0xFF;
  }
  return digest;
}

/// Build the padded tail for a message of [totalBytes] whose unprocessed
/// remainder is [buf]: append 0x80, zeros until length ≡ 56 (mod 64), then the
/// original bit length as a **little-endian** 64-bit integer (RFC 1321 §3).
Uint8List _padTail(List<int> buf, int totalBytes) {
  final bitLen = totalBytes * 8;
  final tail = <int>[...buf, 0x80];
  while (tail.length % 64 != 56) {
    tail.add(0x00);
  }
  for (var i = 0; i < 8; i++) {
    tail.add((bitLen >>> (i * 8)) & 0xFF); // little-endian length
  }
  return Uint8List.fromList(tail);
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Compute the MD5 digest of [data] and return it as a 16-byte [Uint8List].
Uint8List sumMd5(List<int> data) {
  final padded = _padTail(data, data.length);
  var state = Uint32List.fromList(_init);
  for (var off = 0; off < padded.length; off += 64) {
    state = _compress(state, padded, off);
  }
  return _finalize(state);
}

/// Compute MD5 and return the 32-character lowercase hex string.
///
/// ```dart
/// hexString(utf8.encode('abc')); // '900150983cd24fb0d6963f7d28e17f72'
/// ```
String hexString(List<int> data) => _toHex(sumMd5(data));

String _toHex(Uint8List bytes) {
  final sb = StringBuffer();
  for (final b in bytes) {
    sb.write(b.toRadixString(16).padLeft(2, '0'));
  }
  return sb.toString();
}

/// A streaming MD5 hasher that accepts data in multiple [update] chunks and
/// produces the same digest as the one-shot [sumMd5] over the concatenation.
///
/// ```dart
/// final h = Md5Digest()..update(utf8.encode('ab'))..update(utf8.encode('c'));
/// h.hexDigest(); // == hexString(utf8.encode('abc'))
/// ```
class Md5Digest {
  Uint32List _state;
  final List<int> _buf;
  int _byteCount;

  /// Create a new streaming hasher initialised with the MD5 constants.
  Md5Digest()
      : _state = Uint32List.fromList(_init),
        _buf = <int>[],
        _byteCount = 0;

  Md5Digest._(this._state, this._buf, this._byteCount);

  /// Feed more bytes into the hash. Complete 64-byte blocks are compressed
  /// immediately; a partial block is retained until [sumMd5] is called.
  void update(List<int> data) {
    _byteCount += data.length;
    _buf.addAll(data);
    while (_buf.length >= 64) {
      final block = Uint8List.fromList(_buf.sublist(0, 64));
      _state = _compress(_state, block, 0);
      _buf.removeRange(0, 64);
    }
  }

  /// Return the 16-byte digest of all data fed so far. Non-destructive: the
  /// hasher can keep receiving [update]s afterwards.
  Uint8List sumMd5() {
    final tail = _padTail(_buf, _byteCount);
    var state = Uint32List.fromList(_state);
    for (var off = 0; off < tail.length; off += 64) {
      state = _compress(state, tail, off);
    }
    return _finalize(state);
  }

  /// Return the 32-character lowercase hex digest string.
  String hexDigest() => _toHex(sumMd5());

  /// Return an independent copy of the current hasher; hashing either the
  /// original or the copy afterwards does not affect the other.
  Md5Digest cloneDigest() =>
      Md5Digest._(Uint32List.fromList(_state), List<int>.of(_buf), _byteCount);
}
