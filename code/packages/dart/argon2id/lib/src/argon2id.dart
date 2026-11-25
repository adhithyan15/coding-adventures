/// Argon2id hybrid memory-hard password hashing (RFC 9106).
///
/// Argon2id is the hybrid member of the Argon2 family: the first half
/// of the first pass uses side-channel-resistant data-independent
/// addressing (Argon2i), and everything afterwards uses GPU/ASIC-
/// resistant data-dependent addressing (Argon2d).
///
/// ## 64-bit arithmetic without `u64`
///
/// The reference stores block words as `u64` with wrapping add/mul and
/// rotates. Dart integers are signed 64-bit, and the compression
/// function's `2 * a * b` term overflows 64 bits by design — so block
/// words are exact [BigInt] integers masked to 64 bits after every
/// operation. The observable behavior (tags, vectors, errors) is
/// identical; only the representation differs.
library;

import 'dart:typed_data';

import 'package:coding_adventures_blake2b/blake2b.dart' hide KeyTooLong;

import 'errors.dart';

/// Argon2 version 1.3.
const int argon2Version13 = 0x13;

/// Argon2id type identifier.
const int _typeId = 2;

/// Block size in bytes; blocks hold 128 u64 words.
const int _blockSize = 1024;
const int _blockWords = 128;

/// Sync points (slices) per pass.
const int _syncPoints = 4;

/// Addresses produced per address block.
const int _addressesPerBlock = 128;

/// 64-bit mask and 32-bit mask for word arithmetic.
final BigInt _mask64 = (BigInt.one << 64) - BigInt.one;
final BigInt _mask32 = (BigInt.one << 32) - BigInt.one;

/// Optional Argon2id inputs: secret key, associated data, and version
/// override. Absent fields take the defaults (no key, no AD, v1.3).
class Argon2Options {
  /// Secret key bytes (any length fitting in 32 bits when encoded).
  /// Any [List] works; only the length is validated before hashing.
  final List<int>? key;

  /// Associated data bytes (same bound).
  final List<int>? associatedData;

  /// Version override; only 0x13 is accepted.
  final int? version;

  /// Creates options with the given overrides.
  const Argon2Options({this.key, this.associatedData, this.version});
}

BigInt _add64(BigInt a, BigInt b) => (a + b) & _mask64;
BigInt _mul64(BigInt a, BigInt b) => (a * b) & _mask64;

BigInt _rotr64(BigInt x, int n) {
  n &= 63;
  if (n == 0) return x & _mask64;
  return (((x >> n) | (x << (64 - n))) & _mask64);
}

void _g(List<BigInt> v, int a, int b, int c, int d) {
  var va = v[a];
  var vb = v[b];
  var vc = v[c];
  var vd = v[d];
  va = _add64(
      _add64(va, vb), _mul64(BigInt.two, _mul64(va & _mask32, vb & _mask32)));
  vd = _rotr64(vd ^ va, 32);
  vc = _add64(
      _add64(vc, vd), _mul64(BigInt.two, _mul64(vc & _mask32, vd & _mask32)));
  vb = _rotr64(vb ^ vc, 24);
  va = _add64(
      _add64(va, vb), _mul64(BigInt.two, _mul64(va & _mask32, vb & _mask32)));
  vd = _rotr64(vd ^ va, 16);
  vc = _add64(
      _add64(vc, vd), _mul64(BigInt.two, _mul64(vc & _mask32, vd & _mask32)));
  vb = _rotr64(vb ^ vc, 63);
  v[a] = va;
  v[b] = vb;
  v[c] = vc;
  v[d] = vd;
}

void _permutationP(List<BigInt> v) {
  _g(v, 0, 4, 8, 12);
  _g(v, 1, 5, 9, 13);
  _g(v, 2, 6, 10, 14);
  _g(v, 3, 7, 11, 15);
  _g(v, 0, 5, 10, 15);
  _g(v, 1, 6, 11, 12);
  _g(v, 2, 7, 8, 13);
  _g(v, 3, 4, 9, 14);
}

/// Compression with explicit row/column windows. Dart lists have no
/// mutable slices (sublist copies), so windows are copied out,
/// permuted, and copied back.
List<BigInt> _compress(List<BigInt> x, List<BigInt> y) {
  final r = List<BigInt>.generate(_blockWords, (i) => x[i] ^ y[i]);
  final q = List<BigInt>.of(r);
  for (var i = 0; i < 8; i++) {
    final window = q.sublist(i * 16, (i + 1) * 16);
    _permutationP(window);
    q.setRange(i * 16, (i + 1) * 16, window);
  }
  final col = List<BigInt>.filled(16, BigInt.zero);
  for (var c = 0; c < 8; c++) {
    for (var row = 0; row < 8; row++) {
      col[2 * row] = q[row * 16 + 2 * c];
      col[2 * row + 1] = q[row * 16 + 2 * c + 1];
    }
    _permutationP(col);
    for (var row = 0; row < 8; row++) {
      q[row * 16 + 2 * c] = col[2 * row];
      q[row * 16 + 2 * c + 1] = col[2 * row + 1];
    }
  }
  return List<BigInt>.generate(_blockWords, (i) => r[i] ^ q[i]);
}

Uint8List _blockToBytes(List<BigInt> block) {
  final out = Uint8List(_blockSize);
  for (var i = 0; i < _blockWords; i++) {
    final w = block[i];
    for (var b = 0; b < 8; b++) {
      out[i * 8 + b] = ((w >> (8 * b)) & BigInt.from(0xff)).toInt();
    }
  }
  return out;
}

List<BigInt> _bytesToBlock(List<int> data) {
  return List<BigInt>.generate(_blockWords, (i) {
    var w = BigInt.zero;
    for (var b = 7; b >= 0; b--) {
      w = (w << 8) | BigInt.from(data[i * 8 + b]);
    }
    return w;
  });
}

Uint8List _le32(int n) => Uint8List.fromList([
      n & 0xff,
      (n >> 8) & 0xff,
      (n >> 16) & 0xff,
      (n >> 24) & 0xff,
    ]);

/// One BLAKE2b call with Argon2 error wrapping.
///
/// The wrap arm is defensive only: every digest size reaching it is
/// valid by construction (4..64), so the lane's blake2b cannot fail.
/// It mirrors the reference's `map_err` at each call site.
Uint8List _blake2bOrWrap(List<int> input, int digestSize) {
  try {
    return blake2b(input, Blake2bOptions(digestSize: digestSize));
  } on Blake2bError catch (e) {
    throw Argon2Blake2bError(e.toString());
  }
}

/// Variable-length hash H' built over BLAKE2b.
Uint8List _blake2bLong(int t, List<int> x) {
  if (t == 0) {
    throw const Argon2Blake2bError("H' output length must be positive");
  }
  final input = [..._le32(t), ...x];

  if (t <= 64) {
    return _blake2bOrWrap(input, t);
  }

  final r = (t + 31) ~/ 32 - 2;
  var v = _blake2bOrWrap(input, 64);
  final out = <int>[...v.sublist(0, 32)];
  for (var i = 1; i < r; i++) {
    v = _blake2bOrWrap(v, 64);
    out.addAll(v.sublist(0, 32));
  }
  final finalSize = t - 32 * r;
  out.addAll(_blake2bOrWrap(v, finalSize));
  return Uint8List.fromList(out);
}

int _indexAlpha(
  BigInt j1,
  int r,
  int sl,
  int c,
  bool sameLane,
  int q,
  int slLen,
) {
  late final int w;
  late final int start;
  if (r == 0 && sl == 0) {
    w = c - 1;
    start = 0;
  } else if (r == 0) {
    if (sameLane) {
      w = sl * slLen + c - 1;
    } else if (c == 0) {
      w = sl * slLen - 1;
    } else {
      w = sl * slLen;
    }
    start = 0;
  } else {
    if (sameLane) {
      w = q - slLen + c - 1;
    } else if (c == 0) {
      w = q - slLen - 1;
    } else {
      w = q - slLen;
    }
    start = ((sl + 1) * slLen) % q;
  }

  final x = (j1 * j1) >> 32;
  final y = ((BigInt.from(w) * x) >> 32).toInt();
  final rel = w - 1 - y;
  // Dart's % is Euclidean (non-negative for positive q), matching
  // Rust's rem_euclid exactly.
  return (start + rel) % q;
}

class _AddressStream {
  final int r;
  final int lane;
  final int sl;
  final int mPrime;
  final int t;
  int counter = 0;
  List<BigInt> buf = List<BigInt>.filled(_blockWords, BigInt.zero);
  int idx = _addressesPerBlock;

  _AddressStream(this.r, this.lane, this.sl, this.mPrime, this.t);

  BigInt next() {
    if (idx >= _addressesPerBlock) {
      counter += 1;
      final zero = List<BigInt>.filled(_blockWords, BigInt.zero);
      final inputBlock = List<BigInt>.filled(_blockWords, BigInt.zero);
      inputBlock[0] = BigInt.from(r);
      inputBlock[1] = BigInt.from(lane);
      inputBlock[2] = BigInt.from(sl);
      inputBlock[3] = BigInt.from(mPrime);
      inputBlock[4] = BigInt.from(t);
      inputBlock[5] = BigInt.from(_typeId);
      inputBlock[6] = BigInt.from(counter);
      final once = _compress(zero, inputBlock);
      buf = _compress(zero, once);
      idx = 0;
    }
    final v = buf[idx];
    idx += 1;
    return v;
  }
}

bool _dataIndependent(int r, int sl) => r == 0 && sl < 2;

void _fillSegment(
  List<List<List<BigInt>>> memory,
  int r,
  int lane,
  int sl,
  int q,
  int slLen,
  int p,
  int mPrime,
  int t,
) {
  final startingC = (r == 0 && sl == 0) ? 2 : 0;
  final di = _dataIndependent(r, sl);

  _AddressStream? addr;
  if (di) {
    addr = _AddressStream(r, lane, sl, mPrime, t);
    for (var i = 0; i < startingC; i++) {
      addr.next();
    }
  }

  for (var i = startingC; i < slLen; i++) {
    final col = sl * slLen + i;
    final prevCol = col == 0 ? q - 1 : col - 1;
    final prevBlock = List<BigInt>.of(memory[lane][prevCol]);

    final pseudoRand = di ? addr!.next() : prevBlock[0];
    final j1 = pseudoRand & BigInt.from(0xffffffff);
    final j2 = (pseudoRand >> 32) & BigInt.from(0xffffffff);

    final lPrime = (r == 0 && sl == 0) ? lane : (j2 % BigInt.from(p)).toInt();
    final zPrime = _indexAlpha(j1, r, sl, i, lPrime == lane, q, slLen);
    final refBlock = List<BigInt>.of(memory[lPrime][zPrime]);

    final newBlock = _compress(prevBlock, refBlock);
    if (r == 0) {
      memory[lane][col] = newBlock;
    } else {
      final existing = memory[lane][col];
      memory[lane][col] = List<BigInt>.generate(
        _blockWords,
        (k) => existing[k] ^ newBlock[k],
      );
    }
  }
}

void _validate(
  List<int> password,
  List<int> salt,
  int timeCost,
  int memoryCost,
  int parallelism,
  int tagLength,
  List<int> key,
  List<int> ad,
  int version,
) {
  const max32 = 0xffffffff;
  if (password.length > max32) {
    throw PasswordTooLong(password.length);
  }
  if (salt.length < 8) {
    throw SaltTooShort(salt.length);
  }
  if (salt.length > max32) {
    throw SaltTooLong(salt.length);
  }
  if (key.length > max32) {
    throw KeyTooLong(key.length);
  }
  if (ad.length > max32) {
    throw AssociatedDataTooLong(ad.length);
  }
  if (tagLength < 4) {
    throw TagLengthTooSmall(tagLength);
  }
  // The reference takes tag_length as u32, so its TagLengthTooLarge
  // variant is unreachable there; Dart ints are 64-bit, so the lane
  // enforces the bound the type carried in the reference. Without
  // this, _le32 would silently truncate the H0 domain separator and
  // _blake2bLong would attempt a t-byte allocation.
  if (tagLength > max32) {
    throw TagLengthTooLarge(tagLength);
  }
  if (parallelism < 1 || parallelism > 0xffffff) {
    throw InvalidParallelism(parallelism);
  }
  if (memoryCost < 8 * parallelism) {
    throw MemoryTooSmall(got: memoryCost, min: 8 * parallelism);
  }
  if (timeCost < 1) {
    throw const TimeCostZero();
  }
  if (version != argon2Version13) {
    throw UnsupportedVersion(version);
  }
}

/// Computes the Argon2id tag for [password] and [salt].
///
/// Cost parameters: [timeCost] passes, [memoryCost] kibibytes,
/// [parallelism] lanes, [tagLength] output bytes. Optional [opts]
/// carry a secret key, associated data, and a version override.
///
/// Throws [Argon2Error] when inputs fail validation.
Uint8List argon2id(
  List<int> password,
  List<int> salt, {
  required int timeCost,
  required int memoryCost,
  required int parallelism,
  required int tagLength,
  Argon2Options? opts,
}) {
  final options = opts ?? const Argon2Options();
  final key = options.key ?? const <int>[];
  final ad = options.associatedData ?? const <int>[];
  final version = options.version ?? argon2Version13;

  _validate(password, salt, timeCost, memoryCost, parallelism, tagLength, key,
      ad, version);

  final p = parallelism;
  final t = timeCost;
  final segmentLength = memoryCost ~/ (_syncPoints * parallelism);
  final mPrime = segmentLength * _syncPoints * p;
  final q = mPrime ~/ p;
  final slLen = segmentLength;

  final h0in = <int>[
    ..._le32(p),
    ..._le32(tagLength),
    ..._le32(memoryCost),
    ..._le32(t),
    ..._le32(version),
    ..._le32(_typeId),
    ..._le32(password.length),
    ...password,
    ..._le32(salt.length),
    ...salt,
    ..._le32(key.length),
    ...key,
    ..._le32(ad.length),
    ...ad,
  ];
  final h0 = blake2b(h0in, const Blake2bOptions(digestSize: 64));

  final memory = List<List<List<BigInt>>>.generate(
    p,
    (_) => List<List<BigInt>>.generate(
      q,
      (_) => List<BigInt>.filled(_blockWords, BigInt.zero),
    ),
  );

  for (var i = 0; i < p; i++) {
    final b0 = _blake2bLong(_blockSize, [...h0, ..._le32(0), ..._le32(i)]);
    final b1 = _blake2bLong(_blockSize, [...h0, ..._le32(1), ..._le32(i)]);
    memory[i][0] = _bytesToBlock(b0);
    memory[i][1] = _bytesToBlock(b1);
  }

  for (var r = 0; r < t; r++) {
    for (var sl = 0; sl < _syncPoints; sl++) {
      for (var lane = 0; lane < p; lane++) {
        _fillSegment(memory, r, lane, sl, q, slLen, p, mPrime, t);
      }
    }
  }

  final finalBlock = List<BigInt>.of(memory[0][q - 1]);
  for (var lane = 1; lane < p; lane++) {
    for (var k = 0; k < _blockWords; k++) {
      finalBlock[k] ^= memory[lane][q - 1][k];
    }
  }

  return _blake2bLong(tagLength, _blockToBytes(finalBlock));
}

/// Computes the Argon2id tag as lowercase hex.
String argon2idHex(
  List<int> password,
  List<int> salt, {
  required int timeCost,
  required int memoryCost,
  required int parallelism,
  required int tagLength,
  Argon2Options? opts,
}) {
  const digits = '0123456789abcdef';
  final buffer = StringBuffer();
  for (final byte in argon2id(password, salt,
      timeCost: timeCost,
      memoryCost: memoryCost,
      parallelism: parallelism,
      tagLength: tagLength,
      opts: opts)) {
    buffer.write(digits[(byte >> 4) & 0xf]);
    buffer.write(digits[byte & 0xf]);
  }
  return buffer.toString();
}
