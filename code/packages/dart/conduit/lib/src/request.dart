// request.dart — Request class (WEB17)
//
// Request is a thin read-only wrapper around the opaque ConduitRequest* pointer
// passed by conduit-capi to each handler callback.
//
// LIFETIME WARNING
// ────────────────
// The ConduitRequest* is valid ONLY during the handler call. Do NOT store a
// Request object after the handler returns — the pointer it wraps will be freed
// by conduit-capi immediately after the callback returns, resulting in a
// use-after-free if you read any property later.
//
// Copy any data you need before returning from the handler:
//   final name = req.param('name') ?? 'stranger';   // OK — copies immediately
//   storeGlobal(req);                                // DANGER — use-after-free

import 'dart:ffi';
import 'dart:typed_data';
import 'dart:convert';
import 'package:ffi/ffi.dart';
import 'ffi.dart' as ffi;

/// A read-only view of an HTTP request. Valid only inside the handler call.
class Request {
  final Pointer<Void> _ptr;

  /// Internal constructor — only the trampoline layer creates Request objects.
  Request.internal(this._ptr);

  // ── String accessors ──────────────────────────────────────────────────────

  /// HTTP method in uppercase: "GET", "POST", "PUT", "DELETE", etc.
  String get method => ffi.cstrToDart(ffi.conduitRequestMethod(_ptr));

  /// URL path without query string: "/api/users/42".
  String get path => ffi.cstrToDart(ffi.conduitRequestPath(_ptr));

  /// Raw query string without the leading '?': "q=hello&page=2".
  String get queryString => ffi.cstrToDart(ffi.conduitRequestQueryString(_ptr));

  /// Content-Type header value, or "" if absent.
  String get contentType => ffi.cstrToDart(ffi.conduitRequestContentType(_ptr));

  /// Remote address as "IP:port": "127.0.0.1:54321".
  String get remoteAddr => ffi.cstrToDart(ffi.conduitRequestRemoteAddr(_ptr));

  /// Non-empty only inside an onError handler — the Rust error message.
  String get error => ffi.cstrToDart(ffi.conduitRequestError(_ptr));

  // ── Parametric accessors ──────────────────────────────────────────────────

  /// Named route parameter from the URL pattern, or null if absent.
  String? param(String name) {
    final namePtr = name.toNativeUtf8();
    try {
      return ffi.cstrToDartNullable(ffi.conduitRequestParam(_ptr, namePtr));
    } finally {
      calloc.free(namePtr);
    }
  }

  /// Query string value for the given key, or null if absent.
  String? query(String name) {
    final namePtr = name.toNativeUtf8();
    try {
      return ffi.cstrToDartNullable(ffi.conduitRequestQuery(_ptr, namePtr));
    } finally {
      calloc.free(namePtr);
    }
  }

  /// Request header value (case-insensitive lookup), or null if absent.
  String? header(String name) {
    final namePtr = name.toNativeUtf8();
    try {
      return ffi.cstrToDartNullable(ffi.conduitRequestHeader(_ptr, namePtr));
    } finally {
      calloc.free(namePtr);
    }
  }

  // ── Body ─────────────────────────────────────────────────────────────────

  /// Raw request body bytes. Returns an empty list if no body.
  Uint8List body() {
    final lenPtr = calloc<IntPtr>();
    try {
      final bodyPtr = ffi.conduitRequestBody(_ptr, lenPtr);
      final len = lenPtr.value;
      if (bodyPtr == nullptr || len == 0) return Uint8List(0);
      if (len > 1 << 30) {
        throw StateError('Native request body length $len is implausibly large.');
      }
      return Uint8List.fromList(bodyPtr.asTypedList(len));
    } finally {
      calloc.free(lenPtr);
    }
  }

  /// Request body decoded as UTF-8 text.
  String bodyString() => utf8.decode(body());
}
