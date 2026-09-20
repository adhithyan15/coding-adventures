import 'dart:typed_data';

import 'package:coding_adventures_der_tlv/der_tlv.dart';

const packageName = 'coding_adventures_der_asn1';
const packageVersion = '0.1.0';
const defaultMaxDepth = 32;
const defaultMaxTotalElements = 16384;
const defaultMaxOidArcs = 128;

final class Asn1Limits {
  Asn1Limits({
    DerLimits? der,
    this.maxDepth = defaultMaxDepth,
    this.maxTotalElements = defaultMaxTotalElements,
    this.maxOidArcs = defaultMaxOidArcs,
  }) : der = der ?? DerLimits() {
    if (maxDepth < 0 || maxTotalElements < 0 || maxOidArcs < 0) {
      throw ArgumentError('ASN.1 limits must be non-negative');
    }
  }

  final DerLimits der;
  final int maxDepth;
  final int maxTotalElements;
  final int maxOidArcs;

  Asn1Limits copyWith({
    DerLimits? der,
    int? maxDepth,
    int? maxTotalElements,
    int? maxOidArcs,
  }) =>
      Asn1Limits(
        der: der ?? this.der,
        maxDepth: maxDepth ?? this.maxDepth,
        maxTotalElements: maxTotalElements ?? this.maxTotalElements,
        maxOidArcs: maxOidArcs ?? this.maxOidArcs,
      );

  @override
  bool operator ==(Object other) =>
      other is Asn1Limits &&
      _sameDerLimits(der, other.der) &&
      maxDepth == other.maxDepth &&
      maxTotalElements == other.maxTotalElements &&
      maxOidArcs == other.maxOidArcs;

  @override
  int get hashCode => Object.hash(
        der.maxInputLen,
        der.maxValueLen,
        der.maxElements,
        der.maxTagNumber,
        maxDepth,
        maxTotalElements,
        maxOidArcs,
      );
}

bool _sameDerLimits(DerLimits left, DerLimits right) =>
    left.maxInputLen == right.maxInputLen &&
    left.maxValueLen == right.maxValueLen &&
    left.maxElements == right.maxElements &&
    left.maxTagNumber == right.maxTagNumber;

enum Asn1ErrorKind {
  framing('framing'),
  unexpectedTag('unexpected-tag'),
  decoderLimitMismatch('decoder-limit-mismatch'),
  depthLimitExceeded('depth-limit-exceeded'),
  elementLimitExceeded('element-limit-exceeded'),
  invalidBooleanLength('invalid-boolean-length'),
  invalidBooleanValue('invalid-boolean-value'),
  emptyInteger('empty-integer'),
  nonMinimalInteger('non-minimal-integer'),
  negativeInteger('negative-integer'),
  integerOverflow('integer-overflow'),
  missingUnusedBitCount('missing-unused-bit-count'),
  invalidUnusedBitCount('invalid-unused-bit-count'),
  nonZeroBitPadding('non-zero-bit-padding'),
  bitLengthOverflow('bit-length-overflow'),
  nonEmptyNull('non-empty-null'),
  nonAsciiIa5String('non-ascii-ia5-string'),
  emptyObjectIdentifier('empty-object-identifier'),
  unterminatedObjectIdentifier('unterminated-object-identifier'),
  nonMinimalObjectIdentifier('non-minimal-object-identifier'),
  objectIdentifierOverflow('object-identifier-overflow'),
  oidArcLimitExceeded('oid-arc-limit-exceeded');

  const Asn1ErrorKind(this.id);
  final String id;
}

final class Asn1Exception implements Exception {
  const Asn1Exception(this.kind, this.offset, [this.framingKind]);

  factory Asn1Exception.framing(DerException error) =>
      Asn1Exception(Asn1ErrorKind.framing, error.offset, error.kind);

  final Asn1ErrorKind kind;
  final int offset;
  final DerErrorKind? framingKind;

  @override
  String toString() => 'ASN.1 DER value error ${kind.id} at byte $offset';
}

Never _fail(Asn1ErrorKind kind, int offset) =>
    throw Asn1Exception(kind, offset);

final class Asn1Element {
  const Asn1Element(this.element, this.depth);

  final DerElement element;
  final int depth;

  DerTag get tag => element.tag;
  Uint8List get header => element.header;
  Uint8List get value => element.value;
  Uint8List get encoded => element.encoded;
  int get valueOffset => header.length;
}

final class Asn1Decoder {
  Asn1Decoder([Asn1Limits? limits]) : _limits = limits ?? Asn1Limits();

  final Asn1Limits _limits;
  var _elementsRead = 0;

  Asn1Limits get limits => _limits;
  int get elementsRead => _elementsRead;

  Asn1Element decodeExact(Uint8List input) {
    if (_limits.maxDepth == 0) {
      _fail(Asn1ErrorKind.depthLimitExceeded, 0);
    }
    _requireElementCapacity(0);
    try {
      final element = codingAdventuresDerDecodeExact(input, _limits.der);
      _elementsRead += 1;
      return Asn1Element(element, 0);
    } on DerException catch (error) {
      throw Asn1Exception.framing(error);
    }
  }

  Asn1Cursor sequence(Asn1Element element) =>
      _constructed(element, TagClass.universal, 16);

  Asn1Cursor set(Asn1Element element) =>
      _constructed(element, TagClass.universal, 17);

  Asn1Element explicit(Asn1Element element, int tagNumber) {
    _expectTag(element, TagClass.contextSpecific, true, tagNumber);
    final childDepth = _childDepth(element);
    _requireElementCapacity(element.valueOffset);
    try {
      final child = codingAdventuresDerDecodeExact(element.value, _limits.der);
      _elementsRead += 1;
      return Asn1Element(child, childDepth);
    } on DerException catch (error) {
      throw Asn1Exception.framing(error);
    }
  }

  Asn1Cursor _constructed(
    Asn1Element element,
    TagClass tagClass,
    int number,
  ) {
    _expectTag(element, tagClass, true, number);
    final childDepth = _childDepth(element);
    try {
      return Asn1Cursor._(
        DerCursor(element.value, _limits.der),
        childDepth,
        _limits,
      );
    } on DerException catch (error) {
      throw Asn1Exception.framing(error);
    }
  }

  int _childDepth(Asn1Element element) {
    final childDepth = element.depth + 1;
    if (childDepth >= _limits.maxDepth) {
      _fail(Asn1ErrorKind.depthLimitExceeded, 0);
    }
    return childDepth;
  }

  void _requireElementCapacity(int offset) {
    if (_elementsRead >= _limits.maxTotalElements) {
      _fail(Asn1ErrorKind.elementLimitExceeded, offset);
    }
  }
}

final class Asn1Cursor {
  const Asn1Cursor._(this._cursor, this._childDepth, this._limits);

  final DerCursor _cursor;
  final int _childDepth;
  final Asn1Limits _limits;

  Uint8List get remaining => _cursor.remaining;

  Asn1Element? read(Asn1Decoder decoder) {
    if (_cursor.remaining.isEmpty) return null;
    if (decoder.limits != _limits) {
      _fail(Asn1ErrorKind.decoderLimitMismatch, 0);
    }
    decoder._requireElementCapacity(0);
    try {
      final element = _cursor.read();
      if (element == null) return null;
      decoder._elementsRead += 1;
      return Asn1Element(element, _childDepth);
    } on DerException catch (error) {
      throw Asn1Exception.framing(error);
    }
  }

  void finish() {
    try {
      _cursor.finish();
    } on DerException catch (error) {
      throw Asn1Exception.framing(error);
    }
  }
}

final class DerInteger {
  const DerInteger(this.signedBytes, this._valueOffset);

  final Uint8List signedBytes;
  final int _valueOffset;

  bool get isNegative => signedBytes[0] & 0x80 != 0;

  BigInt toU64() {
    if (isNegative) {
      _fail(Asn1ErrorKind.negativeInteger, _valueOffset);
    }
    final magnitude = signedBytes[0] == 0
        ? Uint8List.sublistView(signedBytes, 1)
        : signedBytes;
    if (magnitude.length > 8) {
      _fail(Asn1ErrorKind.integerOverflow, _valueOffset);
    }
    var value = BigInt.zero;
    for (final octet in magnitude) {
      value = value * BigInt.from(256) + BigInt.from(octet);
    }
    return value;
  }
}

final class DerBitString {
  const DerBitString(this.bytes, this.unusedBits, this.bitLength);

  final Uint8List bytes;
  final int unusedBits;
  final int bitLength;
}

final class ObjectIdentifier {
  ObjectIdentifier(this.encoded, List<BigInt> arcs)
      : arcs = List.unmodifiable(arcs);

  final Uint8List encoded;
  final List<BigInt> arcs;

  int get arcCount => arcs.length;

  bool equals(Iterable<BigInt> expected) {
    final values = expected.toList(growable: false);
    if (arcs.length != values.length) return false;
    for (var index = 0; index < arcs.length; index += 1) {
      if (arcs[index] != values[index]) return false;
    }
    return true;
  }
}

bool decodeBoolean(Asn1Element element) {
  _expectUniversalPrimitive(element, 1);
  if (element.value.length != 1) {
    _fail(Asn1ErrorKind.invalidBooleanLength, element.valueOffset);
  }
  if (element.value[0] == 0) return false;
  if (element.value[0] == 0xff) return true;
  _fail(Asn1ErrorKind.invalidBooleanValue, element.valueOffset);
}

DerInteger decodeInteger(Asn1Element element) {
  _expectUniversalPrimitive(element, 2);
  final value = element.value;
  if (value.isEmpty) {
    _fail(Asn1ErrorKind.emptyInteger, element.valueOffset);
  }
  if (value.length > 1 &&
      ((value[0] == 0 && value[1] & 0x80 == 0) ||
          (value[0] == 0xff && value[1] & 0x80 != 0))) {
    _fail(Asn1ErrorKind.nonMinimalInteger, element.valueOffset);
  }
  return DerInteger(value, element.valueOffset);
}

DerBitString decodeBitString(Asn1Element element) {
  _expectUniversalPrimitive(element, 3);
  final value = element.value;
  if (value.isEmpty) {
    _fail(Asn1ErrorKind.missingUnusedBitCount, element.valueOffset);
  }
  final unusedBits = value[0];
  final payload = Uint8List.sublistView(value, 1);
  if (unusedBits > 7 || (payload.isEmpty && unusedBits != 0)) {
    _fail(Asn1ErrorKind.invalidUnusedBitCount, element.valueOffset);
  }
  if (unusedBits != 0 && payload.last & ((1 << unusedBits) - 1) != 0) {
    _fail(
      Asn1ErrorKind.nonZeroBitPadding,
      element.valueOffset + value.length - 1,
    );
  }
  return DerBitString(
    payload,
    unusedBits,
    payload.length * 8 - unusedBits,
  );
}

Uint8List decodeOctetString(Asn1Element element) {
  _expectUniversalPrimitive(element, 4);
  return element.value;
}

Uint8List decodeImplicitOctetString(Asn1Element element, int tagNumber) {
  _expectContextPrimitive(element, tagNumber);
  return element.value;
}

String decodeIA5String(Asn1Element element) {
  _expectUniversalPrimitive(element, 22);
  return _decodeIA5Contents(element);
}

String decodeImplicitIA5String(Asn1Element element, int tagNumber) {
  _expectContextPrimitive(element, tagNumber);
  return _decodeIA5Contents(element);
}

void decodeNull(Asn1Element element) {
  _expectUniversalPrimitive(element, 5);
  if (element.value.isNotEmpty) {
    _fail(Asn1ErrorKind.nonEmptyNull, element.valueOffset);
  }
}

ObjectIdentifier decodeObjectIdentifier(
  Asn1Element element, [
  Asn1Limits? limits,
]) {
  _expectUniversalPrimitive(element, 6);
  return _decodeOidContents(element, limits ?? Asn1Limits());
}

ObjectIdentifier decodeImplicitObjectIdentifier(
  Asn1Element element,
  int tagNumber, [
  Asn1Limits? limits,
]) {
  _expectContextPrimitive(element, tagNumber);
  return _decodeOidContents(element, limits ?? Asn1Limits());
}

String _decodeIA5Contents(Asn1Element element) {
  for (var offset = 0; offset < element.value.length; offset += 1) {
    if (element.value[offset] > 0x7f) {
      _fail(
        Asn1ErrorKind.nonAsciiIa5String,
        element.valueOffset + offset,
      );
    }
  }
  return String.fromCharCodes(element.value);
}

ObjectIdentifier _decodeOidContents(
  Asn1Element element,
  Asn1Limits limits,
) {
  final encoded = element.value;
  if (encoded.isEmpty) {
    _fail(Asn1ErrorKind.emptyObjectIdentifier, element.valueOffset);
  }
  final first = _parseBase128(encoded, 0, element.valueOffset);
  final combined = first.value;
  final forty = BigInt.from(40);
  final eighty = BigInt.from(80);
  final arcs = combined < forty
      ? [BigInt.zero, combined]
      : combined < eighty
          ? [BigInt.one, combined - forty]
          : [BigInt.two, combined - eighty];
  if (arcs.length > limits.maxOidArcs) {
    _fail(Asn1ErrorKind.oidArcLimitExceeded, element.valueOffset);
  }
  var offset = first.nextOffset;
  while (offset < encoded.length) {
    final arcStart = offset;
    final parsed = _parseBase128(encoded, offset, element.valueOffset);
    arcs.add(parsed.value);
    if (arcs.length > limits.maxOidArcs) {
      _fail(
        Asn1ErrorKind.oidArcLimitExceeded,
        element.valueOffset + arcStart,
      );
    }
    offset = parsed.nextOffset;
  }
  return ObjectIdentifier(encoded, arcs);
}

final class _ParsedBase128 {
  const _ParsedBase128(this.value, this.nextOffset);

  final BigInt value;
  final int nextOffset;
}

_ParsedBase128 _parseBase128(
  Uint8List encoded,
  int start,
  int valueOffset,
) {
  if (encoded[start] == 0x80) {
    _fail(Asn1ErrorKind.nonMinimalObjectIdentifier, valueOffset + start);
  }
  var value = BigInt.zero;
  final maxU64 = (BigInt.one << 64) - BigInt.one;
  for (var offset = start;; offset += 1) {
    if (offset >= encoded.length) {
      _fail(Asn1ErrorKind.unterminatedObjectIdentifier, valueOffset + offset);
    }
    final octet = encoded[offset];
    final candidate = value * BigInt.from(128) + BigInt.from(octet & 0x7f);
    if (candidate > maxU64) {
      _fail(Asn1ErrorKind.objectIdentifierOverflow, valueOffset + offset);
    }
    value = candidate;
    if (octet & 0x80 == 0) {
      return _ParsedBase128(value, offset + 1);
    }
  }
}

void _expectUniversalPrimitive(Asn1Element element, int number) {
  _expectTag(element, TagClass.universal, false, number);
}

void _expectContextPrimitive(Asn1Element element, int number) {
  _expectTag(element, TagClass.contextSpecific, false, number);
}

void _expectTag(
  Asn1Element element,
  TagClass tagClass,
  bool constructed,
  int number,
) {
  final tag = element.tag;
  if (tag.tagClass != tagClass ||
      tag.constructed != constructed ||
      tag.number != number) {
    _fail(Asn1ErrorKind.unexpectedTag, 0);
  }
}

// Keep the imported framing function visually distinct from this package's
// decoder method without hiding the dependency behind a wrapper class.
DerElement codingAdventuresDerDecodeExact(
  Uint8List input,
  DerLimits limits,
) =>
    decodeExact(input, limits);
