// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

import 'dart:convert';
import 'dart:typed_data';

import 'cbor_types.dart';
import 'constants.dart';

/// Encodes a [CborValue] into canonical CBOR bytes adhering to RFC 8949 §4.2.3.
///
/// Invariants enforced:
///   - Smallest-form integer and length headers.
///   - Length-first then unsigned bytewise map key sorting.
///   - Duplicate map keys rejected with [CborErrorId.duplicateMapKey].
///   - Recursion depth capped at [maxNestingDepth] with [CborErrorId.encodeTooDeep].
///   - Output length capped at [maxEncodedBytes] with [CborErrorId.encodeTooLarge].
///   - Text strings validated so lone UTF-16 surrogates are rejected with [CborErrorId.invalidUtf8].
///
/// Throws [CborError] if validation fails.
Uint8List encodeChecked(CborValue value) {
  final buffer = <int>[];
  _encodeIntoBuffer(value, buffer, rootDepth);
  return Uint8List.fromList(buffer);
}

/// Appends the canonical CBOR encoding of [value] to [dst].
///
/// Atomic rollback guarantee: if encoding fails, [dst] is guaranteed to remain
/// unchanged.
void encodeIntoChecked(CborValue value, List<int> dst) {
  final encoded = encodeChecked(value);
  dst.addAll(encoded);
}

/// Convenience wrapper around [encodeChecked] for callers with trusted values.
Uint8List encode(CborValue value) => encodeChecked(value);

/// Convenience wrapper around [encodeIntoChecked] for callers with trusted values.
void encodeInto(CborValue value, List<int> dst) =>
    encodeIntoChecked(value, dst);

void _encodeIntoBuffer(CborValue v, List<int> out, int depth) {
  if (depth > maxNestingDepth) {
    throw const CborError(CborErrorId.encodeTooDeep);
  }

  switch (v.kind) {
    case CborKind.unsigned:
      _writeTypeAndArgument(out, majorUnsigned, (v as CborUnsigned).value);
      break;
    case CborKind.negative:
      _writeTypeAndArgument(out, majorNegative, (v as CborNegative).value);
      break;
    case CborKind.bytes:
      final b = (v as CborBytes).bytes;
      _writeTypeAndArgument(out, majorBytes, BigInt.from(b.length));
      _appendChecked(out, b);
      break;
    case CborKind.text:
      final t = (v as CborText).text;
      validateUtf16ScalarString(t);
      final utf8Bytes = utf8.encode(t);
      _writeTypeAndArgument(out, majorText, BigInt.from(utf8Bytes.length));
      _appendChecked(out, utf8Bytes);
      break;
    case CborKind.array:
      final items = (v as CborArray).items;
      _writeTypeAndArgument(out, majorArray, BigInt.from(items.length));
      for (final item in items) {
        _encodeIntoBuffer(item, out, depth + 1);
      }
      break;
    case CborKind.map:
      _encodeMap((v as CborMap).entries, out, depth);
      break;
    case CborKind.tag:
      final tagVal = v as CborTag;
      _writeTypeAndArgument(out, majorTag, tagVal.tag);
      _encodeIntoBuffer(tagVal.value, out, depth + 1);
      break;
    case CborKind.boolean:
      final b = (v as CborBool).value;
      _pushByteChecked(out, b ? wireByteTrue : wireByteFalse);
      break;
    case CborKind.nullValue:
      _pushByteChecked(out, wireByteNull);
      break;
  }
}

final class _EncodedMapEntry {
  final Uint8List encodedKey;
  final CborValue value;

  _EncodedMapEntry(this.encodedKey, this.value);
}

void _encodeMap(List<CborMapEntry> entries, List<int> out, int depth) {
  final encodedEntries = <_EncodedMapEntry>[];
  for (final entry in entries) {
    final keyBuf = <int>[];
    _encodeIntoBuffer(entry.key, keyBuf, depth + 1);
    encodedEntries
        .add(_EncodedMapEntry(Uint8List.fromList(keyBuf), entry.value));
  }

  encodedEntries.sort((a, b) {
    final lenCompare = a.encodedKey.length.compareTo(b.encodedKey.length);
    if (lenCompare != 0) return lenCompare;
    return _compareBytes(a.encodedKey, b.encodedKey);
  });

  for (var i = 1; i < encodedEntries.length; i++) {
    if (bytesEqual(
        encodedEntries[i - 1].encodedKey, encodedEntries[i].encodedKey)) {
      throw const CborError(CborErrorId.duplicateMapKey);
    }
  }

  _writeTypeAndArgument(out, majorMap, BigInt.from(encodedEntries.length));
  for (final entry in encodedEntries) {
    _appendChecked(out, entry.encodedKey);
    _encodeIntoBuffer(entry.value, out, depth + 1);
  }
}

int _compareBytes(Uint8List a, Uint8List b) {
  final minLen = a.length < b.length ? a.length : b.length;
  for (var i = 0; i < minLen; i++) {
    final diff = a[i].compareTo(b[i]);
    if (diff != 0) return diff;
  }
  return a.length.compareTo(b.length);
}

bool _keyStrictlyLess(Uint8List a, Uint8List b) {
  if (a.length != b.length) {
    return a.length < b.length;
  }
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) {
      return a[i] < b[i];
    }
  }
  return false;
}

void _writeBigEndianUint(List<int> out, BigInt value, int byteCount) {
  for (var i = byteCount - 1; i >= 0; i--) {
    final shift = i * bitsPerByte;
    _pushByteChecked(out, ((value >> shift) & BigInt.from(byteMask)).toInt());
  }
}

BigInt _readBigEndianUint(Uint8List bytes, int byteCount) {
  var val = BigInt.zero;
  for (var i = 0; i < byteCount; i++) {
    val = (val << bitsPerByte) | BigInt.from(bytes[i]);
  }
  return val;
}

void _writeTypeAndArgument(List<int> out, int major, BigInt arg) {
  if (arg < BigInt.zero || arg > maxUint64) {
    throw const CborError(CborErrorId.lengthTooLarge);
  }
  final majorBits = major << majorTypeShift;

  if (arg <= BigInt.from(infoInlineMax)) {
    _pushByteChecked(out, majorBits | arg.toInt());
  } else if (arg <= maxUint8) {
    _pushByteChecked(out, majorBits | info1ByteArg);
    _pushByteChecked(out, arg.toInt());
  } else if (arg <= maxUint16) {
    _pushByteChecked(out, majorBits | info2ByteArg);
    _writeBigEndianUint(out, arg, argWidth2Bytes);
  } else if (arg <= maxUint32) {
    _pushByteChecked(out, majorBits | info4ByteArg);
    _writeBigEndianUint(out, arg, argWidth4Bytes);
  } else {
    _pushByteChecked(out, majorBits | info8ByteArg);
    _writeBigEndianUint(out, arg, argWidth8Bytes);
  }
}

void _pushByteChecked(List<int> out, int b) {
  if (out.length >= maxEncodedBytes) {
    throw const CborError(CborErrorId.encodeTooLarge);
  }
  out.add(b);
}

void _appendChecked(List<int> out, List<int> data) {
  if (data.length > maxEncodedBytes - out.length) {
    throw const CborError(CborErrorId.encodeTooLarge);
  }
  out.addAll(data);
}

// ============================================================================
// Strict Decoder
// ============================================================================

final class _ByteCursor {
  final Uint8List data;
  int pos = 0;

  _ByteCursor(this.data);

  int get remaining => data.length - pos;

  int readU8() {
    if (pos >= data.length) {
      throw const CborError(CborErrorId.unexpectedEof);
    }
    final b = data[pos];
    pos++;
    return b;
  }

  Uint8List readN(int n) {
    if (n < 0) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    final end = pos + n;
    if (end < pos || end > data.length) {
      throw const CborError(CborErrorId.unexpectedEof);
    }
    final slice = Uint8List.sublistView(data, pos, end);
    pos = end;
    return slice;
  }
}

int _checkLength(BigInt declared, int remainingBytes, int minBytesPerUnit) {
  if (declared > BigInt.from(remainingBytes)) {
    throw const CborError(CborErrorId.lengthTooLarge);
  }
  final count = declared.toInt();
  if (count > remainingBytes ~/ minBytesPerUnit) {
    throw const CborError(CborErrorId.lengthTooLarge);
  }
  return count;
}

void _validateMinimalArgument(int major, int info, BigInt arg) {
  if (major == majorSimpleOrFloat) return;

  switch (info) {
    case info1ByteArg:
      if (arg <= BigInt.from(infoInlineMax)) {
        throw const CborError(CborErrorId.nonMinimalInteger);
      }
      break;
    case info2ByteArg:
      if (arg <= maxUint8) {
        throw const CborError(CborErrorId.nonMinimalInteger);
      }
      break;
    case info4ByteArg:
      if (arg <= maxUint16) {
        throw const CborError(CborErrorId.nonMinimalInteger);
      }
      break;
    case info8ByteArg:
      if (arg <= maxUint32) {
        throw const CborError(CborErrorId.nonMinimalInteger);
      }
      break;
  }
}

(int major, int info, BigInt arg) _readHeader(_ByteCursor reader) {
  final headerByte = reader.readU8();
  final major = headerByte >> majorTypeShift;
  final info = headerByte & additionalInfoMask;

  BigInt arg;
  if (info <= infoInlineMax) {
    arg = BigInt.from(info);
  } else if (info == info1ByteArg) {
    arg = BigInt.from(reader.readU8());
  } else if (info == info2ByteArg) {
    final bytes = reader.readN(argWidth2Bytes);
    arg = _readBigEndianUint(bytes, argWidth2Bytes);
  } else if (info == info4ByteArg) {
    final bytes = reader.readN(argWidth4Bytes);
    arg = _readBigEndianUint(bytes, argWidth4Bytes);
  } else if (info == info8ByteArg) {
    final bytes = reader.readN(argWidth8Bytes);
    arg = _readBigEndianUint(bytes, argWidth8Bytes);
  } else if (info >= infoReserved28 && info <= infoReserved30) {
    throw const CborError(CborErrorId.reserved);
  } else {
    // info == infoIndefinite (since additionalInfoMask is 0x1F, values 0..31 are exhausted)
    throw const CborError(CborErrorId.indefinite);
  }

  _validateMinimalArgument(major, info, arg);
  return (major, info, arg);
}

/// Parses a single canonical CBOR item from [data].
///
/// Invariants enforced:
///   - Exactly one valid item is consumed; trailing bytes throw [CborErrorId.trailingBytes].
///   - Indefinite items and break markers throw [CborErrorId.indefinite].
///   - Non-minimal integer/length representations throw [CborErrorId.nonMinimalInteger].
///   - Map keys must be strictly increasing in length-first order; out-of-order or duplicate keys throw [CborErrorId.nonCanonicalMapOrder].
///   - Declared lengths exceeding remaining input throw [CborErrorId.lengthTooLarge] before allocation.
///   - Text strings must contain valid UTF-8; invalid UTF-8 throws [CborErrorId.invalidUtf8].
///   - Nesting exceeding [maxNestingDepth] throws [CborErrorId.tooDeep].
///   - Floating-point representations throw [CborErrorId.floatNotSupported].
///   - Unassigned simple values throw [CborErrorId.unsupportedSimple].
CborValue decode(Uint8List data) {
  final reader = _ByteCursor(data);
  final value = _readValue(reader, rootDepth);
  if (reader.pos != data.length) {
    throw const CborError(CborErrorId.trailingBytes);
  }
  return value;
}

CborValue _readArray(_ByteCursor reader, int depth, BigInt arg) {
  final count = _checkLength(arg, reader.remaining, minBytesPerElement);
  final items = <CborValue>[];
  for (var i = 0; i < count; i++) {
    items.add(_readValue(reader, depth + 1));
  }
  return CborArray(items);
}

CborValue _readMap(_ByteCursor reader, int depth, BigInt arg) {
  final count = _checkLength(arg, reader.remaining, minBytesPerMapEntry);
  final spanned = _readSpannedMapEntries(reader, depth, count);
  final entries = spanned.map((s) => CborMapEntry(s.key, s.value)).toList();
  return CborMap(entries);
}

/// Reads [entryCount] spanned map entries from [reader] at [parentDepth].
///
/// Shared between [_readMap] and [decodeMapSpans] to ensure DRY single-source
/// validation of length-first map key ordering and duplicate rejection.
List<SpannedMapEntry> _readSpannedMapEntries(
    _ByteCursor reader, int parentDepth, int entryCount) {
  final entries = <SpannedMapEntry>[];
  Uint8List? prevKeyBytes;

  for (var i = 0; i < entryCount; i++) {
    final keyStart = reader.pos;
    final key = _readValue(reader, parentDepth + 1);
    final keyEnd = reader.pos;
    final curKeyBytes = Uint8List.sublistView(reader.data, keyStart, keyEnd);

    if (prevKeyBytes != null && !_keyStrictlyLess(prevKeyBytes, curKeyBytes)) {
      throw const CborError(CborErrorId.nonCanonicalMapOrder);
    }
    prevKeyBytes = curKeyBytes;

    final valStart = reader.pos;
    final value = _readValue(reader, parentDepth + 1);
    final valEnd = reader.pos;

    entries.add(SpannedMapEntry(
      key: key,
      value: value,
      valueSpan: Span(valStart, valEnd),
    ));
  }
  return entries;
}

CborValue _readValue(_ByteCursor reader, int depth) {
  if (depth > maxNestingDepth) {
    throw const CborError(CborErrorId.tooDeep);
  }

  final (major, info, arg) = _readHeader(reader);

  switch (major) {
    case majorUnsigned:
      return CborUnsigned(arg);
    case majorNegative:
      return CborNegative(arg);
    case majorBytes:
      final length = _checkLength(arg, reader.remaining, minBytesPerElement);
      final bs = reader.readN(length);
      return CborBytes(bs);
    case majorText:
      final length = _checkLength(arg, reader.remaining, minBytesPerElement);
      final bs = reader.readN(length);
      try {
        final str = utf8.decode(bs, allowMalformed: false);
        return CborText(str);
      } on FormatException {
        throw const CborError(CborErrorId.invalidUtf8);
      }
    case majorArray:
      return _readArray(reader, depth, arg);
    case majorMap:
      return _readMap(reader, depth, arg);
    case majorTag:
      final inner = _readValue(reader, depth + 1);
      return CborTag(arg, inner);
    case majorSimpleOrFloat:
      switch (info) {
        case simpleFalse:
          return const CborBool(false);
        case simpleTrue:
          return const CborBool(true);
        case simpleNull:
          return const CborNull();
        case simpleFloat16:
        case simpleFloat32:
        case simpleFloat64:
          throw const CborError(CborErrorId.floatNotSupported);
        default:
          throw const CborError(CborErrorId.unsupportedSimple);
      }
    default:
      throw const CborError(CborErrorId.unsupportedSimple);
  }
}

/// Decodes a single canonical CBOR map from [data] and reports the byte span
/// each entry's value occupied.
///
/// Invariant:
///   `data.sublist(entry.valueSpan.start, entry.valueSpan.end) == encodeChecked(entry.value)`
///   whenever `encodeChecked(entry.value)` succeeds.
///
/// If [data] is a valid canonical CBOR item that is not a map, returns
/// [DecodeMapSpansResult.nonMap] without throwing.
DecodeMapSpansResult decodeMapSpans(Uint8List data) {
  if (data.isEmpty) {
    throw const CborError(CborErrorId.unexpectedEof);
  }
  if ((data[0] >> majorTypeShift) != majorMap) {
    // Confirm data is fully valid canonical CBOR before declaring shape mismatch:
    decode(data);
    return const DecodeMapSpansResult.nonMap();
  }

  final reader = _ByteCursor(data);
  final (_, _, arg) = _readHeader(reader);
  final count = _checkLength(arg, reader.remaining, minBytesPerMapEntry);
  final entries = _readSpannedMapEntries(reader, rootDepth, count);

  if (reader.pos != data.length) {
    throw const CborError(CborErrorId.trailingBytes);
  }
  return DecodeMapSpansResult.map(entries);
}
