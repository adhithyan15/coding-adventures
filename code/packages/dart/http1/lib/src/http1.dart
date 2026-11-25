/// HTTP/1 request and response head parsing: [parseRequestHead],
/// [parseResponseHead], their parsed-head results, redacted summaries, and
/// the [Http1ParseError] vocabulary.
///
/// Semantic head shapes ([RequestHead], [ResponseHead], [Header],
/// [HttpVersion], [BodyKind]) come from the in-lane `http-core` package;
/// this library owns the wire parsing that produces them.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_http_core/http_core.dart';

/// A parsed request head: the semantic head, where the body starts, and
/// how to consume it.
final class ParsedRequestHead {
  const ParsedRequestHead({
    required this.head,
    required this.bodyOffset,
    required this.bodyKind,
  });

  final RequestHead head;

  /// Byte offset into the input where the body starts.
  final int bodyOffset;

  final BodyKind bodyKind;

  /// Summarizes the parsed request without copying headers, target text,
  /// or body bytes into logs.
  Http1RequestHeadSummary summary() => Http1RequestHeadSummary(
        method: head.method,
        targetLen: utf8.encode(head.target).length,
        version: head.version,
        headerCount: head.headers.length,
        bodyOffset: bodyOffset,
        bodyKind: bodyKind,
      );

  @override
  bool operator ==(Object other) =>
      other is ParsedRequestHead &&
      other.head == head &&
      other.bodyOffset == bodyOffset &&
      other.bodyKind == bodyKind;

  @override
  int get hashCode => Object.hash(head, bodyOffset, bodyKind);

  @override
  String toString() => 'ParsedRequestHead($head, $bodyOffset, $bodyKind)';
}

/// A parsed response head: the semantic head, where the body starts, and
/// how to consume it.
final class ParsedResponseHead {
  const ParsedResponseHead({
    required this.head,
    required this.bodyOffset,
    required this.bodyKind,
  });

  final ResponseHead head;

  /// Byte offset into the input where the body starts.
  final int bodyOffset;

  final BodyKind bodyKind;

  /// Summarizes the parsed response without copying headers, reason text,
  /// or body bytes into logs.
  Http1ResponseHeadSummary summary() => Http1ResponseHeadSummary(
        version: head.version,
        status: head.status,
        reasonLen: utf8.encode(head.reason).length,
        headerCount: head.headers.length,
        bodyOffset: bodyOffset,
        bodyKind: bodyKind,
      );

  @override
  bool operator ==(Object other) =>
      other is ParsedResponseHead &&
      other.head == head &&
      other.bodyOffset == bodyOffset &&
      other.bodyKind == bodyKind;

  @override
  int get hashCode => Object.hash(head, bodyOffset, bodyKind);

  @override
  String toString() => 'ParsedResponseHead($head, $bodyOffset, $bodyKind)';
}

/// Redacted summary of a parsed HTTP/1 request head.
///
/// The request target may contain credentials or pairing tokens in local
/// API traffic, so the summary keeps only its byte length.
final class Http1RequestHeadSummary {
  const Http1RequestHeadSummary({
    required this.method,
    required this.targetLen,
    required this.version,
    required this.headerCount,
    required this.bodyOffset,
    required this.bodyKind,
  });

  final String method;

  /// Byte length of the request target (never the text itself).
  final int targetLen;
  final HttpVersion version;
  final int headerCount;
  final int bodyOffset;
  final BodyKind bodyKind;

  @override
  bool operator ==(Object other) =>
      other is Http1RequestHeadSummary &&
      other.method == method &&
      other.targetLen == targetLen &&
      other.version == version &&
      other.headerCount == headerCount &&
      other.bodyOffset == bodyOffset &&
      other.bodyKind == bodyKind;

  @override
  int get hashCode => Object.hash(
      method, targetLen, version, headerCount, bodyOffset, bodyKind);

  @override
  String toString() =>
      'Http1RequestHeadSummary($method, targetLen=$targetLen, $version, '
      'headers=$headerCount, bodyOffset=$bodyOffset, $bodyKind)';
}

/// Redacted summary of a parsed HTTP/1 response head.
final class Http1ResponseHeadSummary {
  const Http1ResponseHeadSummary({
    required this.version,
    required this.status,
    required this.reasonLen,
    required this.headerCount,
    required this.bodyOffset,
    required this.bodyKind,
  });

  final HttpVersion version;
  final int status;

  /// Byte length of the reason phrase (never the text itself).
  final int reasonLen;
  final int headerCount;
  final int bodyOffset;
  final BodyKind bodyKind;

  @override
  bool operator ==(Object other) =>
      other is Http1ResponseHeadSummary &&
      other.version == version &&
      other.status == status &&
      other.reasonLen == reasonLen &&
      other.headerCount == headerCount &&
      other.bodyOffset == bodyOffset &&
      other.bodyKind == bodyKind;

  @override
  int get hashCode => Object.hash(
      version, status, reasonLen, headerCount, bodyOffset, bodyKind);

  @override
  String toString() =>
      'Http1ResponseHeadSummary($version, $status, reasonLen=$reasonLen, '
      'headers=$headerCount, bodyOffset=$bodyOffset, $bodyKind)';
}

/// Errors thrown when HTTP/1 head parsing fails.
sealed class Http1ParseError implements Exception {
  const Http1ParseError();
}

/// The input ended before the head-terminating blank line.
final class IncompleteHead extends Http1ParseError {
  const IncompleteHead();

  @override
  bool operator ==(Object other) => other is IncompleteHead;

  @override
  int get hashCode => 0x68747470;

  @override
  String toString() => 'Http1ParseError: incomplete HTTP/1 head';
}

/// A head line was not valid UTF-8.
final class InvalidHeadEncoding extends Http1ParseError {
  const InvalidHeadEncoding();

  @override
  bool operator ==(Object other) => other is InvalidHeadEncoding;

  @override
  int get hashCode => 0x68747471;

  @override
  String toString() => 'Http1ParseError: invalid HTTP/1 head encoding';
}

/// The start line did not have the required shape.
final class InvalidStartLine extends Http1ParseError {
  const InvalidStartLine(this.line);

  /// The offending start line (empty when the head had no lines at all).
  final String line;

  @override
  bool operator ==(Object other) =>
      other is InvalidStartLine && other.line == line;

  @override
  int get hashCode => Object.hash(0x68747472, line);

  @override
  String toString() => 'Http1ParseError: invalid HTTP/1 start line: $line';
}

/// A header line had no colon or an empty name.
final class InvalidHeader extends Http1ParseError {
  const InvalidHeader(this.line);

  /// The offending header line.
  final String line;

  @override
  bool operator ==(Object other) =>
      other is InvalidHeader && other.line == line;

  @override
  int get hashCode => Object.hash(0x68747473, line);

  @override
  String toString() => 'Http1ParseError: invalid HTTP/1 header: $line';
}

/// The version marker was not `HTTP/x.y` with 16-bit parts.
final class InvalidVersion extends Http1ParseError {
  const InvalidVersion(this.value);

  /// The offending version text.
  final String value;

  @override
  bool operator ==(Object other) =>
      other is InvalidVersion && other.value == value;

  @override
  int get hashCode => Object.hash(0x68747474, value);

  @override
  String toString() => 'Http1ParseError: invalid HTTP version: $value';
}

/// The status code was not ASCII digits fitting in 16 bits.
final class InvalidStatus extends Http1ParseError {
  const InvalidStatus(this.value);

  /// The offending status text.
  final String value;

  @override
  bool operator ==(Object other) =>
      other is InvalidStatus && other.value == value;

  @override
  int get hashCode => Object.hash(0x68747475, value);

  @override
  String toString() => 'Http1ParseError: invalid HTTP status: $value';
}

/// The Content-Length value was not ASCII digits fitting in 64 bits.
final class InvalidContentLength extends Http1ParseError {
  const InvalidContentLength(this.value);

  /// The offending Content-Length text.
  final String value;

  @override
  bool operator ==(Object other) =>
      other is InvalidContentLength && other.value == value;

  @override
  int get hashCode => Object.hash(0x68747476, value);

  @override
  String toString() => 'Http1ParseError: invalid Content-Length: $value';
}

// ── Entry points ─────────────────────────────────────────────────────────

/// Parses an HTTP/1 request head: `METHOD target HTTP/x.y` plus headers.
///
/// Returns the semantic head, the body offset, and the framing decision.
/// Throws [Http1ParseError] on any malformed input.
ParsedRequestHead parseRequestHead(List<int> input) {
  final head = _splitHeadLines(input);
  // Defensive: unreachable, since the blank-line skip guarantees the
  // first content line holds a byte (all-blank input fails earlier with
  // IncompleteHead). Kept for reference parity.
  if (head.lines.isEmpty) {
    throw const InvalidStartLine('');
  }
  final startLine = _decodeLine(head.lines.first);
  final parts = _splitWhiteSpace(startLine);
  if (parts.length != 3) {
    throw InvalidStartLine(startLine);
  }
  final method = parts[0];
  final target = parts[1];
  final versionText = parts[2];

  final HttpVersion version;
  try {
    version = HttpVersion.parse(versionText);
  } on InvalidHttpVersion {
    throw InvalidVersion(versionText);
  }
  final headers = _parseHeaders(head.lines.sublist(1));
  final bodyKind = _requestBodyKind(headers);

  return ParsedRequestHead(
    head: RequestHead(
      method: method,
      target: target,
      version: version,
      headers: headers,
    ),
    bodyOffset: head.bodyOffset,
    bodyKind: bodyKind,
  );
}

/// Parses an HTTP/1 response head: `HTTP/x.y STATUS reason...` plus
/// headers.
///
/// The reason phrase rejoins any extra pieces with single spaces, so
/// interior whitespace runs normalize. Throws [Http1ParseError] on any
/// malformed input.
ParsedResponseHead parseResponseHead(List<int> input) {
  final head = _splitHeadLines(input);
  // Defensive: unreachable, like the request-side twin above. Kept for
  // reference parity.
  if (head.lines.isEmpty) {
    throw const InvalidStartLine('');
  }
  final statusLine = _decodeLine(head.lines.first);
  final pieces = _splitWhiteSpace(statusLine);
  if (pieces.length < 2) {
    throw InvalidStartLine(statusLine);
  }

  final HttpVersion version;
  try {
    version = HttpVersion.parse(pieces[0]);
  } on InvalidHttpVersion {
    throw InvalidVersion(pieces[0]);
  }
  final status = _parseU16(pieces[1]);
  if (status == null) {
    throw InvalidStatus(pieces[1]);
  }
  final reason = pieces.length > 2 ? pieces.sublist(2).join(' ') : '';

  final headers = _parseHeaders(head.lines.sublist(1));
  final bodyKind = _responseBodyKind(status, headers);

  return ParsedResponseHead(
    head: ResponseHead(
      version: version,
      status: status,
      reason: reason,
      headers: headers,
    ),
    bodyOffset: head.bodyOffset,
    bodyKind: bodyKind,
  );
}

// ── Unicode whitespace ───────────────────────────────────────────────────
//
// The reference splits and trims on the Unicode White_Space property.
// Dart's `\s` misses NEL (U+0085) and wrongly splits the BOM (U+FEFF);
// `String.trim` wrongly strips the BOM. Since this parser preserves a
// leading BOM as U+FEFF (like `from_utf8`), using either would misparse
// a BOM-prefixed start line — so the exact set is spelled out once,
// here.

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

/// Splits on whitespace runs, yielding no empty pieces.
///
/// All White_Space members are BMP characters, so scanning UTF-16 code
/// units is safe: surrogate halves never match and pass through inside
/// their piece.
List<String> _splitWhiteSpace(String text) {
  final parts = <String>[];
  var start = 0;
  var i = 0;
  while (i < text.length) {
    if (_isWhiteSpace(text.codeUnitAt(i))) {
      if (i > start) {
        parts.add(text.substring(start, i));
      }
      i++;
      start = i;
    } else {
      i++;
    }
  }
  if (i > start) {
    parts.add(text.substring(start, i));
  }
  return parts;
}

/// Trims leading and trailing White_Space, mirroring `str::trim`.
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

// ── Head splitting ───────────────────────────────────────────────────────

/// The head's raw lines plus the body's starting offset.
final class _SplitHead {
  const _SplitHead(this.lines, this.bodyOffset);

  /// Raw head lines without line terminators (`\r` stripped).
  final List<Uint8List> lines;
  final int bodyOffset;
}

/// Splits [input] into head lines and finds the body offset.
///
/// Leading blank lines (`\r\n` or bare `\n`) are skipped; lines end at
/// `\n` with a trailing `\r` stripped; the head ends at the first empty
/// line, and the body starts after its terminator. Throws
/// [IncompleteHead] when the input ends mid-head.
_SplitHead _splitHeadLines(List<int> input) {
  final bytes = input is Uint8List ? input : Uint8List.fromList(input);
  var index = 0;
  while (index < bytes.length) {
    if (index + 1 < bytes.length &&
        bytes[index] == 0x0D &&
        bytes[index + 1] == 0x0A) {
      index += 2;
    } else if (bytes[index] == 0x0A) {
      index += 1;
    } else {
      break;
    }
  }

  final lines = <Uint8List>[];
  while (true) {
    if (index >= bytes.length) {
      throw const IncompleteHead();
    }
    final lineStart = index;
    while (index < bytes.length && bytes[index] != 0x0A) {
      index++;
    }
    if (index >= bytes.length) {
      throw const IncompleteHead();
    }
    final lineEnd =
        (index > lineStart && bytes[index - 1] == 0x0D) ? index - 1 : index;
    final line = bytes.sublist(lineStart, lineEnd);
    index++;

    if (line.isEmpty) {
      return _SplitHead(lines, index);
    }
    lines.add(line);
  }
}

// ── Headers ──────────────────────────────────────────────────────────────

/// Decodes one raw head line as strict UTF-8.
///
/// `utf8.decode` strips a leading BOM, while the reference's `from_utf8`
/// keeps it as U+FEFF — so a leading BOM is decoded explicitly and
/// prepended. Anything else malformed throws [InvalidHeadEncoding].
String _decodeLine(Uint8List line) {
  try {
    if (line.length >= 3 &&
        line[0] == 0xEF &&
        line[1] == 0xBB &&
        line[2] == 0xBF) {
      return '\uFEFF${utf8.decode(line.sublist(3), allowMalformed: false)}';
    }
    return utf8.decode(line, allowMalformed: false);
  } on FormatException {
    throw const InvalidHeadEncoding();
  }
}

/// Parses header lines into [Header]s.
///
/// Each line splits at its first colon; the name trims (Unicode
/// whitespace, like the reference) and must be non-empty; the value
/// trims spaces and tabs only.
List<Header> _parseHeaders(List<Uint8List> lines) {
  final headers = <Header>[];
  for (final line in lines) {
    final text = _decodeLine(line);
    final colon = text.indexOf(':');
    if (colon == -1) {
      throw InvalidHeader(text);
    }
    final name = _trimWhiteSpace(text.substring(0, colon));
    if (name.isEmpty) {
      throw InvalidHeader(text);
    }
    headers.add(Header(name, _trimSpacesAndTabs(text.substring(colon + 1))));
  }
  return headers;
}

/// Trims leading and trailing spaces and horizontal tabs, mirroring the
/// reference's `trim_matches([' ', '\t'])`.
String _trimSpacesAndTabs(String text) {
  var start = 0;
  var end = text.length;
  while (start < end &&
      (text.codeUnitAt(start) == 0x20 || text.codeUnitAt(start) == 0x09)) {
    start++;
  }
  while (end > start &&
      (text.codeUnitAt(end - 1) == 0x20 || text.codeUnitAt(end - 1) == 0x09)) {
    end--;
  }
  return text.substring(start, end);
}

// ── Body framing ─────────────────────────────────────────────────────────

BodyKind _requestBodyKind(List<Header> headers) {
  if (_hasChunkedTransferEncoding(headers)) {
    return const BodyChunked();
  }
  final length = _declaredContentLength(headers);
  if (length == null || length == 0) {
    return const BodyNone();
  }
  return BodyContentLength(length);
}

BodyKind _responseBodyKind(int status, List<Header> headers) {
  // Bodyless statuses win over every header, including chunked.
  if ((status >= 100 && status < 200) || status == 204 || status == 304) {
    return const BodyNone();
  }
  if (_hasChunkedTransferEncoding(headers)) {
    return const BodyChunked();
  }
  final length = _declaredContentLength(headers);
  if (length == null) {
    return const BodyUntilEof();
  }
  if (length == 0) {
    return const BodyNone();
  }
  return BodyContentLength(length);
}

/// The first Content-Length value when any header carries that name
/// (ASCII case-insensitive), or `null` when none does.
int? _declaredContentLength(List<Header> headers) {
  for (final header in headers) {
    if (_asciiEqualsIgnoreCase(header.name, 'Content-Length')) {
      final length = _parseUsize(header.value);
      if (length == null) {
        throw InvalidContentLength(header.value);
      }
      return length;
    }
  }
  return null;
}

/// Whether any Transfer-Encoding header lists the `chunked` coding
/// (comma-separated, ASCII case-insensitive, surrounding whitespace
/// ignored).
bool _hasChunkedTransferEncoding(List<Header> headers) {
  for (final header in headers) {
    if (!_asciiEqualsIgnoreCase(header.name, 'Transfer-Encoding')) {
      continue;
    }
    for (final coding in header.value.split(',')) {
      if (_asciiEqualsIgnoreCase(_trimWhiteSpace(coding), 'chunked')) {
        return true;
      }
    }
  }
  return false;
}

/// ASCII-only case-insensitive comparison, mirroring the reference's
/// `eq_ignore_ascii_case` exactly (Unicode lowercasing would equate
/// characters like the Kelvin sign with `k`).
bool _asciiEqualsIgnoreCase(String a, String b) {
  if (a.length != b.length) {
    return false;
  }
  for (var i = 0; i < a.length; i++) {
    var x = a.codeUnitAt(i);
    var y = b.codeUnitAt(i);
    if (x >= 0x41 && x <= 0x5A) x += 0x20;
    if (y >= 0x41 && y <= 0x5A) y += 0x20;
    if (x != y) {
      return false;
    }
  }
  return true;
}

/// Parses ASCII digits as u16, mirroring Rust's unsigned parse exactly:
/// no signs, no whitespace, no empty strings, nothing past 65535.
int? _parseU16(String text) {
  if (text.isEmpty || text.length > 5) {
    return null;
  }
  for (var i = 0; i < text.length; i++) {
    final c = text.codeUnitAt(i);
    if (c < 0x30 || c > 0x39) {
      return null;
    }
  }
  final value = int.parse(text);
  return value <= 65535 ? value : null;
}

/// Parses ASCII digits as a body length, mirroring Rust's `usize` parse.
///
/// One platform limit applies: Dart integers are signed 64-bit, so
/// lengths past 2^63 - 1 are rejected where a 64-bit reference would
/// accept them up to 2^64 - 1. No real framing decision changes — a
/// multi-exabyte Content-Length cannot describe bytes on hand either
/// way — and the rejection carries the same error.
int? _parseUsize(String text) {
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
