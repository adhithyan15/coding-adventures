// response.dart — Response class and HaltException (WEB17)
//
// Response is an immutable value type: a status code, a body string, and a
// list of (name, value) header pairs. All factory constructors set sensible
// defaults; withStatus / withHeader return new instances (immutable updates).
//
// STATUS RANGE
// ────────────
// We accept [100, 999]: the IANA-registered HTTP range is [100, 599]; we
// allow up to 999 for experimental / proprietary codes, matching WEB15–16.
//
// RESPONSE BUILDING PATTERN
// ─────────────────────────
// Dart supports optional named parameters on class constructors and static
// methods — unlike F# module-let bindings (WEB16), so status overrides ARE
// first-class here:
//
//   Response.json('{}', status: 201)
//   Response.html('<p>gone</p>', status: 410)
//   Response.html('<p>hi</p>').withStatus(201).withHeader('x-foo', 'bar')

import 'dart:ffi';
import 'dart:convert';
import 'package:ffi/ffi.dart';
import 'ffi.dart' as ffi;

/// An immutable HTTP response: status code, body text, and optional headers.
class Response {
  final int status;
  final String body;
  final List<(String, String)> headers;

  const Response._(this.status, this.body, this.headers);

  // ── Validation ───────────────────────────────────────────────────────────

  static void _checkStatus(int status) {
    if (status < 100 || status > 999) {
      throw ArgumentError.value(
        status,
        'status',
        'HTTP status code must be in [100, 999].',
      );
    }
  }

  // ── Factory constructors ─────────────────────────────────────────────────

  /// HTML response (content-type: text/html; charset=utf-8). Default 200.
  factory Response.html(String body, {int status = 200}) {
    _checkStatus(status);
    return Response._(status, body, [
      ('content-type', 'text/html; charset=utf-8'),
    ]);
  }

  /// JSON response (content-type: application/json). Default 200.
  factory Response.json(String body, {int status = 200}) {
    _checkStatus(status);
    return Response._(status, body, [
      ('content-type', 'application/json'),
    ]);
  }

  /// Plain-text response (content-type: text/plain; charset=utf-8). Default 200.
  factory Response.text(String body, {int status = 200}) {
    _checkStatus(status);
    return Response._(status, body, [
      ('content-type', 'text/plain; charset=utf-8'),
    ]);
  }

  /// Redirect (HTTP 302 by default). Rejects location containing CR or LF to
  /// prevent HTTP response splitting / header injection.
  factory Response.redirect(String location, {int status = 302}) {
    _checkStatus(status);
    if (location.contains('\r') || location.contains('\n')) {
      throw ArgumentError.value(
        location,
        'location',
        'Redirect location must not contain CR or LF.',
      );
    }
    return Response._(status, '', [('location', location)]);
  }

  /// Arbitrary status, body, and header list.
  factory Response.respond(
    int status,
    String body, {
    List<(String, String)> headers = const [],
  }) {
    _checkStatus(status);
    return Response._(status, body, List.unmodifiable(headers));
  }

  // ── Transformations ───────────────────────────────────────────────────────

  /// Return a new Response with the status code replaced.
  Response withStatus(int status) {
    _checkStatus(status);
    return Response._(status, body, headers);
  }

  /// Return a new Response with an additional header appended.
  ///
  /// Rejects CR or LF in name or value to prevent HTTP response splitting.
  Response withHeader(String name, String value) {
    if (name.contains('\r') || name.contains('\n') ||
        value.contains('\r') || value.contains('\n')) {
      throw ArgumentError(
        'Header name and value must not contain CR or LF.',
      );
    }
    return Response._(status, body, [...headers, (name, value)]);
  }

  // ── Native conversion (internal) ──────────────────────────────────────────

  /// Allocate a ConduitResponse* — ownership transfers to Rust.
  Pointer<Void> toNative() {
    _checkStatus(status);

    final bytes = utf8.encode(body);
    // Allocate a pinned native buffer for the body bytes.
    // We use a 1-byte placeholder when the body is empty to guarantee a
    // valid (non-null) pointer to conduit_response_new.
    final buf = calloc<Uint8>(bytes.isEmpty ? 1 : bytes.length);
    try {
      if (bytes.isNotEmpty) {
        final typedBuf = buf.asTypedList(bytes.length);
        typedBuf.setAll(0, bytes);
      }
      final resp = ffi.conduitResponseNew(status, buf, bytes.length);
      for (final (name, value) in headers) {
        final nPtr = name.toNativeUtf8();
        final vPtr = value.toNativeUtf8();
        try {
          ffi.conduitResponseSetHeader(resp, nPtr, vPtr);
        } finally {
          calloc.free(nPtr);
          calloc.free(vPtr);
        }
      }
      return resp;
    } finally {
      calloc.free(buf);
    }
  }

  /// Read a ConduitResponse* into a managed Response, copying all data.
  /// Caller retains ownership of [ptr] and must free it afterwards.
  static Response fromNative(Pointer<Void> ptr) {
    final status = ffi.conduitResponseStatus(ptr);

    final lenPtr = calloc<IntPtr>();
    String bodyStr;
    try {
      final bodyPtr = ffi.conduitResponseBody(ptr, lenPtr);
      final len = lenPtr.value;
      if (bodyPtr == nullptr || len == 0) {
        bodyStr = '';
      } else {
        if (len > 1 << 30) {
          throw StateError('Native response body length $len is implausibly large.');
        }
        bodyStr = utf8.decode(bodyPtr.asTypedList(len));
      }
    } finally {
      calloc.free(lenPtr);
    }

    final count = ffi.conduitResponseHeaderCount(ptr);
    final headers = <(String, String)>[];
    for (var i = 0; i < count; i++) {
      final n = ffi.cstrToDart(ffi.conduitResponseHeaderName(ptr, i));
      final v = ffi.cstrToDart(ffi.conduitResponseHeaderValue(ptr, i));
      headers.add((n, v));
    }

    return Response._(status, bodyStr, List.unmodifiable(headers));
  }

  @override
  String toString() => 'Response($status, ${body.length} bytes)';
}

/// Throw inside a handler to immediately short-circuit with [response].
///
/// Equivalent to Sinatra's `halt` or Express's early `return res.send(...)`.
/// The trampoline catches this and calls [response].toNative() instead of
/// propagating the exception across the FFI boundary (which is undefined
/// behaviour).
class HaltException implements Exception {
  final Response response;
  const HaltException(this.response);
}
