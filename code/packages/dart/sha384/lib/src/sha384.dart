/// Fixed-memory SHA-384 from FIPS 180-4.
///
/// SHA-384 uses the SHA-512-family 1024-bit block and 80-round
/// compression function with a distinct initial state, then returns
/// the leftmost 384 bits of the final state.
///
/// ## 64-bit arithmetic without `u64`
///
/// Dart integers are signed 64-bit, so literals above 2^63-1 do not
/// compile and raw shifts do not wrap like `u64`. The lane-private
/// [_U64] type stores high/low 32-bit halves (the same shape the
/// in-lane sha512 and blake2b packages use) with wrapping add, AND,
/// NOT, XOR, rotate-right, and logical shift. The compression function
/// is then a direct transliteration of the reference.
///
/// ## Memory hygiene
///
/// The reference scrubs chaining state, buffers, counters, and round
/// temporaries via `Drop` and explicit wipes. This port uses the
/// in-lane zeroize package for every buffer it owns, and [Sha384Hasher]
/// offers an explicit [Sha384Hasher.dispose] (Dart has no
/// destructors). Word temporaries are immutable values with no wipe
/// equivalent — see the zeroize lane docs for the exact platform
/// limits, which apply here unchanged.
library;

import 'dart:typed_data';

import 'package:coding_adventures_zeroize/zeroize.dart';

import 'constants.dart';

/// A 64-bit word as two unsigned 32-bit halves.
class _U64 {
  /// High and low halves, always in `0..2^32`. Mutable so [Sha384Hasher.dispose] can scrub live state.
  int hi;
  int lo;

  /// Creates a word, masking halves to 32 bits.
  _U64(int hi, int lo)
      : hi = hi & 0xffffffff,
        lo = lo & 0xffffffff;

  /// Wrapping addition.
  _U64 operator +(_U64 other) {
    final sumLo = lo + other.lo;
    return _U64(hi + other.hi + (sumLo >> 32), sumLo);
  }

  /// Bitwise AND.
  _U64 operator &(_U64 other) => _U64(hi & other.hi, lo & other.lo);

  /// Bitwise XOR.
  _U64 operator ^(_U64 other) => _U64(hi ^ other.hi, lo ^ other.lo);

  /// Bitwise NOT.
  _U64 operator ~() => _U64(~hi, ~lo);

  /// Logical right shift by [n] bits. Requires `0 < n < 32` (the only
  /// call sites shift by 6 and 7); larger shifts have no meaning here.
  _U64 shr(int n) {
    assert(n > 0 && n < 32, 'shr supports shifts 1..31');
    return _U64(
      (hi >>> n) & 0xffffffff,
      (((lo >>> n) | (hi << (32 - n))) & 0xffffffff),
    );
  }

  /// Rotate right by [n] bits.
  _U64 rotR(int n) {
    n &= 63;
    if (n == 0) return _U64(hi, lo);
    if (n == 32) return _U64(lo, hi);
    if (n > 32) return _U64(lo, hi).rotR(n - 32);
    return _U64(
      (hi >>> n) | (lo << (32 - n)),
      (lo >>> n) | (hi << (32 - n)),
    );
  }

  /// Reads eight big-endian bytes at [offset].
  factory _U64.fromBEBytes(List<int> bytes, int offset) => _U64(
        (bytes[offset] << 24) |
            (bytes[offset + 1] << 16) |
            (bytes[offset + 2] << 8) |
            bytes[offset + 3],
        (bytes[offset + 4] << 24) |
            (bytes[offset + 5] << 16) |
            (bytes[offset + 6] << 8) |
            bytes[offset + 7],
      );

  /// Writes this word as eight big-endian bytes at [offset].
  void writeBEBytes(List<int> out, int offset) {
    out[offset] = (hi >>> 24) & 0xff;
    out[offset + 1] = (hi >>> 16) & 0xff;
    out[offset + 2] = (hi >>> 8) & 0xff;
    out[offset + 3] = hi & 0xff;
    out[offset + 4] = (lo >>> 24) & 0xff;
    out[offset + 5] = (lo >>> 16) & 0xff;
    out[offset + 6] = (lo >>> 8) & 0xff;
    out[offset + 7] = lo & 0xff;
  }

  /// Overwrites both halves with zero.
  void wipe() {
    hi = 0;
    lo = 0;
  }
}

_U64 _choice(_U64 x, _U64 y, _U64 z) => (x & y) ^ ((~x) & z);

_U64 _majority(_U64 x, _U64 y, _U64 z) => (x & y) ^ (x & z) ^ (y & z);

_U64 _bigSigma0(_U64 v) => v.rotR(28) ^ v.rotR(34) ^ v.rotR(39);

_U64 _bigSigma1(_U64 v) => v.rotR(14) ^ v.rotR(18) ^ v.rotR(41);

_U64 _smallSigma0(_U64 v) => v.rotR(1) ^ v.rotR(8) ^ v.shr(7);

_U64 _smallSigma1(_U64 v) => v.rotR(19) ^ v.rotR(61) ^ v.shr(6);

void _compress(List<_U64> state, Uint8List block) {
  final schedule = List<_U64>.generate(80, (_) => _U64(0, 0));
  for (var index = 0; index < 16; index++) {
    schedule[index] = _U64.fromBEBytes(block, index * 8);
  }
  for (var index = 16; index < 80; index++) {
    schedule[index] = _smallSigma1(schedule[index - 2]) +
        schedule[index - 7] +
        _smallSigma0(schedule[index - 15]) +
        schedule[index - 16];
  }

  var a = state[0];
  var b = state[1];
  var c = state[2];
  var d = state[3];
  var e = state[4];
  var f = state[5];
  var g = state[6];
  var h = state[7];

  for (var index = 0; index < 80; index++) {
    final k = sha384RoundConstants[index];
    final temporary1 = h +
        _bigSigma1(e) +
        _choice(e, f, g) +
        _U64(k.$1, k.$2) +
        schedule[index];
    final temporary2 = _bigSigma0(a) + _majority(a, b, c);
    h = g;
    g = f;
    f = e;
    e = d + temporary1;
    d = c;
    c = b;
    b = a;
    a = temporary1 + temporary2;
    temporary1.wipe();
    temporary2.wipe();
  }

  // Feed-forward: compute the sums, wipe the old chaining words
  // (they would otherwise survive as garbage holding prior state),
  // then install the sums. Safe: a..h are always fresh objects by
  // round end, never aliases of state[i].
  final sums = [
    state[0] + a,
    state[1] + b,
    state[2] + c,
    state[3] + d,
    state[4] + e,
    state[5] + f,
    state[6] + g,
    state[7] + h,
  ];
  for (final word in state) {
    word.wipe();
  }
  for (var i = 0; i < 8; i++) {
    state[i] = sums[i];
  }

  for (final word in schedule) {
    word.wipe();
  }
  for (final word in [a, b, c, d, e, f, g, h]) {
    word.wipe();
  }
}

/// Stable, payload-free failure from streaming SHA-384: the message's
/// bit length would not fit the FIPS 180-4 128-bit field.
class Sha384MessageTooLong implements Exception {
  /// Creates the error.
  const Sha384MessageTooLong();

  @override
  bool operator ==(Object other) => other is Sha384MessageTooLong;

  @override
  int get hashCode => 0;

  @override
  String toString() => 'SHA-384 message is too long';
}

/// Compute one SHA-384 digest.
Uint8List sha384(List<int> data) {
  final hasher = Sha384Hasher();
  hasher.update(data);
  final out = hasher.digest();
  hasher.dispose();
  return out;
}

/// Compute one SHA-384 digest as lowercase hex.
String sha384Hex(List<int> data) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in sha384(data)) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}

/// Incremental SHA-384 state.
///
/// Owns one partial block and the chaining state. Buffers are wiped
/// when consumed; call [dispose] to scrub all retained state (Dart has
/// no destructors, so — unlike the reference's `Drop` — this never
/// runs automatically). Using a hasher after [dispose] throws
/// [StateError]: hashing with scrubbed state would silently produce
/// wrong digests, so the failure is loud.
class Sha384Hasher {
  final List<_U64> _state;
  final Uint8List _buffer = Uint8List(sha384BlockLength);
  int _bufferLen = 0;
  BigInt _byteCount = BigInt.zero;
  bool _disposed = false;

  /// Constructs an empty SHA-384 state.
  Sha384Hasher()
      : _state = [
          for (final pair in sha384InitialState) _U64(pair.$1, pair.$2),
        ];

  /// Absorbs more message bytes.
  ///
  /// The total encoded length is checked before any retained state
  /// changes, so a failure leaves the hasher untouched.
  ///
  /// Throws [Sha384MessageTooLong] if the resulting bit length would
  /// not fit the standard's 128-bit length field, and [StateError] if
  /// the hasher was disposed.
  void update(List<int> data) {
    _checkLive();
    final nextCount = _byteCount + BigInt.from(data.length);
    if (nextCount > sha384MaxMessageBytes) {
      throw const Sha384MessageTooLong();
    }

    var offset = 0;
    if (_bufferLen != 0) {
      final needed = sha384BlockLength - _bufferLen;
      final copied =
          needed < data.length - offset ? needed : data.length - offset;
      _buffer.setRange(_bufferLen, _bufferLen + copied, data, offset);
      _bufferLen += copied;
      offset += copied;

      if (_bufferLen != sha384BlockLength) {
        _byteCount = nextCount;
        return;
      }

      _compress(_state, _buffer);
      zeroizeBytes(_buffer);
      _bufferLen = 0;
    }

    while (data.length - offset >= sha384BlockLength) {
      // The transient copy is scrubbed after use: unlike the reference
      // (which compresses over borrowed slices), copies would otherwise
      // leave dead plaintext in the heap until GC. setRange copies
      // straight from the source with no intermediate list.
      final block = Uint8List(sha384BlockLength);
      block.setRange(0, sha384BlockLength, data, offset);
      _compress(_state, block);
      zeroizeBytes(block);
      offset += sha384BlockLength;
    }

    final rest = data.length - offset;
    if (rest > 0) {
      _buffer.setRange(0, rest, data, offset);
    }
    _bufferLen = rest;
    _byteCount = nextCount;
  }

  /// Returns a non-destructive 48-byte digest snapshot.
  ///
  /// Throws [StateError] if the hasher was disposed.
  Uint8List digest() {
    _checkLive();
    final state = [for (final w in _state) _U64(w.hi, w.lo)];
    final tail = Uint8List(sha384BlockLength * 2);
    tail.setRange(0, _bufferLen, _buffer);
    tail[_bufferLen] = 0x80;

    final tailLen = _bufferLen <= sha384SingleBlockLimit
        ? sha384BlockLength
        : sha384BlockLength * 2;
    final bitLength = _byteCount * BigInt.from(8);
    for (var i = 0; i < sha384LengthFieldLength; i++) {
      tail[tailLen - sha384LengthFieldLength + i] =
          ((bitLength >> (8 * (15 - i))) & BigInt.from(0xff)).toInt();
    }

    for (var off = 0; off < tailLen; off += sha384BlockLength) {
      _compress(
          state, Uint8List.sublistView(tail, off, off + sha384BlockLength));
    }

    final out = Uint8List(sha384DigestLength);
    for (var index = 0; index < 6; index++) {
      state[index].writeBEBytes(out, index * 8);
    }

    for (final word in state) {
      word.wipe();
    }
    zeroizeBytes(tail);
    return out;
  }

  /// Scrubs all retained state. Using the hasher afterwards throws
  /// [StateError]. Idempotent.
  void dispose() {
    for (final word in _state) {
      word.wipe();
    }
    zeroizeBytes(_buffer);
    _bufferLen = 0;
    _byteCount = BigInt.zero;
    _disposed = true;
  }

  /// Whether [dispose] has run.
  bool get isDisposed => _disposed;

  void _checkLive() {
    if (_disposed) {
      throw StateError('Sha384Hasher used after dispose()');
    }
  }
}

/// Test-only helper: sets the byte counter, mirroring the in-lane
/// sha512 package's counter helper. There is no other way to approach
/// the 2^127-byte limit.
void setSha384ByteCounterForTesting(Sha384Hasher hasher, BigInt value) {
  hasher._byteCount = value;
}
