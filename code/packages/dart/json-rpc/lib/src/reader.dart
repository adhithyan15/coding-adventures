/// [MessageReader] — reads Content-Length-framed JSON-RPC messages from
/// an in-memory byte buffer.
///
/// Each message is preceded by an HTTP-style header block:
///
/// ```text
/// Content-Length: 47\r\n
/// \r\n
/// {"jsonrpc":"2.0","id":1,"method":"initialize"}
/// ```
///
/// The reader consumes the buffer through a cursor: [readMessage] returns
/// the next parsed message, or `null` at clean EOF, and throws
/// [ResponseError] on framing or parse failures. A stream-backed reader
/// for live stdio is future work for the LSP-server lane; the framing
/// behavior here is identical for buffered input.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'errors.dart';
import 'message.dart';

/// Reads framed JSON-RPC messages from bytes.
///
/// Construct with the full input; each [readMessage]/[readRaw] call
/// advances past one frame.
final class MessageReader {
  MessageReader(List<int> bytes)
      : _bytes = bytes is Uint8List ? bytes : Uint8List.fromList(bytes),
        _cursor = 0;

  final Uint8List _bytes;
  int _cursor;

  /// Bytes consumed so far (the cursor position).
  int get bytesConsumed => _cursor;

  /// Whether the cursor reached the end of the buffer.
  bool get isAtEnd => _cursor >= _bytes.length;

  /// Reads and parses the next framed message.
  ///
  /// Returns `null` on clean EOF. Throws [ResponseError] on framing or
  /// parse errors.
  Message? readMessage() {
    final raw = readRaw();
    if (raw == null) return null;
    return parseMessage(raw);
  }

  /// Reads the next raw JSON payload bytes without parsing them.
  ///
  /// Returns `null` on clean EOF. Throws [ResponseError] on framing
  /// errors.
  Uint8List? readRaw() {
    final contentLength = _readHeaders();
    if (contentLength == null) return null;
    // Subtracted form: `_cursor + contentLength` would wrap negative for
    // hostile near-int64 lengths and bypass the check into a RangeError.
    // (`_cursor <= length` always holds, so the subtraction cannot
    // underflow.)
    if (contentLength > _bytes.length - _cursor) {
      // Like `read_exact`, the short bytes are consumed even though the
      // read fails: the next call must see EOF, not a retry.
      _cursor = _bytes.length;
      throw ResponseError.parseError(
          'I/O error reading payload: failed to fill whole buffer');
    }
    final payload = _bytes.sublist(_cursor, _cursor + contentLength);
    _cursor += contentLength;
    return payload;
  }

  /// Reads header lines to the blank terminator, returning the declared
  /// content length — or `null` on clean EOF before any header line.
  ///
  /// Throws [ResponseError] when EOF lands mid-headers, no Content-Length
  /// appears, its value is malformed, or a header line is not UTF-8.
  int? _readHeaders() {
    int? contentLength;
    var firstLine = true;
    while (true) {
      final line = _readLine();
      if (line == null) {
        if (firstLine) return null;
        throw ResponseError.parseError('EOF in header block');
      }
      firstLine = false;

      final String text;
      try {
        text = _decodeHeaderLine(line);
      } on FormatException {
        throw ResponseError.parseError(
            'I/O error reading header: stream did not contain valid UTF-8');
      }
      // Strip the terminator the way `trim_end_matches` does: the one
      // trailing `\n` a line can hold, then every trailing `\r`. (Yes,
      // that makes even `"\r\r\n"` a blank line — reference behavior,
      // pinned by tests.)
      var stripped = text;
      if (stripped.endsWith('\n')) {
        stripped = stripped.substring(0, stripped.length - 1);
      }
      // One substring, not one per `\r`: the naive loop is quadratic on
      // hostile `\r` runs.
      var end = stripped.length;
      while (end > 0 && stripped.codeUnitAt(end - 1) == 0x0D) {
        end--;
      }
      stripped = stripped.substring(0, end);
      if (stripped.isEmpty) {
        if (contentLength == null) {
          throw ResponseError.parseError('missing Content-Length header');
        }
        return contentLength;
      }

      // Case-sensitive with the space, per the LSP spec: anything else
      // (including `Content-Length:5`) is some other header and ignored.
      const prefix = 'Content-Length: ';
      if (stripped.startsWith(prefix)) {
        final valueStr = stripped.substring(prefix.length);
        final length = _parseLength(_trimWhiteSpace(valueStr));
        if (length == null) {
          throw ResponseError.parseError(
              "invalid Content-Length value: '$valueStr'");
        }
        contentLength = length;
      }
    }
  }

  /// Reads one line up to and including `\n`, without the terminator
  /// handling — or `null` at EOF with no bytes left.
  ///
  /// A final unterminated line still yields its bytes (like `read_line`);
  /// only a fully-consumed buffer reports EOF.
  Uint8List? _readLine() {
    if (_cursor >= _bytes.length) return null;
    var end = _cursor;
    while (end < _bytes.length && _bytes[end] != 0x0A) {
      end++;
    }
    final lineEnd = end < _bytes.length ? end + 1 : end;
    final line = _bytes.sublist(_cursor, lineEnd);
    _cursor = lineEnd;
    return line;
  }
}

/// Decodes one header line as strict UTF-8, preserving a leading BOM as
/// U+FEFF the way `read_line` into a `String` does (`utf8.decode` would
/// strip it, which would wrongly accept a BOM-prefixed Content-Length).
String _decodeHeaderLine(Uint8List line) {
  if (line.length >= 3 &&
      line[0] == 0xEF &&
      line[1] == 0xBB &&
      line[2] == 0xBF) {
    return '\uFEFF${utf8.decode(line.sublist(3), allowMalformed: false)}';
  }
  return utf8.decode(line, allowMalformed: false);
}

/// Trims Unicode White_Space, mirroring `str::trim`.
///
/// (A twin of the http1 lane's helper: Dart's `trim` wrongly strips the
/// BOM, which matters nowhere here but costs nothing to get right.)
String _trimWhiteSpace(String text) {
  var start = 0;
  var end = text.length;
  while (start < end && _isWhiteSpace(text.codeUnitAt(start))) {
    start++;
  }
  while (end > start && _isWhiteSpace(text.codeUnitAt(end - 1))) {
    end--;
  }
  return text.substring(start, end);
}

/// Whether [c] has the Unicode White_Space property.
bool _isWhiteSpace(int c) =>
    (c >= 0x09 && c <= 0x0D) ||
    c == 0x20 ||
    c == 0x85 ||
    c == 0xA0 ||
    c == 0x1680 ||
    (c >= 0x2000 && c <= 0x200A) ||
    c == 0x2028 ||
    c == 0x2029 ||
    c == 0x202F ||
    c == 0x205F ||
    c == 0x3000;

/// Parses ASCII digits as a content length, mirroring Rust's `usize`
/// parse: no signs, no whitespace, no empties, nothing past 2^63 - 1
/// (Dart integers are signed 64-bit; larger values fail closed with the
/// same error the reference would raise for malformed input).
int? _parseLength(String text) {
  if (text.isEmpty) {
    return null;
  }
  for (var i = 0; i < text.length; i++) {
    final c = text.codeUnitAt(i);
    if (c < 0x30 || c > 0x39) {
      return null;
    }
  }
  try {
    return int.parse(text);
  } on FormatException {
    return null;
  }
}
