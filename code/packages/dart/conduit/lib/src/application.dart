// application.dart — Application builder (WEB17)
//
// Application is a fluent builder. Methods return `this` for chaining.
// Call bind() to obtain a Server — after bind(), the Application is consumed
// and must not be used again.
//
// USAGE
// ─────
//   final server = Application()
//     .set('app_name', 'hello')
//     .get('/', (req) => Response.html('<h1>Hi</h1>'))
//     .bind('127.0.0.1', 3000);
//
// Read settings BEFORE bind() — the native ConduitApp* is consumed by bind()
// and conduitAppGetSetting will no longer work.

import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';
import 'ffi.dart' as ffi;
import 'request.dart';
import 'response.dart';
import 'server.dart';
import 'trampolines.dart' as tramp;

/// Initialise the bridge and create a native ConduitApp*.
Pointer<Void> _init() {
  ffi.ensureBridgeInitialised();
  return ffi.conduitAppNew();
}

/// Fluent builder for configuring and binding a conduit web server.
class Application {
  Pointer<Void> _ptr;
  bool _consumed = false;

  Application() : _ptr = _init() {
    if (_ptr == nullptr) {
      throw StateError(
        'conduit_app_new() returned null — native library failed to initialise.',
      );
    }
  }

  Pointer<Void> get _checkedPtr {
    if (_consumed) throw StateError('Application has already been consumed by bind().');
    if (_ptr == nullptr) throw StateError('Application native pointer is null.');
    return _ptr;
  }

  // ── Settings ──────────────────────────────────────────────────────────────

  /// Store a string setting. Must be called before bind().
  Application set(String key, String value) {
    final kPtr = key.toNativeUtf8();
    final vPtr = value.toNativeUtf8();
    try {
      ffi.conduitAppSetSetting(_checkedPtr, kPtr, vPtr);
    } finally {
      calloc.free(kPtr);
      calloc.free(vPtr);
    }
    return this;
  }

  /// Retrieve a setting stored with set(). Returns null if absent.
  /// Must be called before bind() — the native app is consumed by bind().
  String? getSetting(String key) {
    final kPtr = key.toNativeUtf8();
    try {
      final ptr = ffi.conduitAppGetSetting(_checkedPtr, kPtr);
      if (ptr == nullptr) return null;
      final s = ptr.toDartString();
      ffi.conduitStringFree(ptr);
      return s;
    } finally {
      calloc.free(kPtr);
    }
  }

  // ── Route registration ────────────────────────────────────────────────────

  Application _addRoute(
    String method,
    String pattern,
    Response Function(Request) handler,
  ) {
    final mPtr = method.toNativeUtf8();
    final pPtr = pattern.toNativeUtf8();
    try {
      final ctx = tramp.allocHandler(handler);
      ffi.conduitAppAddRoute(
        _checkedPtr,
        mPtr,
        pPtr,
        ffi.bridgeHandlerFn,
        ctx,
        ffi.bridgeCtxFreeFn,
      );
    } finally {
      calloc.free(mPtr);
      calloc.free(pPtr);
    }
    return this;
  }

  /// Register a GET route.
  Application get(String pattern, Response Function(Request) handler) =>
      _addRoute('GET', pattern, handler);

  /// Register a POST route.
  Application post(String pattern, Response Function(Request) handler) =>
      _addRoute('POST', pattern, handler);

  /// Register a PUT route.
  Application put(String pattern, Response Function(Request) handler) =>
      _addRoute('PUT', pattern, handler);

  /// Register a DELETE route.
  Application delete(String pattern, Response Function(Request) handler) =>
      _addRoute('DELETE', pattern, handler);

  /// Register a PATCH route.
  Application patch(String pattern, Response Function(Request) handler) =>
      _addRoute('PATCH', pattern, handler);

  /// Register a route with an explicit HTTP method string.
  Application route(
    String method,
    String pattern,
    Response Function(Request) handler,
  ) => _addRoute(method, pattern, handler);

  // ── Filters and hooks ─────────────────────────────────────────────────────

  /// Add a before-filter. Return null to continue; return a Response to
  /// short-circuit immediately and send that response.
  Application before(Response? Function(Request) filter) {
    final ctx = tramp.allocBefore(filter);
    ffi.conduitAppAddBefore(_checkedPtr, ffi.bridgeBeforeFn, ctx, ffi.bridgeCtxFreeFn);
    return this;
  }

  /// Add an after-hook. Receives the current response and returns the
  /// (possibly modified) replacement.
  Application after(Response Function(Request, Response) hook) {
    final ctx = tramp.allocAfter(hook);
    ffi.conduitAppAddAfter(_checkedPtr, ffi.bridgeAfterFn, ctx, ffi.bridgeCtxFreeFn);
    return this;
  }

  // ── Special handlers ──────────────────────────────────────────────────────

  /// Register a custom 404 handler. Runs when no route matches.
  Application notFound(Response Function(Request) handler) {
    final ctx = tramp.allocHandler(handler);
    ffi.conduitAppSetNotFound(
      _checkedPtr,
      ffi.bridgeHandlerFn,
      ctx,
      ffi.bridgeCtxFreeFn,
    );
    return this;
  }

  /// Register a custom error handler. Runs when a route handler throws an
  /// unhandled exception. Use req.error to read the sanitised error message.
  Application onError(Response Function(Request) handler) {
    final ctx = tramp.allocHandler(handler);
    ffi.conduitAppSetErrorHandler(
      _checkedPtr,
      ffi.bridgeHandlerFn,
      ctx,
      ffi.bridgeCtxFreeFn,
    );
    return this;
  }

  // ── Bind ──────────────────────────────────────────────────────────────────

  /// Consume the Application, bind to [host]:[port], and return a Server.
  /// The Application must not be used after this call.
  ///
  /// Pass port 0 to let the OS pick an ephemeral port; read it back via
  /// server.localPort.
  ///
  /// Throws [StateError] on failure (e.g. port already in use).
  Server bind(String host, int port) {
    final ptr = _checkedPtr;
    _consumed = true;
    _ptr = nullptr;

    final hostPtr = host.toNativeUtf8();
    Pointer<Void> srv;
    try {
      srv = ffi.conduitServerBind(hostPtr, port, ptr);
    } finally {
      calloc.free(hostPtr);
    }

    if (srv == nullptr) {
      final rawErr = ffi.cstrToDart(ffi.conduitLastError());
      final safeErr = ffi.sanitizeForLog(rawErr);
      // Raw error → stderr only; generic message in the exception.
      stderr.writeln('[conduit] conduit_server_bind failed: $safeErr');
      throw StateError(
        'Failed to bind conduit server on $host:$port. See stderr for details.',
      );
    }

    return Server.internal(srv);
  }

  // ── Dispose ───────────────────────────────────────────────────────────────

  /// Free the native ConduitApp* if bind() was never called (e.g. an error
  /// occurred during route registration). Safe to call multiple times.
  void dispose() {
    if (!_consumed && _ptr != nullptr) {
      ffi.conduitAppFree(_ptr);
      _ptr = nullptr;
    }
  }
}
