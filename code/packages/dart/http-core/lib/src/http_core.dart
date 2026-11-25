/// Shared HTTP message types and helpers.
///
/// Version-specific parsers disagree about wire syntax, but they
/// should agree about the semantic shapes that application code
/// consumes: headers, versions, request heads, response heads, and
/// body framing hints, plus route patterns over request targets.
///
/// ## Port notes
///
/// Rust unsigned parsing (`u16`/`usize`) rejects signs, whitespace,
/// and overflow. Dart's [int.parse] accepts a leading `+`/`-`, so
/// numeric fields validate ASCII-digits-only before parsing:
/// version parts must additionally fit 16 bits (exactly mirroring
/// the reference), while content lengths must fit a Dart int —
/// values at or above 2^63 return null where the reference would
/// return `Some` on 64-bit targets (documented, boundary-tested).
/// Header lookup folds ASCII case only, never full Unicode.
library;

import 'errors.dart';

/// Package version, mirroring the reference crate.
const String httpCoreVersion = '0.1.0';

/// A parsed route segment used for path matching.
sealed class RouteSegment {
  const RouteSegment();
}

/// A literal segment that must match exactly.
final class RouteLiteral extends RouteSegment {
  final String value;
  const RouteLiteral(this.value);

  @override
  bool operator ==(Object other) =>
      other is RouteLiteral && value == other.value;
  @override
  int get hashCode => value.hashCode;
  @override
  String toString() => 'RouteLiteral($value)';
}

/// A `:name` parameter capturing its segment.
final class RouteParam extends RouteSegment {
  final String name;
  const RouteParam(this.name);

  @override
  bool operator ==(Object other) => other is RouteParam && name == other.name;
  @override
  int get hashCode => name.hashCode;
  @override
  String toString() => 'RouteParam($name)';
}

/// A generic HTTP path pattern such as `/hello/:name`.
class RoutePattern {
  final List<RouteSegment> segments;

  const RoutePattern(this.segments);

  /// Parses a pattern: segments starting with `:` become params
  /// (an empty name is allowed, mirroring the reference).
  factory RoutePattern.parse(String pattern) => RoutePattern(
        splitPathSegments(pattern)
            .map((segment) => segment.startsWith(':')
                ? RouteParam(segment.substring(1))
                : RouteLiteral(segment))
            .toList(),
      );

  /// Matches a path, returning captured `(name, value)` pairs, or
  /// null when the segment count differs or a literal mismatches.
  List<(String, String)>? matchPath(String path) {
    final pathSegments = splitPathSegments(path);
    if (pathSegments.length != segments.length) {
      return null;
    }
    final params = <(String, String)>[];
    for (var i = 0; i < segments.length; i++) {
      final segment = segments[i];
      final actual = pathSegments[i];
      switch (segment) {
        case RouteLiteral(value: final expected) when expected == actual:
          break;
        case RouteLiteral():
          return null;
        case RouteParam(name: final name):
          params.add((name, actual));
      }
    }
    return params;
  }

  /// Matches a full request target such as
  /// `/clip/v2/resource?limit=10`: only the path portion counts, so
  /// query strings cannot make an otherwise-valid route miss.
  List<(String, String)>? matchTarget(String target) =>
      matchPath(parseRequestTarget(target).path);

  @override
  bool operator ==(Object other) =>
      other is RoutePattern && _equalSegments(segments, other.segments);
  @override
  int get hashCode => Object.hashAll(segments);
  @override
  String toString() => 'RoutePattern($segments)';
}

bool _equalSegments(List<RouteSegment> a, List<RouteSegment> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// One HTTP header line, preserved in arrival order.
class Header {
  final String name;
  final String value;

  const Header(this.name, this.value);

  @override
  bool operator ==(Object other) =>
      other is Header && name == other.name && value == other.value;
  @override
  int get hashCode => Object.hash(name, value);
  @override
  String toString() => 'Header($name, $value)';
}

/// A parsed origin-form request target.
class RequestTarget {
  final String path;
  final String? query;
  final String? fragment;

  const RequestTarget({required this.path, this.query, this.fragment});

  /// Raw `name=value` pairs in the query string. Values are
  /// intentionally NOT percent-decoded: this package is a
  /// syntax-level core feeding callers with different decoding
  /// policies. Empty pieces are skipped; a piece without `=`
  /// yields an empty value.
  Iterable<(String, String)> queryPairs() sync* {
    var rest = query ?? '';
    while (rest.isNotEmpty) {
      final amp = rest.indexOf('&');
      final String piece;
      if (amp == -1) {
        piece = rest;
        rest = '';
      } else {
        piece = rest.substring(0, amp);
        rest = rest.substring(amp + 1);
      }
      if (piece.isEmpty) {
        continue;
      }
      final eq = piece.indexOf('=');
      if (eq == -1) {
        yield (piece, '');
      } else {
        yield (piece.substring(0, eq), piece.substring(eq + 1));
      }
    }
  }

  /// The first value for [name], or null when absent.
  String? queryValue(String name) {
    for (final (candidate, value) in queryPairs()) {
      if (candidate == name) {
        return value;
      }
    }
    return null;
  }

  @override
  bool operator ==(Object other) =>
      other is RequestTarget &&
      path == other.path &&
      query == other.query &&
      fragment == other.fragment;
  @override
  int get hashCode => Object.hash(path, query, fragment);
  @override
  String toString() => 'RequestTarget($path, $query, $fragment)';
}

/// A semantic HTTP version.
class HttpVersion {
  final int major;
  final int minor;

  const HttpVersion({required this.major, required this.minor});

  /// Parses a textual `HTTP/x.y` marker with 16-bit parts.
  ///
  /// Throws [InvalidHttpVersion] unless the shape is exact.
  factory HttpVersion.parse(String text) {
    final fail = InvalidHttpVersion(text);
    if (!text.startsWith('HTTP/')) {
      throw fail;
    }
    final rest = text.substring(5);
    final dot = rest.indexOf('.');
    if (dot == -1) {
      throw fail;
    }
    final major = _parseU16(rest.substring(0, dot));
    final minor = _parseU16(rest.substring(dot + 1));
    if (major == null || minor == null) {
      throw fail;
    }
    return HttpVersion(major: major, minor: minor);
  }

  @override
  bool operator ==(Object other) =>
      other is HttpVersion && major == other.major && minor == other.minor;
  @override
  int get hashCode => Object.hash(major, minor);
  @override
  String toString() => 'HTTP/$major.$minor';
}

/// Parses ASCII digits as u16, mirroring Rust's unsigned parse
/// exactly (no signs, no whitespace, no overflow past 65535).
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

/// Describes how a caller should consume the payload bytes.
sealed class BodyKind {
  const BodyKind();
}

/// No payload.
final class BodyNone extends BodyKind {
  const BodyNone();

  @override
  bool operator ==(Object other) => other is BodyNone;
  @override
  int get hashCode => 0;
  @override
  String toString() => 'BodyNone()';
}

/// Exactly [length] payload bytes.
final class BodyContentLength extends BodyKind {
  final int length;
  const BodyContentLength(this.length);

  @override
  bool operator ==(Object other) =>
      other is BodyContentLength && length == other.length;
  @override
  int get hashCode => length.hashCode;
  @override
  String toString() => 'BodyContentLength($length)';
}

/// Payload runs until EOF.
final class BodyUntilEof extends BodyKind {
  const BodyUntilEof();

  @override
  bool operator ==(Object other) => other is BodyUntilEof;
  @override
  int get hashCode => 1;
  @override
  String toString() => 'BodyUntilEof()';
}

/// Chunked transfer encoding.
final class BodyChunked extends BodyKind {
  const BodyChunked();

  @override
  bool operator ==(Object other) => other is BodyChunked;
  @override
  int get hashCode => 2;
  @override
  String toString() => 'BodyChunked()';
}

/// The semantic shape of a request head.
class RequestHead {
  final String method;
  final String target;
  final HttpVersion version;
  final List<Header> headers;

  const RequestHead({
    required this.method,
    required this.target,
    required this.version,
    required this.headers,
  });

  String? header(String name) => findHeader(headers, name);

  RequestTarget targetParts() => parseRequestTarget(target);

  String path() => targetParts().path;

  String? queryValue(String name) => targetParts().queryValue(name);

  int? contentLength() => parseContentLength(headers);

  (String, String?)? contentType() => parseContentType(headers);

  @override
  bool operator ==(Object other) =>
      other is RequestHead &&
      method == other.method &&
      target == other.target &&
      version == other.version &&
      _equalHeaders(headers, other.headers);
  @override
  int get hashCode =>
      Object.hash(method, target, version, Object.hashAll(headers));
  @override
  String toString() => 'RequestHead($method, $target, $version, $headers)';
}

/// The semantic shape of a response head.
class ResponseHead {
  final HttpVersion version;
  final int status;
  final String reason;
  final List<Header> headers;

  const ResponseHead({
    required this.version,
    required this.status,
    required this.reason,
    required this.headers,
  });

  String? header(String name) => findHeader(headers, name);

  int? contentLength() => parseContentLength(headers);

  (String, String?)? contentType() => parseContentType(headers);

  @override
  bool operator ==(Object other) =>
      other is ResponseHead &&
      version == other.version &&
      status == other.status &&
      reason == other.reason &&
      _equalHeaders(headers, other.headers);
  @override
  int get hashCode =>
      Object.hash(version, status, reason, Object.hashAll(headers));
  @override
  String toString() => 'ResponseHead($version, $status, $reason, $headers)';
}

bool _equalHeaders(List<Header> a, List<Header> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

bool _asciiEqualsIgnoreCase(String a, String b) {
  if (a.length != b.length) {
    return false;
  }
  for (var i = 0; i < a.length; i++) {
    var x = a.codeUnitAt(i);
    var y = b.codeUnitAt(i);
    // Fold ASCII uppercase to lowercase only (never full Unicode).
    if (x >= 0x41 && x <= 0x5a) {
      x |= 0x20;
    }
    if (y >= 0x41 && y <= 0x5a) {
      y |= 0x20;
    }
    if (x != y) {
      return false;
    }
  }
  return true;
}

/// Returns the first matching header value using ASCII
/// case-insensitive lookup.
String? findHeader(List<Header> headers, String name) {
  for (final header in headers) {
    if (_asciiEqualsIgnoreCase(header.name, name)) {
      return header.value;
    }
  }
  return null;
}

/// Parses `Content-Length` when present and valid (ASCII digits
/// fitting a Dart int; values at or above 2^63 return null).
int? parseContentLength(List<Header> headers) {
  final value = findHeader(headers, 'Content-Length');
  if (value == null || value.isEmpty) {
    return null;
  }
  for (var i = 0; i < value.length; i++) {
    final c = value.codeUnitAt(i);
    if (c < 0x30 || c > 0x39) {
      return null;
    }
  }
  try {
    return int.parse(value);
  } on FormatException {
    return null;
  }
}

/// Splits `Content-Type` into media type and optional charset.
(String, String?)? parseContentType(List<Header> headers) {
  final value = findHeader(headers, 'Content-Type');
  if (value == null) {
    return null;
  }
  final pieces = value.split(';').map((s) => s.trim()).toList();
  final mediaType = pieces.first;
  if (mediaType.isEmpty) {
    return null;
  }
  String? charset;
  for (final piece in pieces.skip(1)) {
    final eq = piece.indexOf('=');
    if (eq == -1) {
      continue;
    }
    if (_asciiEqualsIgnoreCase(piece.substring(0, eq).trim(), 'charset')) {
      charset = _trimQuotes(piece.substring(eq + 1).trim());
      break;
    }
  }
  return (mediaType, charset);
}

String _trimQuotes(String text) {
  var start = 0;
  var end = text.length;
  while (start < end && text.codeUnitAt(start) == 0x22) {
    start++;
  }
  while (end > start && text.codeUnitAt(end - 1) == 0x22) {
    end--;
  }
  return text.substring(start, end);
}

/// Splits an origin-form HTTP request target into path, query, and
/// fragment. An empty path becomes `/`.
RequestTarget parseRequestTarget(String target) {
  final hash = target.indexOf('#');
  final String beforeFragment;
  final String? fragment;
  if (hash == -1) {
    beforeFragment = target;
    fragment = null;
  } else {
    beforeFragment = target.substring(0, hash);
    fragment = target.substring(hash + 1);
  }
  final q = beforeFragment.indexOf('?');
  final String path;
  final String? query;
  if (q == -1) {
    path = beforeFragment;
    query = null;
  } else {
    path = beforeFragment.substring(0, q);
    query = beforeFragment.substring(q + 1);
  }
  return RequestTarget(
    path: path.isEmpty ? '/' : path,
    query: query,
    fragment: fragment,
  );
}

/// Splits an HTTP path or route pattern into slash-delimited
/// segments, dropping empties (`/` yields none).
List<String> splitPathSegments(String path) {
  if (path == '/') {
    return [];
  }
  return path.split('/').where((segment) => segment.isNotEmpty).toList();
}
