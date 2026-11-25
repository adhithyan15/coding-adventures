/// URL parsing, resolution, and canonicalization.
///
/// The Dart lane of the Rust `url-parser` crate (spec NET00): [Url.parse]
/// splits absolute URLs into components, [Url.resolve] implements RFC 1808
/// relative resolution, [Url.canonicalize] normalizes identity,
/// [percentEncode]/[percentDecode] convert percent-coding, and [UrlError]
/// reports the six failure modes.
///
/// Components store raw substrings (NOT percent-decoded — decoding is
/// opt-in via [percentDecode], exactly as in the reference, despite the
/// spec's "percent-decoded" wording). The fragment split uses the FIRST
/// `#`, also as the reference implements it (the spec text says "from the
/// right", which would mis-split `?a=1#b#c`).
library;

import 'dart:convert';

/// Failures from URL parsing, resolution, and decoding.
///
/// Thrown (never returned) by [Url.parse], [Url.resolve],
/// [Url.canonicalize], and [percentDecode]. Two values — [emptyHost] and
/// [relativeWithoutBase] — exist for API parity with the reference but are
/// never thrown by it or by this port (the reference defines but never
/// constructs them either).
enum UrlError implements Exception {
  /// No scheme found (e.g. `www.example.com` without `http://`).
  missingScheme("missing scheme (expected '://')"),

  /// Scheme fails `[a-z][a-z0-9+.-]*` (after lowercasing).
  invalidScheme('invalid scheme (must be [a-z][a-z0-9+.-]*)'),

  /// Port is not 0-65535 (e.g. `http://host:99999`).
  invalidPort('invalid port (must be 0-65535)'),

  /// Malformed percent-encoding (`%GG`, truncated `%2`, bad UTF-8).
  invalidPercentEncoding('malformed percent-encoding'),

  /// Empty host in an authority-based URL (`http:///path`).
  ///
  /// Defined for parity; never thrown (an empty host parses to null).
  emptyHost('empty host in authority-based URL'),

  /// Relative URL cannot be resolved without a base.
  ///
  /// Defined for parity; never thrown ([Url.resolve] always has a base).
  relativeWithoutBase('relative URL requires a base URL');

  /// Human-readable message.
  final String message;
  const UrlError(this.message);

  @override
  String toString() => message;
}

/// A parsed URL with all components separated.
///
/// Invariants (mirroring the reference):
///
/// * [scheme] is always lowercased;
/// * [host] is always lowercased (when present);
/// * [path] starts with `/` for authority-based URLs;
/// * [query] excludes the leading `?`, [fragment] the leading `#`;
/// * fields store raw substrings (see the library docs).
///
/// Equality compares every field including the preserved input (mirroring
/// the reference's derived equality, which covers its private `raw` too).
class Url {
  /// The scheme (protocol), lowercased. Examples: `http`, `ftp`, `mailto`.
  final String scheme;

  /// Optional userinfo before the `@`. Example: `alice:secret`.
  final String? userinfo;

  /// Optional host, lowercased. Example: `www.example.com`.
  final String? host;

  /// Optional explicit port. Null means the scheme default.
  final int? port;

  /// The path component. Starts with `/` for authority-based URLs.
  final String path;

  /// Optional query string, without the leading `?`.
  final String? query;

  /// Optional fragment identifier, without the leading `#`.
  final String? fragment;

  /// The original input string, preserved verbatim (untrimmed).
  final String _raw;

  const Url._({
    required this.scheme,
    required this.userinfo,
    required this.host,
    required this.port,
    required this.path,
    required this.query,
    required this.fragment,
    required String raw,
  }) : _raw = raw;

  /// Parses an absolute URL string.
  ///
  /// Accepts `scheme://...` form (with authority) and `scheme:path` form
  /// (e.g. `mailto:alice@example.com`, no authority). Surrounding
  /// whitespace is trimmed for parsing but preserved in the stored input.
  /// Throws [UrlError] on any malformed input.
  factory Url.parse(String input) {
    final raw = input;
    input = input.trim();

    // Step 1: extract the scheme at "://", else the "scheme:path" form.
    final schemeEnd = input.indexOf('://');
    late final String scheme;
    late final String afterScheme;
    if (schemeEnd >= 0) {
      scheme = input.substring(0, schemeEnd).toLowerCase();
      _validateScheme(scheme);
      afterScheme = input.substring(schemeEnd + 3);
    } else {
      final colon = input.indexOf(':');
      if (colon > 0 && !input.substring(0, colon).contains('/')) {
        scheme = input.substring(0, colon).toLowerCase();
        _validateScheme(scheme);
        // No authority — the rest is the path (still split for # and ?).
        final pathAndFragment = _splitFragment(input.substring(colon + 1));
        final pathAndQuery = _splitQuery(pathAndFragment.$1);
        return Url._(
          scheme: scheme,
          userinfo: null,
          host: null,
          port: null,
          path: pathAndQuery.$1,
          query: pathAndQuery.$2,
          fragment: pathAndFragment.$2,
          raw: raw,
        );
      }
      throw UrlError.missingScheme;
    }

    // Step 2: fragment at the first '#'. Step 3: query at the first '?'.
    final afterFragment = _splitFragment(afterScheme);
    final afterQuery = _splitQuery(afterFragment.$1);

    // Step 4: authority before the first '/', else the whole rest with a
    // default "/" path.
    final rest = afterQuery.$1;
    final slash = rest.indexOf('/');
    final String authorityStr;
    final String path;
    if (slash >= 0) {
      authorityStr = rest.substring(0, slash);
      path = rest.substring(slash);
    } else {
      authorityStr = rest;
      path = '/';
    }

    // Step 5: userinfo before the last '@'.
    final at = authorityStr.lastIndexOf('@');
    final String? userinfo;
    final String hostPort;
    if (at >= 0) {
      userinfo = authorityStr.substring(0, at);
      hostPort = authorityStr.substring(at + 1);
    } else {
      userinfo = null;
      hostPort = authorityStr;
    }

    // Steps 6-7: port and host. IPv6 hosts stay bracketed; the port
    // delimiter is the ':' after ']'. For other hosts the last ':' splits
    // only when followed by non-empty all-digit text — otherwise (including
    // a trailing "host:" colon) the whole string stays the host.
    final String hostStr;
    final int? port;
    if (hostPort.startsWith('[')) {
      final bracket = hostPort.indexOf(']');
      if (bracket < 0) {
        // Malformed IPv6: the whole thing is the host.
        hostStr = hostPort;
        port = null;
      } else {
        hostStr = hostPort.substring(0, bracket + 1);
        final afterBracket = hostPort.substring(bracket + 1);
        port = afterBracket.startsWith(':')
            ? _parsePort(afterBracket.substring(1))
            : null;
      }
    } else {
      final colon = hostPort.lastIndexOf(':');
      if (colon >= 0) {
        final maybePort = hostPort.substring(colon + 1);
        if (maybePort.isNotEmpty && _isAllDigits(maybePort)) {
          hostStr = hostPort.substring(0, colon);
          port = _parsePort(maybePort);
        } else {
          hostStr = hostPort;
          port = null;
        }
      } else {
        hostStr = hostPort;
        port = null;
      }
    }

    return Url._(
      scheme: scheme,
      userinfo: userinfo,
      host: hostStr.isEmpty ? null : hostStr.toLowerCase(),
      port: port,
      path: path,
      query: afterQuery.$2,
      fragment: afterFragment.$2,
      raw: raw,
    );
  }

  /// Resolves a relative URL against this URL as the base (RFC 1808):
  ///
  /// * empty reference → the base without its fragment;
  /// * `#fragment` → the base with the fragment replaced;
  /// * a reference with a scheme → parsed as absolute;
  /// * `//host/path` → this scheme plus the new authority;
  /// * `/path` → this scheme and authority plus the new path;
  /// * otherwise → the reference merged onto the base path with dot
  ///   segments removed.
  ///
  /// The reference's query/fragment (when the reference carries them)
  /// replace the base's; a reference without them clears them.
  Url resolve(String relative) {
    relative = relative.trim();

    if (relative.isEmpty) {
      const clear = _Present<String?>(null);
      final result = _copyWith(fragment: clear);
      return result._withRaw(result.toUrlString());
    }

    if (relative.startsWith('#')) {
      final result = _copyWith(fragment: _optional(relative.substring(1)));
      return result._withRaw(result.toUrlString());
    }

    // Absolute reference: parse as-is when the part before ':' is
    // scheme-shaped (non-empty, alphanumerics or +.-, alpha first).
    if (relative.contains('://') ||
        (relative.contains(':') && !relative.startsWith('/'))) {
      final colon = relative.indexOf(':');
      if (colon >= 0) {
        final maybeScheme = relative.substring(0, colon);
        if (_isSchemeShaped(maybeScheme)) {
          return Url.parse(relative);
        }
      }
    }

    // Scheme-relative: inherit only the scheme.
    if (relative.startsWith('//')) {
      return Url.parse('$scheme:$relative');
    }

    // Absolute path: inherit scheme and authority.
    if (relative.startsWith('/')) {
      final pathAndFragment = _splitFragment(relative);
      final pathAndQuery = _splitQuery(pathAndFragment.$1);
      final result = _copyWith(
        path: _removeDotSegments(pathAndQuery.$1),
        query: _optional(pathAndQuery.$2),
        fragment: _optional(pathAndFragment.$2),
      );
      return result._withRaw(result.toUrlString());
    }

    // Relative path: merge onto the base path.
    final pathAndFragment = _splitFragment(relative);
    final pathAndQuery = _splitQuery(pathAndFragment.$1);
    final merged = _mergePaths(path, pathAndQuery.$1);
    final result = _copyWith(
      path: _removeDotSegments(merged),
      query: _optional(pathAndQuery.$2),
      fragment: _optional(pathAndFragment.$2),
    );
    return result._withRaw(result.toUrlString());
  }

  /// The effective port: the explicit port, else the scheme default
  /// (http 80, https 443, ftp 21), else null.
  int? effectivePort() => port ?? _defaultPort(scheme);

  /// The authority string: `[userinfo@]host[:port]`.
  String authority() {
    final buffer = StringBuffer();
    if (userinfo != null) {
      buffer.write(userinfo);
      buffer.write('@');
    }
    if (host != null) buffer.write(host);
    if (port != null) {
      buffer.write(':');
      buffer.write(port);
    }
    return buffer.toString();
  }

  /// Serializes back to a URL string.
  String toUrlString() {
    final buffer = StringBuffer()..write(scheme);
    if (host != null) {
      buffer.write('://');
      buffer.write(authority());
    } else {
      buffer.write(':');
    }
    buffer.write(path);
    if (query != null) {
      buffer.write('?');
      buffer.write(query);
    }
    if (fragment != null) {
      buffer.write('#');
      buffer.write(fragment);
    }
    return buffer.toString();
  }

  /// Returns a canonical spelling for stable URL identity: lowercased
  /// scheme and host, explicit default port dropped, percent escapes
  /// normalized (unreserved bytes decoded, retained escapes uppercased),
  /// path dot segments removed. Fragments are kept, so callers choose
  /// whether identity includes them.
  ///
  /// Throws [UrlError.invalidPercentEncoding] when any component carries a
  /// malformed escape.
  Url canonicalize() {
    final userinfo =
        this.userinfo == null ? null : _normalizeEscapes(this.userinfo!);
    final path = _removeDotSegments(_normalizeEscapes(this.path));
    final query = this.query == null ? null : _normalizeEscapes(this.query!);
    final fragment =
        this.fragment == null ? null : _normalizeEscapes(this.fragment!);
    final loweredScheme = _asciiLowercase(scheme);
    final loweredHost = host == null ? null : _asciiLowercase(host!);
    var port = this.port;
    if (port == _defaultPort(loweredScheme)) port = null;
    final result = Url._(
      scheme: loweredScheme,
      userinfo: userinfo,
      host: loweredHost,
      port: port,
      path: (loweredHost != null && path.isEmpty) ? '/' : path,
      query: query,
      fragment: fragment,
      raw: '',
    );
    return result._withRaw(result.toUrlString());
  }

  Url _copyWith({
    String? path,
    _Maybe<String?> query = _absent,
    _Maybe<String?> fragment = _absent,
  }) =>
      Url._(
        scheme: scheme,
        userinfo: userinfo,
        host: host,
        port: port,
        path: path ?? this.path,
        query: query is _Present<String?> ? query.value : this.query,
        fragment:
            fragment is _Present<String?> ? fragment.value : this.fragment,
        raw: _raw,
      );

  Url _withRaw(String raw) => Url._(
        scheme: scheme,
        userinfo: userinfo,
        host: host,
        port: port,
        path: path,
        query: query,
        fragment: fragment,
        raw: raw,
      );

  @override
  bool operator ==(Object other) =>
      other is Url &&
      scheme == other.scheme &&
      userinfo == other.userinfo &&
      host == other.host &&
      port == other.port &&
      path == other.path &&
      query == other.query &&
      fragment == other.fragment &&
      _raw == other._raw;

  @override
  int get hashCode =>
      Object.hash(scheme, userinfo, host, port, path, query, fragment, _raw);

  @override
  String toString() => toUrlString();
}

/// Optional-override marker: distinguishes "leave the field" ([_absent])
/// from "set the field, possibly to null" ([_Present]).
class _Maybe<T> {
  const _Maybe();
}

class _Present<T> extends _Maybe<T> {
  final T value;
  const _Present(this.value);
}

const _absent = _Maybe<Never>();

_Maybe<String?> _optional(String? value) => _Present<String?>(value);

/// Percent-encodes a string for a URL path or query: every UTF-8 byte
/// outside `A-Z a-z 0-9 - _ . ~ /` becomes an uppercase `%XX` escape.
///
/// Throws [UrlError.invalidPercentEncoding] when [input] holds lone
/// surrogates (unencodable text — unrepresentable in the reference's UTF-8
/// strings, so the closest failure mode is used).
String percentEncode(String input) {
  if (_hasLoneSurrogate(input)) throw UrlError.invalidPercentEncoding;
  final bytes = utf8.encode(input);
  final buffer = StringBuffer();
  for (final byte in bytes) {
    if (_isUnreserved(byte)) {
      buffer.writeCharCode(byte);
    } else {
      buffer
        ..write('%')
        ..write(byte.toRadixString(16).padLeft(2, '0').toUpperCase());
    }
  }
  return buffer.toString();
}

/// Percent-decodes a string: `%20` → space, `%E6%97%A5` → 日.
///
/// Each `%XX` contributes one byte; the bytes then decode as UTF-8.
/// Throws [UrlError.invalidPercentEncoding] on truncated escapes, non-hex
/// digits, invalid UTF-8, or lone surrogates in [input].
String percentDecode(String input) {
  // Walk the UTF-8 bytes exactly as the reference walks input.as_bytes():
  // '%' (0x25) can never appear inside a multibyte sequence, so byte
  // scanning is safe and literal multibyte runs survive verbatim.
  if (_hasLoneSurrogate(input)) throw UrlError.invalidPercentEncoding;
  final bytes = utf8.encode(input);
  final result = <int>[];
  var i = 0;
  while (i < bytes.length) {
    if (bytes[i] != 0x25) {
      result.add(bytes[i]);
      i++;
      continue;
    }
    if (i + 2 >= bytes.length) throw UrlError.invalidPercentEncoding;
    final high = _hexValue(bytes[i + 1]);
    final low = _hexValue(bytes[i + 2]);
    result.add((high << 4) | low);
    i += 3;
  }
  try {
    // Dart's utf8.decode strips a LEADING byte-order mark while the
    // reference's from_utf8 preserves it. A one-byte ASCII pad moves any
    // BOM off the front (mid-string BOMs survive verbatim, as probed);
    // dropping the pad character afterwards restores the exact meaning.
    // Invalid sequences still throw: the pad is one complete ASCII
    // character and cannot interact with them.
    return utf8.decode([0x41, ...result]).substring(1);
  } on FormatException {
    throw UrlError.invalidPercentEncoding;
  }
}

/// True iff [s] holds an unpaired UTF-16 surrogate.
///
/// Dart strings can contain lone surrogates, which have no UTF-8 encoding
/// (and are unrepresentable in the reference's strings). Both percent
/// functions reject such input explicitly rather than depending on codec
/// leniency.
bool _hasLoneSurrogate(String s) {
  final units = s.codeUnits;
  var i = 0;
  while (i < units.length) {
    final unit = units[i];
    if (unit >= 0xD800 && unit <= 0xDBFF) {
      if (i + 1 >= units.length) return true;
      final next = units[i + 1];
      if (next < 0xDC00 || next > 0xDFFF) return true;
      i += 2;
    } else if (unit >= 0xDC00 && unit <= 0xDFFF) {
      return true;
    } else {
      i++;
    }
  }
  return false;
}

bool _isUnreserved(int byte) =>
    (byte >= 0x30 && byte <= 0x39) ||
    (byte >= 0x41 && byte <= 0x5a) ||
    (byte >= 0x61 && byte <= 0x7a) ||
    byte == 0x2d ||
    byte == 0x5f ||
    byte == 0x2e ||
    byte == 0x7e ||
    byte == 0x2f;

bool _isCanonicalUnreserved(int byte) =>
    (byte >= 0x30 && byte <= 0x39) ||
    (byte >= 0x41 && byte <= 0x5a) ||
    (byte >= 0x61 && byte <= 0x7a) ||
    byte == 0x2d ||
    byte == 0x5f ||
    byte == 0x2e ||
    byte == 0x7e;

/// Normalizes the `%XX` escapes in one component: escapes decoding to
/// unreserved bytes are replaced by the byte; retained escapes are
/// uppercased. Anything else passes through verbatim.
String _normalizeEscapes(String input) {
  final units = input.codeUnits;
  final buffer = StringBuffer();
  var index = 0;
  while (index < units.length) {
    if (units[index] != 0x25) {
      final start = index;
      while (index < units.length && units[index] != 0x25) {
        index++;
      }
      buffer.write(input.substring(start, index));
      continue;
    }
    if (index + 2 >= units.length) {
      throw UrlError.invalidPercentEncoding;
    }
    final high = _hexValue(units[index + 1]);
    final low = _hexValue(units[index + 2]);
    final decoded = (high << 4) | low;
    if (_isCanonicalUnreserved(decoded)) {
      buffer.writeCharCode(decoded);
    } else {
      buffer
        ..write('%')
        ..write(_upperHexDigit(high))
        ..write(_upperHexDigit(low));
    }
    index += 3;
  }
  return buffer.toString();
}

String _upperHexDigit(int value) => '0123456789ABCDEF'[value];

int _hexValue(int unit) {
  if (unit >= 0x30 && unit <= 0x39) return unit - 0x30;
  if (unit >= 0x61 && unit <= 0x66) return unit - 0x61 + 10;
  if (unit >= 0x41 && unit <= 0x46) return unit - 0x41 + 10;
  throw UrlError.invalidPercentEncoding;
}

void _validateScheme(String scheme) {
  if (scheme.isEmpty) throw UrlError.invalidScheme;
  final units = scheme.codeUnits;
  if (units[0] < 0x61 || units[0] > 0x7a) {
    throw UrlError.invalidScheme;
  }
  for (var i = 1; i < units.length; i++) {
    final c = units[i];
    final ok = (c >= 0x61 && c <= 0x7a) ||
        (c >= 0x30 && c <= 0x39) ||
        c == 0x2b ||
        c == 0x2d ||
        c == 0x2e;
    if (!ok) throw UrlError.invalidScheme;
  }
}

int _parsePort(String s) {
  final port = int.tryParse(s);
  if (port == null || port < 0 || port > 65535) {
    throw UrlError.invalidPort;
  }
  return port;
}

bool _isAllDigits(String s) {
  for (final unit in s.codeUnits) {
    if (unit < 0x30 || unit > 0x39) return false;
  }
  return true;
}

int? _defaultPort(String scheme) {
  switch (scheme) {
    case 'http':
      return 80;
    case 'https':
      return 443;
    case 'ftp':
      return 21;
    default:
      return null;
  }
}

(String, String?) _splitFragment(String input) {
  final pos = input.indexOf('#');
  if (pos < 0) return (input, null);
  return (input.substring(0, pos), input.substring(pos + 1));
}

(String, String?) _splitQuery(String input) {
  final pos = input.indexOf('?');
  if (pos < 0) return (input, null);
  return (input.substring(0, pos), input.substring(pos + 1));
}

bool _isSchemeShaped(String s) {
  if (s.isEmpty) return false;
  final units = s.codeUnits;
  final first = units[0];
  final firstOk =
      (first >= 0x41 && first <= 0x5a) || (first >= 0x61 && first <= 0x7a);
  if (!firstOk) return false;
  for (var i = 1; i < units.length; i++) {
    final c = units[i];
    final ok = (c >= 0x41 && c <= 0x5a) ||
        (c >= 0x61 && c <= 0x7a) ||
        (c >= 0x30 && c <= 0x39) ||
        c == 0x2b ||
        c == 0x2d ||
        c == 0x2e;
    if (!ok) return false;
  }
  return true;
}

String _mergePaths(String basePath, String relativePath) {
  final pos = basePath.lastIndexOf('/');
  if (pos < 0) return '/$relativePath';
  return basePath.substring(0, pos + 1) + relativePath;
}

String _removeDotSegments(String path) {
  final output = <String>[];
  for (final segment in path.split('/')) {
    if (segment == '.') continue;
    if (segment == '..') {
      if (output.isNotEmpty) output.removeLast();
    } else {
      output.add(segment);
    }
  }
  final result = output.join('/');
  if (path.startsWith('/') && !result.startsWith('/')) {
    return '/$result';
  }
  return result;
}

String _asciiLowercase(String s) {
  final buffer = StringBuffer();
  for (final unit in s.codeUnits) {
    // Defensive in practice: parse already lowercases schemes and hosts,
    // and the constructor is private, so uppercase input cannot arrive
    // here. Kept to mirror the reference, which re-lowercases likewise.
    if (unit >= 0x41 && unit <= 0x5a) {
      buffer.writeCharCode(unit + 32);
    } else {
      buffer.writeCharCode(unit);
    }
  }
  return buffer.toString();
}
