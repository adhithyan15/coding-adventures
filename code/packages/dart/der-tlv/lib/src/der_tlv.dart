import 'dart:typed_data';

const packageName = 'coding_adventures_der_tlv';
const packageVersion = '0.1.0';
const defaultMaxInputLen = 1024 * 1024;
const defaultMaxValueLen = 1024 * 1024;
const defaultMaxElements = 4096;

enum TagClass {
  universal('universal'),
  application('application'),
  contextSpecific('context-specific'),
  private('private');

  const TagClass(this.wireName);
  final String wireName;
}

enum DerErrorKind {
  emptyInput('empty-input'),
  truncatedHighTag('truncated-high-tag'),
  truncatedLength('truncated-length'),
  truncatedValue('truncated-value'),
  endOfContents('end-of-contents'),
  nonMinimalTag('non-minimal-tag'),
  tagOverflow('tag-overflow'),
  indefiniteLength('indefinite-length'),
  reservedLength('reserved-length'),
  nonMinimalLength('non-minimal-length'),
  lengthTooWide('length-too-wide'),
  lengthHostOverflow('length-host-overflow'),
  inputLimitExceeded('input-limit-exceeded'),
  valueLimitExceeded('value-limit-exceeded'),
  elementLimitExceeded('element-limit-exceeded'),
  tagLimitExceeded('tag-limit-exceeded'),
  trailingData('trailing-data');

  const DerErrorKind(this.id);
  final String id;
}

final class DerException implements Exception {
  const DerException(this.kind, this.offset);
  final DerErrorKind kind;
  final int offset;

  @override
  String toString() => 'DER framing error ${kind.id} at byte $offset';
}

final class DerLimits {
  DerLimits({
    this.maxInputLen = defaultMaxInputLen,
    BigInt? maxValueLen,
    this.maxElements = defaultMaxElements,
    this.maxTagNumber = 0xffffffff,
  }) : maxValueLen = maxValueLen ?? BigInt.from(defaultMaxValueLen) {
    if (maxInputLen < 0 || this.maxValueLen.isNegative || maxElements < 0) {
      throw ArgumentError('DER limits must be non-negative');
    }
    if (maxTagNumber < 0 || maxTagNumber > 0xffffffff) {
      throw ArgumentError('maxTagNumber must fit u32');
    }
  }

  final int maxInputLen;
  final BigInt maxValueLen;
  final int maxElements;
  final int maxTagNumber;
}

final class DerTag {
  const DerTag(this.tagClass, this.constructed, this.number);
  final TagClass tagClass;
  final bool constructed;
  final int number;
}

final class DerElement {
  const DerElement._(
    this.tag,
    this._input,
    this._start,
    this._headerLen,
    this._encodedLen,
  );

  final DerTag tag;
  final Uint8List _input;
  final int _start;
  final int _headerLen;
  final int _encodedLen;

  Uint8List get header =>
      Uint8List.sublistView(_input, _start, _start + _headerLen);
  Uint8List get value => Uint8List.sublistView(
        _input,
        _start + _headerLen,
        _start + _encodedLen,
      );
  Uint8List get encoded =>
      Uint8List.sublistView(_input, _start, _start + _encodedLen);
}

final class DecodeResult {
  const DecodeResult(this.element, this.remainder);
  final DerElement element;
  final Uint8List remainder;
}

final class _Decoded {
  const _Decoded(this.element, this.nextOffset);
  final DerElement element;
  final int nextOffset;
}

Never _fail(DerErrorKind kind, int offset) => throw DerException(kind, offset);

_Decoded _decodeAt(
    Uint8List input, int start, int available, DerLimits limits) {
  if (available > limits.maxInputLen) {
    _fail(DerErrorKind.inputLimitExceeded, start);
  }
  if (available == 0) _fail(DerErrorKind.emptyInput, start);

  final first = input[start];
  final tagClass = TagClass.values[first >> 6];
  final constructed = first & 0x20 != 0;
  final low = first & 0x1f;
  var number = BigInt.zero;
  var identifierLen = 1;
  if (low != 0x1f) {
    number = BigInt.from(low);
    if (number > BigInt.from(limits.maxTagNumber)) {
      _fail(DerErrorKind.tagLimitExceeded, start);
    }
  } else {
    var index = 1;
    while (true) {
      if (index >= available) {
        _fail(DerErrorKind.truncatedHighTag, start + index);
      }
      final octet = input[start + index];
      final payload = BigInt.from(octet & 0x7f);
      if (index == 1 && payload == BigInt.zero) {
        _fail(DerErrorKind.nonMinimalTag, start + index);
      }
      final candidate = number * BigInt.from(128) + payload;
      if (candidate > BigInt.from(0xffffffff)) {
        _fail(DerErrorKind.tagOverflow, start + index);
      }
      number = candidate;
      if (number > BigInt.from(limits.maxTagNumber)) {
        _fail(DerErrorKind.tagLimitExceeded, start + index);
      }
      index += 1;
      if (octet & 0x80 == 0) break;
    }
    if (number < BigInt.from(31)) {
      _fail(DerErrorKind.nonMinimalTag, start);
    }
    identifierLen = index;
  }

  if (tagClass == TagClass.universal && number == BigInt.zero) {
    _fail(DerErrorKind.endOfContents, start);
  }

  final lengthOffset = start + identifierLen;
  if (identifierLen >= available) {
    _fail(DerErrorKind.truncatedLength, lengthOffset);
  }
  final firstLength = input[lengthOffset];
  var valueLen = BigInt.zero;
  var lengthLen = 1;
  if (firstLength < 0x80) {
    valueLen = BigInt.from(firstLength);
  } else {
    if (firstLength == 0x80) {
      _fail(DerErrorKind.indefiniteLength, lengthOffset);
    }
    if (firstLength == 0xff) {
      _fail(DerErrorKind.reservedLength, lengthOffset);
    }
    final count = firstLength & 0x7f;
    if (count > 8) _fail(DerErrorKind.lengthTooWide, lengthOffset);
    final valueStart = identifierLen + 1;
    final valueEnd = valueStart + count;
    if (valueEnd > available) {
      _fail(DerErrorKind.truncatedLength, start + available);
    }
    if (input[start + valueStart] == 0) {
      _fail(DerErrorKind.nonMinimalLength, start + valueStart);
    }
    for (var index = valueStart; index < valueEnd; index += 1) {
      valueLen =
          valueLen * BigInt.from(256) + BigInt.from(input[start + index]);
    }
    if (valueLen < BigInt.from(128)) {
      _fail(DerErrorKind.nonMinimalLength, lengthOffset);
    }
    lengthLen = 1 + count;
  }

  final hostMax = BigInt.from(0x7fffffffffffffff);
  if (valueLen > hostMax) {
    _fail(DerErrorKind.lengthHostOverflow, lengthOffset);
  }
  if (valueLen > limits.maxValueLen) {
    _fail(DerErrorKind.valueLimitExceeded, lengthOffset);
  }
  final headerLen = identifierLen + lengthLen;
  if (valueLen > hostMax - BigInt.from(headerLen)) {
    _fail(DerErrorKind.lengthHostOverflow, lengthOffset);
  }
  final encodedLen = headerLen + valueLen.toInt();
  if (encodedLen > available) {
    _fail(DerErrorKind.truncatedValue, start + available);
  }
  return _Decoded(
    DerElement._(
      DerTag(tagClass, constructed, number.toInt()),
      input,
      start,
      headerLen,
      encodedLen,
    ),
    start + encodedLen,
  );
}

DecodeResult decodeOne(Uint8List input, [DerLimits? limits]) {
  final decoded = _decodeAt(input, 0, input.length, limits ?? DerLimits());
  return DecodeResult(
    decoded.element,
    Uint8List.sublistView(input, decoded.nextOffset),
  );
}

DerElement decodeExact(Uint8List input, [DerLimits? limits]) {
  final decoded = _decodeAt(input, 0, input.length, limits ?? DerLimits());
  if (decoded.nextOffset != input.length) {
    _fail(DerErrorKind.trailingData, decoded.nextOffset);
  }
  return decoded.element;
}

final class DerCursor {
  DerCursor(this._input, [DerLimits? limits])
      : _limits = limits ?? DerLimits() {
    if (_input.length > _limits.maxInputLen) {
      _fail(DerErrorKind.inputLimitExceeded, 0);
    }
  }

  final Uint8List _input;
  final DerLimits _limits;
  var _offset = 0;
  var _elementsRead = 0;

  int get elementsRead => _elementsRead;
  Uint8List get remaining => Uint8List.sublistView(_input, _offset);

  DerElement? read() {
    if (_offset == _input.length) return null;
    if (_elementsRead >= _limits.maxElements) {
      _fail(DerErrorKind.elementLimitExceeded, _offset);
    }
    final decoded =
        _decodeAt(_input, _offset, _input.length - _offset, _limits);
    _offset = decoded.nextOffset;
    _elementsRead += 1;
    return decoded.element;
  }

  void finish() {
    if (_offset != _input.length) {
      _fail(DerErrorKind.trailingData, _offset);
    }
  }
}
