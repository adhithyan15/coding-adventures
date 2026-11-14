// Copyright (c) 2026, the coding-adventures project authors.
// All rights reserved. Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

import 'dart:typed_data';

import 'constants.dart';
import 'messages.dart';

/// Maximum 64-bit unsigned integer value (2^64 - 1).
final BigInt maxUint64 = (BigInt.one << 64) - BigInt.one;

/// Validates that a Dart string consists strictly of valid Unicode scalar values.
///
/// In Dart, [String] is represented as UTF-16 code units. If an unpaired high
/// or low surrogate is passed, Dart's standard [utf8.encode] silently replaces
/// it with U+FFFD ('\xEF\xBF\xBD'), creating duplicate encodings and hash collisions
/// (a critical security vulnerability). This validator ensures lone surrogates are
/// rejected immediately with [CborErrorId.invalidUtf8].
void validateUtf16ScalarString(String text) {
  final units = text.codeUnits;
  final length = units.length;
  for (var i = 0; i < length; i++) {
    final unit = units[i];
    if (unit >= surrogateHighStart && unit <= surrogateHighEnd) {
      if (i + 1 >= length) {
        throw const CborError(CborErrorId.invalidUtf8);
      }
      final next = units[i + 1];
      if (next < surrogateLowStart || next > surrogateLowEnd) {
        throw const CborError(CborErrorId.invalidUtf8);
      }
      i++; // Skip the paired low surrogate
    } else if (unit >= surrogateLowStart && unit <= surrogateLowEnd) {
      throw const CborError(CborErrorId.invalidUtf8);
    }
  }
}

/// The 14 closed, language-neutral error identifiers defined by the CBR01 specification.
enum CborErrorId {
  unexpectedEof('unexpected-eof'),
  trailingBytes('trailing-bytes'),
  reserved('reserved'),
  indefinite('indefinite'),
  nonMinimalInteger('non-minimal-integer'),
  invalidUtf8('invalid-utf8'),
  nonCanonicalMapOrder('non-canonical-map-order'),
  unsupportedSimple('unsupported-simple'),
  floatNotSupported('float-not-supported'),
  tooDeep('too-deep'),
  lengthTooLarge('length-too-large'),
  duplicateMapKey('duplicate-map-key'),
  encodeTooDeep('encode-too-deep'),
  encodeTooLarge('encode-too-large');

  final String value;
  const CborErrorId(this.value);
}

/// A stable, payload-blind CBR01 conformance error.
///
/// Threat model invariant: [toString] returns a static literal diagnostic
/// prefixed with `canonical-cbor: ` that never reflects user keys, payload bytes,
/// stream offsets, or numeric lengths into log files.
class CborError implements Exception {
  final CborErrorId id;

  const CborError(this.id);

  @override
  String toString() => errorMessageFor(id);

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is CborError && other.id == id);

  @override
  int get hashCode => id.hashCode;
}

/// A half-open byte index range [start, end) within an input slice.
final class Span {
  final int start;
  final int end;

  const Span(this.start, this.end);

  int get length => end - start;

  @override
  String toString() => '[$start, $end)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is Span && other.start == start && other.end == end);

  @override
  int get hashCode => Object.hash(start, end);
}

/// A decoded map entry alongside the byte span occupied by its value.
///
/// In accordance with Vault security standards, [toString] redacts the key
/// and value payloads to prevent sensitive credentials or cryptographic keys
/// from being leaked into telemetry or logging pipelines.
final class SpannedMapEntry {
  final CborValue key;
  final CborValue value;
  final Span valueSpan;

  const SpannedMapEntry({
    required this.key,
    required this.value,
    required this.valueSpan,
  });

  @override
  String toString() =>
      'SpannedMapEntry(key: $redactedPlaceholder, value: $redactedPlaceholder, valueSpan: $valueSpan)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is SpannedMapEntry &&
          other.key == key &&
          other.value == value &&
          other.valueSpan == valueSpan);

  @override
  int get hashCode => Object.hash(key, value, valueSpan);
}

/// Result of [decodeMapSpans] distinguishing shape mismatches from profile errors.
final class DecodeMapSpansResult {
  final List<SpannedMapEntry>? entries;
  final bool isMap;

  const DecodeMapSpansResult.map(List<SpannedMapEntry> this.entries)
      : isMap = true;

  const DecodeMapSpansResult.nonMap()
      : entries = null,
        isMap = false;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is DecodeMapSpansResult &&
          other.isMap == isMap &&
          _spannedEntriesEqual(entries, other.entries));

  @override
  int get hashCode => Object.hash(isMap, entries?.length);
}

bool _spannedEntriesEqual(List<SpannedMapEntry>? a, List<SpannedMapEntry>? b) {
  if (identical(a, b)) return true;
  if (a == null || b == null) return false;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// The high-level data types defined by the CBOR data model.
enum CborKind {
  unsigned,
  negative,
  bytes,
  text,
  array,
  map,
  tag,
  boolean,
  nullValue,
}

/// Abstract base class for all canonical CBOR values.
///
/// Sealed class hierarchy allows exhaustive compile-time pattern matching in Dart 3.
sealed class CborValue {
  const CborValue();

  CborKind get kind;
}

/// Major type 0: Unsigned 64-bit integer.
final class CborUnsigned extends CborValue {
  final BigInt value;

  CborUnsigned(BigInt value)
      : value = _validateU64(value),
        super();

  factory CborUnsigned.fromInt(int value) {
    if (value < 0) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    return CborUnsigned(BigInt.from(value));
  }

  static BigInt _validateU64(BigInt v) {
    if (v < BigInt.zero || v > maxUint64) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    return v;
  }

  @override
  CborKind get kind => CborKind.unsigned;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is CborUnsigned && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'CborUnsigned($value)';
}

/// Major type 1: Negative integer, represented by the unsigned argument n in -1 - n.
final class CborNegative extends CborValue {
  final BigInt value;

  CborNegative(BigInt value)
      : value = _validateU64(value),
        super();

  factory CborNegative.fromInt(int value) {
    if (value < 0) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    return CborNegative(BigInt.from(value));
  }

  static BigInt _validateU64(BigInt v) {
    if (v < BigInt.zero || v > maxUint64) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    return v;
  }

  @override
  CborKind get kind => CborKind.negative;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is CborNegative && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'CborNegative($value)';
}

/// Major type 2: Byte string.
final class CborBytes extends CborValue {
  final Uint8List _bytes;

  CborBytes(List<int> bytes)
      : _bytes = Uint8List.fromList(bytes),
        super();

  Uint8List get bytes => Uint8List.fromList(_bytes);

  int get length => _bytes.length;

  @override
  CborKind get kind => CborKind.bytes;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is CborBytes && _bytesEqual(_bytes, other._bytes));

  @override
  int get hashCode => _bytesHashCode(_bytes);

  @override
  String toString() => 'CborBytes($_bytes)';
}

/// Major type 3: UTF-8 text string.
final class CborText extends CborValue {
  final String text;

  CborText(String text)
      : text = text,
        super() {
    validateUtf16ScalarString(text);
  }

  @override
  CborKind get kind => CborKind.text;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is CborText && other.text == text);

  @override
  int get hashCode => text.hashCode;

  @override
  String toString() => 'CborText("$text")';
}

/// Major type 4: Array of CBOR items.
final class CborArray extends CborValue {
  final List<CborValue> _items;

  CborArray(List<CborValue> items)
      : _items = List<CborValue>.unmodifiable(items),
        super();

  List<CborValue> get items => _items;

  int get length => _items.length;

  @override
  CborKind get kind => CborKind.array;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is CborArray && _listEquals(_items, other._items));

  @override
  int get hashCode => Object.hashAll(_items);

  @override
  String toString() => 'CborArray($_items)';
}

/// A key-value pair in a CBOR map.
final class CborMapEntry {
  final CborValue key;
  final CborValue value;

  const CborMapEntry(this.key, this.value);

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is CborMapEntry && other.key == key && other.value == value);

  @override
  int get hashCode => Object.hash(key, value);

  @override
  String toString() => 'CborMapEntry($key, $value)';
}

/// Major type 5: Map of key-value pairs.
final class CborMap extends CborValue {
  final List<CborMapEntry> _entries;

  CborMap(List<CborMapEntry> entries)
      : _entries = List<CborMapEntry>.unmodifiable(entries),
        super();

  List<CborMapEntry> get entries => _entries;

  int get length => _entries.length;

  @override
  CborKind get kind => CborKind.map;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is CborMap && _mapEntriesEqual(_entries, other._entries));

  @override
  int get hashCode => Object.hashAll(_entries);

  @override
  String toString() => 'CborMap($_entries)';
}

/// Major type 6: Tagged CBOR item.
final class CborTag extends CborValue {
  final BigInt tag;
  final CborValue value;

  CborTag(BigInt tag, this.value)
      : tag = _validateU64(tag),
        super();

  factory CborTag.fromInt(int tag, CborValue value) {
    if (tag < 0) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    return CborTag(BigInt.from(tag), value);
  }

  static BigInt _validateU64(BigInt v) {
    if (v < BigInt.zero || v > maxUint64) {
      throw const CborError(CborErrorId.lengthTooLarge);
    }
    return v;
  }

  @override
  CborKind get kind => CborKind.tag;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      (other is CborTag && other.tag == tag && other.value == value);

  @override
  int get hashCode => Object.hash(tag, value);

  @override
  String toString() => 'CborTag($tag, $value)';
}

/// Major type 7: Boolean value (simple 20 or 21).
final class CborBool extends CborValue {
  final bool value;

  const CborBool(this.value);

  @override
  CborKind get kind => CborKind.boolean;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || (other is CborBool && other.value == value);

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'CborBool($value)';
}

/// Major type 7: Null value (simple 22).
final class CborNull extends CborValue {
  const CborNull();

  @override
  CborKind get kind => CborKind.nullValue;

  @override
  bool operator ==(Object other) => other is CborNull;

  @override
  int get hashCode => 0;

  @override
  String toString() => 'CborNull()';
}

// Byte-slice comparison helpers:

bool _bytesEqual(Uint8List a, Uint8List b) {
  if (identical(a, b)) return true;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

int _bytesHashCode(Uint8List bytes) {
  var hash = 0;
  for (var i = 0; i < bytes.length; i++) {
    hash = (hash * 31 + bytes[i]) & 0x3FFFFFFF;
  }
  return hash;
}

bool _listEquals(List<CborValue> a, List<CborValue> b) {
  if (identical(a, b)) return true;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

bool _mapEntriesEqual(List<CborMapEntry> a, List<CborMapEntry> b) {
  if (identical(a, b)) return true;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
