/// BLAKE2b cryptographic hash function (RFC 7693), from scratch.
///
/// BLAKE2b is a modern hash faster than MD5 on 64-bit hardware and as
/// secure as SHA-3 against known attacks. Output 1..64 bytes, optional
/// key, salt, and personalization. Sequential mode only; tree hashing,
/// BLAKE2s, BLAKE2bp, BLAKE2sp, BLAKE2Xb, and BLAKE3 are out of scope
/// (see `code/specs/HF06-blake2b.md`).
///
/// Key invariant and classic BLAKE2 off-by-one: the *last real block*
/// is the one flagged final. For messages whose byte length is an exact
/// multiple of 128, we do NOT emit a spurious all-zero final block —
/// streaming flushes only when the buffer *strictly exceeds* 128 bytes,
/// so at [Blake2bHasher.digest] time there is always something in the
/// buffer to flag final (even if it's a full block).
///
/// ## 64-bit arithmetic without `u64`
///
/// Dart integers are signed 64-bit, so literals above `2^63 - 1` do not
/// compile and raw `+`/`>>` do not wrap like `u64`. The lane-private
/// [_U64] type stores high/low 32-bit halves (the same shape the
/// in-lane sha512 package uses) with wrapping add, XOR, rotate-right,
/// and little-endian conversion. The compression function then reads
/// as a direct transliteration of the reference.
library;

import 'dart:typed_data';

import 'constants.dart';
import 'errors.dart';

/// A 64-bit word as two unsigned 32-bit halves.
class _U64 {
  /// High 32 bits, always in `0..2^32`.
  final int hi;

  /// Low 32 bits, always in `0..2^32`.
  final int lo;

  /// Creates a word. Halves are masked to 32 bits.
  _U64(int hi, int lo)
      : hi = hi & 0xffffffff,
        lo = lo & 0xffffffff;

  /// Wrapping addition.
  _U64 operator +(_U64 other) {
    final sumLo = lo + other.lo;
    return _U64(hi + other.hi + (sumLo >> 32), sumLo);
  }

  /// Bitwise XOR.
  _U64 operator ^(_U64 other) => _U64(hi ^ other.hi, lo ^ other.lo);

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

  /// Reads eight little-endian bytes at [offset].
  factory _U64.fromLEBytes(List<int> bytes, int offset) => _U64(
        bytes[offset + 4] |
            (bytes[offset + 5] << 8) |
            (bytes[offset + 6] << 16) |
            (bytes[offset + 7] << 24),
        bytes[offset] |
            (bytes[offset + 1] << 8) |
            (bytes[offset + 2] << 16) |
            (bytes[offset + 3] << 24),
      );

  /// Writes this word as eight little-endian bytes at [offset].
  void writeLEBytes(List<int> out, int offset) {
    out[offset] = lo & 0xff;
    out[offset + 1] = (lo >>> 8) & 0xff;
    out[offset + 2] = (lo >>> 16) & 0xff;
    out[offset + 3] = (lo >>> 24) & 0xff;
    out[offset + 4] = hi & 0xff;
    out[offset + 5] = (hi >>> 8) & 0xff;
    out[offset + 6] = (hi >>> 16) & 0xff;
    out[offset + 7] = (hi >>> 24) & 0xff;
  }
}

/// BLAKE2b quarter-round G. Rotation constants (R1..R4) = (32, 24, 16,
/// 63). All adds wrap — the algorithm is defined over Z/2^64.
void _mix(List<_U64> v, int a, int b, int c, int d, _U64 x, _U64 y) {
  v[a] = v[a] + v[b] + x;
  v[d] = (v[d] ^ v[a]).rotR(32);
  v[c] = v[c] + v[d];
  v[b] = (v[b] ^ v[c]).rotR(24);
  v[a] = v[a] + v[b] + y;
  v[d] = (v[d] ^ v[a]).rotR(16);
  v[c] = v[c] + v[d];
  v[b] = (v[b] ^ v[c]).rotR(63);
}

/// Compression function F. [countLow]/[countHigh] are the 128-bit total
/// byte count so far (including the current block's bytes).
/// [isFinal] must be true iff this is the last compression call; that
/// triggers the v[14] inversion used to domain-separate the final block.
void _compress(
  List<_U64> h,
  List<int> block,
  _U64 countLow,
  _U64 countHigh,
  bool isFinal,
) {
  final m = List<_U64>.generate(16, (i) => _U64.fromLEBytes(block, i * 8));
  final v = List<_U64>.filled(16, _U64(0, 0));
  for (var i = 0; i < 8; i++) {
    v[i] = h[i];
  }
  for (var i = 0; i < 8; i++) {
    final (hi, lo) = blake2bIv[i];
    v[i + 8] = _U64(hi, lo);
  }

  v[12] = v[12] ^ countLow;
  v[13] = v[13] ^ countHigh;
  if (isFinal) {
    v[14] = v[14] ^ _U64(0xffffffff, 0xffffffff);
  }

  for (var i = 0; i < 12; i++) {
    final s = blake2bSigma[i % 10];
    _mix(v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
    _mix(v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
    _mix(v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
    _mix(v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
    _mix(v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
    _mix(v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
    _mix(v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
    _mix(v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
  }

  // Davies-Meyer feed-forward: XOR both halves of v into the state.
  for (var i = 0; i < 8; i++) {
    h[i] = h[i] ^ v[i] ^ v[i + 8];
  }
}

/// Builds the parameter-block-XOR-ed starting state. Sequential mode
/// only (fanout=1, depth=1).
List<_U64> _initialState(
  int digestSize,
  int keyLength,
  Uint8List salt,
  Uint8List personal,
) {
  final p = Uint8List(64);
  p[0] = digestSize;
  p[1] = keyLength;
  p[2] = 1; // fanout
  p[3] = 1; // depth
  for (var i = 0; i < salt.length; i++) {
    p[32 + i] = salt[i];
  }
  for (var i = 0; i < personal.length; i++) {
    p[48 + i] = personal[i];
  }

  return List<_U64>.generate(8, (i) {
    final (hi, lo) = blake2bIv[i];
    return _U64(hi, lo) ^ _U64.fromLEBytes(p, i * 8);
  });
}

/// BLAKE2b options. All fields are optional; defaults are a 64-byte
/// digest, unkeyed, no salt, no personalization.
class Blake2bOptions {
  /// Digest length in bytes (1..64). Defaults to 64.
  final int? digestSize;

  /// Key bytes (0..64). Defaults to unkeyed.
  final Uint8List? key;

  /// Salt bytes (empty or exactly 16). Defaults to none.
  final Uint8List? salt;

  /// Personalization bytes (empty or exactly 16). Defaults to none.
  final Uint8List? personal;

  /// Creates options with the given overrides.
  const Blake2bOptions({this.digestSize, this.key, this.salt, this.personal});
}

void _validate(
    int digestSize, Uint8List key, Uint8List salt, Uint8List personal) {
  if (digestSize < 1 || digestSize > blake2bMaxDigest) {
    throw InvalidDigestSize(digestSize);
  }
  if (key.length > blake2bMaxKey) {
    throw KeyTooLong(key.length);
  }
  if (salt.isNotEmpty && salt.length != blake2bParamBytes) {
    throw InvalidSaltLength(salt.length);
  }
  if (personal.isNotEmpty && personal.length != blake2bParamBytes) {
    throw InvalidPersonalLength(personal.length);
  }
}

String _bytesToHex(List<int> bytes) {
  final buffer = StringBuffer();
  for (final b in bytes) {
    buffer.write(b.toRadixString(16).padLeft(2, '0'));
  }
  return buffer.toString();
}

/// Streaming BLAKE2b hasher. [digest] is non-destructive; repeated
/// calls return the same bytes and the hasher remains usable for
/// further [update] calls.
class Blake2bHasher {
  List<_U64> _state;
  final List<int> _buffer = [];
  BigInt _byteCount = BigInt.zero;
  final int _digestSize;

  /// Constructs a hasher with the given [options].
  ///
  /// Throws [Blake2bError] when an option fails validation.
  factory Blake2bHasher([Blake2bOptions? options]) {
    final opts = options ?? const Blake2bOptions();
    final digestSize = opts.digestSize ?? blake2bMaxDigest;
    final key = opts.key ?? Uint8List(0);
    final salt = opts.salt ?? Uint8List(0);
    final personal = opts.personal ?? Uint8List(0);
    _validate(digestSize, key, salt, personal);

    final state = _initialState(digestSize, key.length, salt, personal);

    // Keyed mode: prepend the key zero-padded to one full block.
    final buffer = <int>[];
    if (key.isNotEmpty) {
      buffer.addAll(List<int>.filled(blake2bBlockSize, 0));
      for (var i = 0; i < key.length; i++) {
        buffer[i] = key[i];
      }
    }

    return Blake2bHasher._(state, buffer, digestSize);
  }

  Blake2bHasher._(this._state, List<int> buffer, this._digestSize) {
    _buffer.addAll(buffer);
  }

  /// Appends bytes to the message. Flushes full blocks except the
  /// latest: we always keep at least one byte in the buffer until
  /// [digest] so the final compression is the one flagged final.
  ///
  /// Returns `this` for chaining (mirrors the reference's `&mut Self`
  /// return).
  Blake2bHasher update(List<int> data) {
    _buffer.addAll(data);
    while (_buffer.length > blake2bBlockSize) {
      _byteCount += BigInt.from(blake2bBlockSize);
      final block = _buffer.sublist(0, blake2bBlockSize);
      _compress(_state, block, _counterLow(), _counterHigh(), false);
      _buffer.removeRange(0, blake2bBlockSize);
    }
    return this;
  }

  /// Finalizes and returns the digest. Non-destructive: the hasher
  /// keeps its state, so [digest] can be called again or followed by
  /// more [update] calls.
  Uint8List digest() {
    final state = List<_U64>.of(_state);
    final finalBlock = List<int>.filled(blake2bBlockSize, 0);
    for (var i = 0; i < _buffer.length; i++) {
      finalBlock[i] = _buffer[i];
    }
    final total = _byteCount + BigInt.from(_buffer.length);
    _compress(state, finalBlock, _u64Of(total), _u64Of(total >> 64), true);

    final out = Uint8List(64);
    for (var i = 0; i < 8; i++) {
      state[i].writeLEBytes(out, i * 8);
    }
    return Uint8List.fromList(out.sublist(0, _digestSize));
  }

  /// Finalizes and returns the digest as lowercase hex.
  String hexDigest() => _bytesToHex(digest());

  /// Returns an independent clone at this point in the stream.
  Blake2bHasher copy() {
    final clone = Blake2bHasher._(
      List<_U64>.of(_state),
      List<int>.of(_buffer),
      _digestSize,
    );
    clone._byteCount = _byteCount;
    return clone;
  }

  _U64 _counterLow() => _u64Of(_byteCount);
  _U64 _counterHigh() => _u64Of(_byteCount >> 64);
}

_U64 _u64Of(BigInt value) {
  final masked = value & ((BigInt.one << 64) - BigInt.one);
  return _U64(
    ((masked >> 32) & BigInt.from(0xffffffff)).toInt(),
    (masked & BigInt.from(0xffffffff)).toInt(),
  );
}

/// One-shot BLAKE2b. Returns raw bytes of length `digestSize`.
Uint8List blake2b(List<int> data, [Blake2bOptions? options]) =>
    Blake2bHasher(options).update(data).digest();

/// One-shot BLAKE2b returning lowercase hex.
String blake2bHex(List<int> data, [Blake2bOptions? options]) =>
    _bytesToHex(blake2b(data, options));
