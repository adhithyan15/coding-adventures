import 'dart:typed_data';

import 'constants.dart';
import 'messages.dart';

const int digestLength = Sha512Constants.digestLength;

final List<String> _hexChars = [
  '0',
  '1',
  '2',
  '3',
  '4',
  '5',
  '6',
  '7',
  '8',
  '9',
  'a',
  'b',
  'c',
  'd',
  'e',
  'f',
];

/// Encodes 128-bit byte length into a 16-byte big-endian bit length field (FIPS 180-4 §5.1.2).
/// Enforces the 2^128-bit maximum domain bound before bit-shift transformation.
Uint8List encodeBitLength(BigInt highU64, BigInt lowU64) {
  // Enforce FIPS domain bound: bits 61..63 of high 64-bit counter must be 0
  if (highU64 < BigInt.zero ||
      lowU64 < BigInt.zero ||
      highU64 >= Sha512Constants.overflowLimitHigh ||
      lowU64 > Sha512Constants.mask64) {
    throw Sha512Exception(Sha512ErrorCode.messageTooLong);
  }
  // Multiply 128-bit byte count by 8 to produce bit length, carrying 3 bits from low to high
  final bitHigh = ((highU64 << Sha512Constants.bitsPerByteShift) |
          (lowU64 >> Sha512Constants.byteToBitCarryShift)) &
      Sha512Constants.mask64;
  final bitLow =
      (lowU64 << Sha512Constants.bitsPerByteShift) & Sha512Constants.mask64;

  final result = Uint8List(Sha512Constants.lengthFieldBytes);
  _putBigIntBigEndian(result, 0, bitHigh);
  _putBigIntBigEndian(result, Sha512Constants.wordBytes, bitLow);
  return result;
}

void _putBigIntBigEndian(Uint8List out, int offset, BigInt val) {
  var current = val;
  for (var i = 7; i >= 0; i--) {
    out[offset + i] = (current & BigInt.from(Sha512Constants.byteMask)).toInt();
    current = current >> Sha512Constants.wordBytes;
  }
}

String _toHex(List<int> bytes) {
  final sb = StringBuffer();
  for (var i = 0; i < bytes.length; i++) {
    final b = bytes[i];
    sb.write(
      _hexChars[
          (b >>> Sha512Constants.nibbleBits) & Sha512Constants.nibbleMask],
    );
    sb.write(_hexChars[b & Sha512Constants.nibbleMask]);
  }
  return sb.toString();
}

Word64 _ch(Word64 x, Word64 y, Word64 z) => (x & y) ^ ((~x) & z);
Word64 _maj(Word64 x, Word64 y, Word64 z) => (x & y) ^ (x & z) ^ (y & z);

Word64 _bigSigma0(Word64 x) =>
    x.rotr(Sha512Constants.bs0R1) ^
    x.rotr(Sha512Constants.bs0R2) ^
    x.rotr(Sha512Constants.bs0R3);

Word64 _bigSigma1(Word64 x) =>
    x.rotr(Sha512Constants.bs1R1) ^
    x.rotr(Sha512Constants.bs1R2) ^
    x.rotr(Sha512Constants.bs1R3);

Word64 _smallSigma0(Word64 x) =>
    x.rotr(Sha512Constants.ss0R1) ^
    x.rotr(Sha512Constants.ss0R2) ^
    x.shr(Sha512Constants.ss0S);

Word64 _smallSigma1(Word64 x) =>
    x.rotr(Sha512Constants.ss1R1) ^
    x.rotr(Sha512Constants.ss1R2) ^
    x.shr(Sha512Constants.ss1S);

Word64 _readWord(List<int> block, int offset) {
  final hi = ((block[offset] & Sha512Constants.byteMask) << 24) |
      ((block[offset + 1] & Sha512Constants.byteMask) << 16) |
      ((block[offset + 2] & Sha512Constants.byteMask) << 8) |
      (block[offset + 3] & Sha512Constants.byteMask);
  final lo = ((block[offset + 4] & Sha512Constants.byteMask) << 24) |
      ((block[offset + 5] & Sha512Constants.byteMask) << 16) |
      ((block[offset + 6] & Sha512Constants.byteMask) << 8) |
      (block[offset + 7] & Sha512Constants.byteMask);
  return Word64(hi & Sha512Constants.mask32, lo & Sha512Constants.mask32);
}

void _writeWord(Uint8List out, int offset, Word64 w) {
  out[offset] = (w.hi >>> 24) & Sha512Constants.byteMask;
  out[offset + 1] = (w.hi >>> 16) & Sha512Constants.byteMask;
  out[offset + 2] = (w.hi >>> 8) & Sha512Constants.byteMask;
  out[offset + 3] = w.hi & Sha512Constants.byteMask;
  out[offset + 4] = (w.lo >>> 24) & Sha512Constants.byteMask;
  out[offset + 5] = (w.lo >>> 16) & Sha512Constants.byteMask;
  out[offset + 6] = (w.lo >>> 8) & Sha512Constants.byteMask;
  out[offset + 7] = w.lo & Sha512Constants.byteMask;
}

void _compress(List<Word64> state, List<int> block, int offset) {
  final w = List<Word64>.filled(Sha512Constants.rounds, Word64.zero);

  // Initialize first 16 words from big-endian block bytes
  for (var i = 0; i < Sha512Constants.scheduleInitWords; i++) {
    w[i] = _readWord(block, offset + i * Sha512Constants.wordBytes);
  }

  // Expand 16-word message schedule to 80 rounds
  for (var t = Sha512Constants.scheduleInitWords;
      t < Sha512Constants.rounds;
      t++) {
    w[t] =
        _smallSigma1(w[t - 2]) + w[t - 7] + _smallSigma0(w[t - 15]) + w[t - 16];
  }

  // Load working variables
  var a = state[0];
  var b = state[1];
  var c = state[2];
  var d = state[3];
  var e = state[4];
  var f = state[5];
  var g = state[6];
  var h = state[7];

  // Execute 80 mixing rounds
  for (var t = 0; t < Sha512Constants.rounds; t++) {
    final t1 = h + _bigSigma1(e) + _ch(e, f, g) + Sha512Constants.k[t] + w[t];
    final t2 = _bigSigma0(a) + _maj(a, b, c);
    h = g;
    g = f;
    f = e;
    e = d + t1;
    d = c;
    c = b;
    b = a;
    a = t1 + t2;
  }

  // Feed-forward accumulation into state
  state[0] = state[0] + a;
  state[1] = state[1] + b;
  state[2] = state[2] + c;
  state[3] = state[3] + d;
  state[4] = state[4] + e;
  state[5] = state[5] + f;
  state[6] = state[6] + g;
  state[7] = state[7] + h;
}

/// Streaming SHA-512 Hasher complying with the HF04 contract.
class Sha512Hasher {
  final List<Word64> _state;
  final Uint8List _buffer;
  int _bufferLength;
  BigInt _byteCountHigh;
  BigInt _byteCountLow;

  Sha512Hasher()
      : _state = List<Word64>.of(Sha512Constants.initialState),
        _buffer = Uint8List(Sha512Constants.blockSize),
        _bufferLength = 0,
        _byteCountHigh = BigInt.zero,
        _byteCountLow = BigInt.zero;

  Sha512Hasher._copy(
    this._state,
    this._buffer,
    this._bufferLength,
    this._byteCountHigh,
    this._byteCountLow,
  );

  void _setByteCounters(BigInt high, BigInt low) {
    _byteCountHigh = high;
    _byteCountLow = low;
  }

  Sha512Hasher update(List<int> data, [int offset = 0, int? length]) {
    // Overflow-safe bounds checking prevents integer wrapping attacks
    if (offset < 0 || offset > data.length) {
      throw RangeError(Sha512Messages.errInvalidOffsetOrLength);
    }
    final effectiveLength = length ?? (data.length - offset);
    if (effectiveLength < 0 || effectiveLength > data.length - offset) {
      throw RangeError(Sha512Messages.errInvalidOffsetOrLength);
    }
    if (effectiveLength == 0) {
      return this;
    }

    // 1. Synchronously validate all bytes: (b & ~0xFF) != 0 before state mutation
    for (var i = 0; i < effectiveLength; i++) {
      final b = data[offset + i];
      if ((b & ~Sha512Constants.byteMask) != 0) {
        throw Sha512Exception(Sha512ErrorCode.invalidByte);
      }
    }

    // 2. Failure-atomic 128-bit domain check before mutating internal state
    final added = BigInt.from(effectiveLength);
    final nextLow = _byteCountLow + added;
    final carry = nextLow >> Sha512Constants.wordBits;
    final nextLowMasked = nextLow & Sha512Constants.mask64;
    final nextHigh = _byteCountHigh + carry;
    if (nextHigh >= Sha512Constants.overflowLimitHigh) {
      throw Sha512Exception(Sha512ErrorCode.messageTooLong);
    }

    // Commit counter advances only after domain bound validation succeeds
    _byteCountLow = nextLowMasked;
    _byteCountHigh = nextHigh;

    var curOffset = offset;
    var remaining = effectiveLength;

    // Drain partial buffer if present
    if (_bufferLength > 0) {
      final needed = Sha512Constants.blockSize - _bufferLength;
      final toCopy = remaining < needed ? remaining : needed;
      _buffer.setRange(_bufferLength, _bufferLength + toCopy, data, curOffset);
      _bufferLength += toCopy;
      curOffset += toCopy;
      remaining -= toCopy;

      if (_bufferLength == Sha512Constants.blockSize) {
        _compress(_state, _buffer, 0);
        _bufferLength = 0;
      }
    }

    // Process full 128-byte blocks directly from caller array without copying
    while (remaining >= Sha512Constants.blockSize) {
      _compress(_state, data, curOffset);
      curOffset += Sha512Constants.blockSize;
      remaining -= Sha512Constants.blockSize;
    }

    // Stash remaining partial block in internal buffer
    if (remaining > 0) {
      _buffer.setRange(0, remaining, data, curOffset);
      _bufferLength = remaining;
    }

    return this;
  }

  /// Non-destructive snapshot of the hash state at the current byte position.
  Uint8List digest() {
    // Snapshot working state so caller can continue streaming afterward
    final snapshotState = List<Word64>.of(_state);
    final tail = Uint8List(Sha512Constants.blockSize * 2);
    tail.setRange(0, _bufferLength, _buffer);
    tail[_bufferLength] = Sha512Constants.paddingMarker;

    // Zero-pad until tail length reaches 112 mod 128
    var padLen = _bufferLength + 1;
    while (
        (padLen % Sha512Constants.blockSize) != Sha512Constants.paddingOffset) {
      tail[padLen++] = 0;
    }

    // Append 128-bit big-endian bit length
    final lenBytes = encodeBitLength(_byteCountHigh, _byteCountLow);
    tail.setRange(padLen, padLen + Sha512Constants.lengthFieldBytes, lenBytes);
    padLen += Sha512Constants.lengthFieldBytes;

    // Compress 1 or 2 final padded blocks
    var i = 0;
    while (i < padLen) {
      _compress(snapshotState, tail, i);
      i += Sha512Constants.blockSize;
    }

    // Serialize 8 words into 64-byte big-endian digest
    final digest = Uint8List(Sha512Constants.digestLength);
    for (var j = 0; j < Sha512Constants.stateWords; j++) {
      _writeWord(digest, j * Sha512Constants.wordBytes, snapshotState[j]);
    }
    return digest;
  }

  String hexDigest() => _toHex(digest());

  Sha512Hasher copy() {
    final copyBuf = Uint8List(Sha512Constants.blockSize);
    copyBuf.setRange(0, _bufferLength, _buffer);
    return Sha512Hasher._copy(
      List<Word64>.of(_state),
      copyBuf,
      _bufferLength,
      _byteCountHigh,
      _byteCountLow,
    );
  }

  Sha512Hasher cloneHasher() => copy();

  void reset() {
    for (var i = 0; i < Sha512Constants.stateWords; i++) {
      _state[i] = Sha512Constants.initialState[i];
    }
    _bufferLength = 0;
    _byteCountHigh = BigInt.zero;
    _byteCountLow = BigInt.zero;
  }
}

/// Compute the SHA-512 digest of [data] and return it as a 64-byte [Uint8List].
Uint8List sha512(List<int> data) {
  final hasher = Sha512Hasher();
  hasher.update(data);
  return hasher.digest();
}

/// Compute the SHA-512 digest of [data] and return the 128-character lowercase hex string.
String sha512Hex(List<int> data) {
  final hasher = Sha512Hasher();
  hasher.update(data);
  return hasher.hexDigest();
}

/// Internal test helper for simulating counter transitions without public API exposure.
void setHasherByteCountersForTesting(
  Sha512Hasher hasher,
  BigInt high,
  BigInt low,
) {
  hasher._setByteCounters(high, low);
}

/// Exception thrown by the SHA-512 package on validation failure or domain overflow.
class Sha512Exception implements Exception {
  final Sha512ErrorCode code;
  final String message;

  Sha512Exception(this.code) : message = Sha512Messages.messageFor(code);

  @override
  String toString() => message;
}
