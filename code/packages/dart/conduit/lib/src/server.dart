// server.dart — Server class (WEB17)
//
// Returned by Application.bind(). Call serveBackground() to start the Tokio
// accept-loop on a background OS thread, or await serve() to start and wait
// until stop() is called (the idiomatic Dart pattern for long-running servers).
//
// THREADING
// ─────────
// Dart runs on a single isolate event loop. The conduit-capi background server
// dispatches handlers from a Rust/Tokio OS thread. Our trampolines use
// NativeCallable.isolateLocal which posts those calls to the Dart event loop
// and blocks the Rust thread until Dart returns. This requires the event loop
// to remain free — hence serve() is async rather than blocking on the C call.
//
// LIFECYCLE
// ─────────
//   Application.bind() → Server (stopped)
//   server.serveBackground() or await server.serve() → Server (running)
//   server.stop() → Server (stopped again)
//   server.dispose() → Server (freed; do not use)

import 'dart:async';
import 'dart:ffi';
import 'ffi.dart' as ffi;

/// A bound conduit server. Call dispose() when done to free native resources.
class Server {
  Pointer<Void> _ptr;
  bool _freed = false;
  Completer<void>? _serveCompleter;

  /// Internal constructor — only Application.bind() creates Server instances.
  Server.internal(this._ptr);

  void _checkAlive() {
    if (_freed) throw StateError('Server has already been disposed.');
    if (_ptr == nullptr) throw StateError('Server native pointer is null.');
  }

  // ── Properties ────────────────────────────────────────────────────────────

  /// The TCP port the server is listening on. Useful when you bound with port 0.
  int get localPort {
    _checkAlive();
    return ffi.conduitServerLocalPort(_ptr);
  }

  /// True once the Tokio accept-loop is live. Returns false (not throws) if
  /// already disposed.
  bool get isRunning {
    if (_freed || _ptr == nullptr) return false;
    return ffi.conduitServerRunning(_ptr) != 0;
  }

  // ── Serve ─────────────────────────────────────────────────────────────────

  /// Start the Tokio accept-loop on a background OS thread and return
  /// immediately. The Dart event loop remains free to handle incoming requests
  /// via the NativeCallable trampolines.
  void serveBackground() {
    _checkAlive();
    ffi.conduitServerServeBackground(_ptr);
  }

  /// Start the server and wait until stop() is called.
  ///
  /// This is the idiomatic entry point for a long-running server:
  ///
  ///   Future<void> main() async {
  ///     final server = Application().get('/', ...).bind('127.0.0.1', 3000);
  ///     print('listening on port ${server.localPort}');
  ///     await server.serve();  // returns when stop() is called
  ///   }
  ///
  /// Calls serveBackground() internally so the Dart event loop stays free
  /// to process incoming requests via NativeCallable.isolateLocal.
  Future<void> serve() async {
    serveBackground();
    _serveCompleter = Completer<void>();
    return _serveCompleter!.future;
  }

  // ── Stop ─────────────────────────────────────────────────────────────────

  /// Signal the server to stop accepting new connections.
  void stop() {
    _checkAlive();
    ffi.conduitServerStop(_ptr);
    _serveCompleter?.complete();
    _serveCompleter = null;
  }

  // ── Dispose ───────────────────────────────────────────────────────────────

  /// Free the native Server*. Call this when done (e.g. in a finally block).
  void dispose() {
    if (!_freed && _ptr != nullptr) {
      ffi.conduitServerFree(_ptr);
      _ptr = nullptr;
      _freed = true;
      _serveCompleter?.complete();
      _serveCompleter = null;
    }
  }
}
