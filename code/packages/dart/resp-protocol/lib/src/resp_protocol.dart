/// RESP (REdis Serialization Protocol): the Dart lane of DT23.
///
/// RESP is the wire format between Redis clients and servers: five typed
/// values (simple strings, errors, integers, bulk strings, arrays) framed
/// with a type byte, length prefixes, and CRLF terminators. Bulk strings
/// carry explicit lengths, so payloads may hold arbitrary binary data.
///
/// This lane mirrors the sibling lanes: constructor helpers, total `encode`,
/// incremental `decode` (null when more bytes are needed, throw when
/// malformed), `decodeAll` for pipelined streams, and a streaming
/// [RespDecoder]. Integer parsing is strict (rejects trailing garbage),
/// matching the strict lanes.
///
/// ```dart
/// import 'package:coding_adventures_resp_protocol/coding_adventures_resp_protocol.dart';
///
/// void main() {
///   final frame = array([bulkString('SET'), bulkString('k'), bulkString('v')]);
///   final wire = encode(frame);
///   final back = decode(wire)!;
///   assert(back.consumed == wire.length);
/// }
/// ```
library resp_protocol;

import 'dart:convert';
import 'dart:typed_data';

// ─── Values ─────────────────────────────────────────────────────────────────
//
// Five RESP2 value shapes. All are immutable value types with deep equality
// so tests can compare decoded output structurally.

/// A decoded RESP value.
sealed class RespValue {
  const RespValue();
}

/// `+OK\r\n`: short non-binary status replies.
class RespSimpleString extends RespValue {
  /// The status text (no CR or LF by construction).
  final String value;
  const RespSimpleString(this.value);

  @override
  bool operator ==(Object other) =>
      other is RespSimpleString && other.value == value;

  @override
  int get hashCode => Object.hash('simple-string', value);

  @override
  String toString() => 'RespSimpleString($value)';
}

/// `-ERR boom\r\n`: short non-binary error replies.
class RespErrorValue extends RespValue {
  /// The error text.
  final String value;
  const RespErrorValue(this.value);

  @override
  bool operator ==(Object other) =>
      other is RespErrorValue && other.value == value;

  @override
  int get hashCode => Object.hash('error', value);

  @override
  String toString() => 'RespErrorValue($value)';
}

/// `:42\r\n`: 64-bit signed integers.
class RespInteger extends RespValue {
  /// The integer value.
  final int value;
  const RespInteger(this.value);

  @override
  bool operator ==(Object other) =>
      other is RespInteger && other.value == value;

  @override
  int get hashCode => Object.hash('integer', value);

  @override
  String toString() => 'RespInteger($value)';
}

/// `$3\r\nfoo\r\n` (or `$-1\r\n` for null): counted binary-safe strings.
class RespBulkString extends RespValue {
  /// Payload bytes, or null for the null bulk string.
  final Uint8List? value;
  const RespBulkString(this.value);

  @override
  bool operator ==(Object other) =>
      other is RespBulkString && _bytesEqual(value, other.value);

  @override
  int get hashCode => Object.hash('bulk-string', _bytesHash(value));

  @override
  String toString() => 'RespBulkString(${value?.length} bytes)';
}

/// `*2\r\n...` (or `*-1\r\n` for null): counted nested arrays.
class RespArray extends RespValue {
  /// Elements, or null for the null array.
  final List<RespValue>? value;
  const RespArray(this.value);

  @override
  bool operator ==(Object other) {
    if (other is! RespArray) return false;
    if (value == null || other.value == null) {
      return value == null && other.value == null;
    }
    if (value!.length != other.value!.length) return false;
    for (var i = 0; i < value!.length; i++) {
      if (value![i] != other.value![i]) return false;
    }
    return true;
  }

  @override
  int get hashCode => value == null
      // Deep element hash: List.hashCode is identity-based and would break
      // the ==/hashCode contract for equal arrays.
      ? Object.hash('array', null)
      : Object.hashAll(['array', ...value!]);

  @override
  String toString() => 'RespArray(${value?.length} items)';
}

bool _bytesEqual(Uint8List? a, Uint8List? b) {
  if (identical(a, b)) return true;
  if (a == null || b == null) return false;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

int _bytesHash(Uint8List? bytes) {
  if (bytes == null) return 0;
  var hash = 0;
  for (final b in bytes) {
    hash = (hash * 31 + b) & 0x3fffffff;
  }
  return hash;
}

/// One decoded value plus the bytes consumed.
class RespDecodeResult {
  /// The decoded value.
  final RespValue value;

  /// Bytes consumed from the input start.
  final int consumed;
  const RespDecodeResult(this.value, this.consumed);
}

/// Result of decoding a pipelined stream.
class RespDecodeAllResult {
  /// Values decoded in order.
  final List<RespValue> values;

  /// Total bytes consumed.
  final int consumed;
  const RespDecodeAllResult(this.values, this.consumed);
}

/// Malformed input (as opposed to merely truncated input).
class RespDecodeError implements Exception {
  /// What was malformed.
  final String message;
  const RespDecodeError(this.message);
  @override
  String toString() => 'RespDecodeError: $message';
}

/// An unencodable value (reserved for future strictness checks).
class RespEncodeError implements Exception {
  /// What could not be encoded.
  final String message;
  const RespEncodeError(this.message);
  @override
  String toString() => 'RespEncodeError: $message';
}

// ─── Constructors ───────────────────────────────────────────────────────────

/// Builds a simple string.
RespSimpleString simpleString(String value) => RespSimpleString(value);

/// Builds an error value.
RespErrorValue errorValue(String value) => RespErrorValue(value);

/// Builds an integer value.
RespInteger integer(int value) => RespInteger(value);

/// Builds a bulk string, copying [value] (UTF-8 for [String]).
RespBulkString bulkString(Object? value) {
  if (value == null) return const RespBulkString(null);
  if (value is String) {
    return RespBulkString(Uint8List.fromList(utf8.encode(value)));
  }
  if (value is Uint8List) {
    return RespBulkString(Uint8List.fromList(value));
  }
  if (value is List<int>) {
    return RespBulkString(Uint8List.fromList(value));
  }
  throw ArgumentError('bulkString takes String, bytes, or null');
}

/// Builds an array value.
RespArray array(List<RespValue>? value) => RespArray(value);

// ─── Encoding ───────────────────────────────────────────────────────────────
//
// Encoding is total over well-formed values: every constructor output has
// exactly one wire form (integers render decimally, byte counts prefix bulk
// payloads and arrays, nulls render as -1).

/// Encodes [value] to its wire form.
Uint8List encode(RespValue value) {
  switch (value) {
    case RespSimpleString(:final value):
      return encodeSimpleString(value);
    case RespErrorValue(:final value):
      return encodeError(value);
    case RespInteger(:final value):
      return encodeInteger(value);
    case RespBulkString(:final value):
      return encodeBulkString(value);
    case RespArray(:final value):
      return encodeArray(value);
  }
}

/// Encodes `+<value>\r\n`.
Uint8List encodeSimpleString(String value) =>
    Uint8List.fromList(utf8.encode('+$value\r\n'));

/// Encodes `-<value>\r\n`.
Uint8List encodeError(String value) =>
    Uint8List.fromList(utf8.encode('-$value\r\n'));

/// Encodes `:<value>\r\n` (Dart ints are always integers by construction).
Uint8List encodeInteger(int value) =>
    Uint8List.fromList(utf8.encode(':$value\r\n'));

/// Encodes `$<len>\r\n<bytes>\r\n`, or `$-1\r\n` for null.
Uint8List encodeBulkString(Object? value) {
  if (value == null) {
    return Uint8List.fromList(utf8.encode('\$-1\r\n'));
  }
  final bytes = value is String ? utf8.encode(value) : (value as List<int>);
  return _concatBytes([
    utf8.encode('\$${bytes.length}\r\n'),
    bytes,
    utf8.encode('\r\n'),
  ]);
}

/// Encodes `*<len>\r\n<elements>`, or `*-1\r\n` for null.
Uint8List encodeArray(List<RespValue>? values) {
  if (values == null) {
    return Uint8List.fromList(utf8.encode('*-1\r\n'));
  }
  final parts = <List<int>>[utf8.encode('*${values.length}\r\n')];
  for (final value in values) {
    parts.add(encode(value));
  }
  return _concatBytes(parts);
}

// ─── Decoding ───────────────────────────────────────────────────────────────
//
// Decoding is incremental: truncated input returns null (feed more bytes),
// while malformed input throws [RespDecodeError]. Anything that is not one
// of the five type bytes opens an inline command (a bare line split on
// whitespace), which is how `telnet` sessions speak to Redis.

/// Decodes one value from [input], or null when truncated.
///
/// Throws [RespDecodeError] on malformed input.
RespDecodeResult? decode(Object input) {
  final buffer = _asBytes(input);
  if (buffer.isEmpty) return null;

  switch (buffer[0]) {
    case 43: // +
      return _decodeSimpleString(buffer);
    case 45: // -
      return _decodeError(buffer);
    case 58: // :
      return _decodeInteger(buffer);
    case 36: // $
      return _decodeBulkString(buffer);
    case 42: // *
      return _decodeArray(buffer);
    default:
      return _decodeInlineCommand(buffer);
  }
}

/// Decodes pipelined values until input runs out or truncates.
RespDecodeAllResult decodeAll(Object input) {
  final buffer = _asBytes(input);
  final values = <RespValue>[];
  var offset = 0;
  while (offset < buffer.length) {
    final result = decode(buffer.sublist(offset));
    if (result == null) break;
    values.add(result.value);
    offset += result.consumed;
  }
  return RespDecodeAllResult(values, offset);
}

/// Streaming decoder: feed chunks, pull complete messages.
class RespDecoder {
  Uint8List _buffer = Uint8List(0);
  final List<RespValue> _queue = [];

  /// Appends [data] and decodes every complete message.
  ///
  /// Malformed bytes throw [RespDecodeError] out of `feed` (matching the
  /// sibling lanes, which likewise let decode errors propagate rather than
  /// stashing them).
  void feed(Object data) {
    _buffer = _concatBytes([_buffer, _asBytes(data)]);
    _drain();
  }

  /// True when at least one complete message waits.
  bool hasMessage() => _queue.isNotEmpty;

  /// Removes and returns the oldest message.
  ///
  /// Throws [RespDecodeError] when the queue is empty.
  RespValue getMessage() {
    if (_queue.isEmpty) {
      throw const RespDecodeError('decoder buffer is empty');
    }
    return _queue.removeAt(0);
  }

  /// Feeds [data] and returns every message decoded so far.
  List<RespValue> decodeAll(Object data) {
    feed(data);
    final messages = [..._queue];
    _queue.clear();
    return messages;
  }

  void _drain() {
    while (_buffer.isNotEmpty) {
      final result = decode(_buffer);
      if (result == null) return;
      _queue.add(result.value);
      _buffer = Uint8List.fromList(_buffer.sublist(result.consumed));
    }
  }
}

RespDecodeResult? _decodeSimpleString(Uint8List buffer) {
  final line = _readLine(buffer, 1);
  if (line == null) return null;
  return RespDecodeResult(
    simpleString(utf8.decode(line.line)),
    line.consumed,
  );
}

RespDecodeResult? _decodeError(Uint8List buffer) {
  final line = _readLine(buffer, 1);
  if (line == null) return null;
  return RespDecodeResult(
    errorValue(utf8.decode(line.line)),
    line.consumed,
  );
}

/// Strict decimal shape: optional minus, digits only.
///
/// `int.tryParse` alone would accept surrounding whitespace and `+`, which
/// the strict sibling lanes reject — so the shape gates the parse.
bool _isStrictInt(String text) =>
    text.isNotEmpty &&
    (text.codeUnitAt(0) == 45
        ? text.length > 1 && _allDigits(text, 1)
        : _allDigits(text, 0));

bool _allDigits(String text, int start) {
  if (start >= text.length) return false;
  for (var i = start; i < text.length; i++) {
    final unit = text.codeUnitAt(i);
    if (unit < 48 || unit > 57) return false;
  }
  return true;
}

int? _strictInt(String text) {
  if (!_isStrictInt(text)) return null;
  try {
    return int.parse(text);
  } on FormatException {
    // Out of 64-bit range (correct shape, unrepresentable value).
    return null;
  }
}

RespDecodeResult? _decodeInteger(Uint8List buffer) {
  final line = _readLine(buffer, 1);
  if (line == null) return null;
  final value = _strictInt(utf8.decode(line.line));
  if (value == null) {
    throw const RespDecodeError('invalid RESP integer');
  }
  return RespDecodeResult(integer(value), line.consumed);
}

RespDecodeResult? _decodeBulkString(Uint8List buffer) {
  final line = _readLine(buffer, 1);
  if (line == null) return null;
  final length = _strictInt(utf8.decode(line.line));
  if (length == null) {
    throw const RespDecodeError('invalid RESP bulk string length');
  }
  if (length == -1) {
    return RespDecodeResult(bulkString(null), line.consumed);
  }
  if (length < -1) {
    throw const RespDecodeError('bulk string length cannot be negative');
  }

  final bodyStart = line.consumed;
  final bodyEnd = bodyStart + length;
  final tailEnd = bodyEnd + 2;
  if (buffer.length < tailEnd) return null;
  if (buffer[bodyEnd] != 13 || buffer[bodyEnd + 1] != 10) {
    throw const RespDecodeError('missing trailing CRLF after bulk string body');
  }
  return RespDecodeResult(
    RespBulkString(Uint8List.fromList(buffer.sublist(bodyStart, bodyEnd))),
    tailEnd,
  );
}

RespDecodeResult? _decodeArray(Uint8List buffer) {
  final line = _readLine(buffer, 1);
  if (line == null) return null;
  final count = _strictInt(utf8.decode(line.line));
  if (count == null) {
    throw const RespDecodeError('invalid RESP array length');
  }
  if (count == -1) {
    return RespDecodeResult(array(null), line.consumed);
  }
  if (count < -1) {
    throw const RespDecodeError('array length cannot be negative');
  }

  final values = <RespValue>[];
  var offset = line.consumed;
  for (var i = 0; i < count; i++) {
    final result = decode(buffer.sublist(offset));
    if (result == null) return null;
    values.add(result.value);
    offset += result.consumed;
  }
  return RespDecodeResult(array(values), offset);
}

RespDecodeResult? _decodeInlineCommand(Uint8List buffer) {
  final line = _readLine(buffer, 0);
  if (line == null) return null;
  final text = utf8.decode(line.line).trim();
  final tokens = text.isEmpty
      ? <RespValue>[]
      : text
          .split(RegExp(r'\s+'))
          .where((token) => token.isNotEmpty)
          .map(bulkString)
          .toList();
  return RespDecodeResult(array(tokens), line.consumed);
}

/// A CRLF-terminated line plus total bytes consumed.
class _Line {
  final Uint8List line;
  final int consumed;
  const _Line(this.line, this.consumed);
}

_Line? _readLine(Uint8List buffer, int start) {
  for (var i = start; i < buffer.length - 1; i++) {
    if (buffer[i] == 13 && buffer[i + 1] == 10) {
      return _Line(
        Uint8List.fromList(buffer.sublist(start, i)),
        i + 2,
      );
    }
  }
  return null;
}

Uint8List _concatBytes(List<List<int>> chunks) {
  var length = 0;
  for (final chunk in chunks) {
    length += chunk.length;
  }
  final result = Uint8List(length);
  var offset = 0;
  for (final chunk in chunks) {
    result.setRange(offset, offset + chunk.length, chunk);
    offset += chunk.length;
  }
  return result;
}

Uint8List _asBytes(Object input) {
  if (input is String) {
    return Uint8List.fromList(utf8.encode(input));
  }
  return Uint8List.fromList(input as List<int>);
}
