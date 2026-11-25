/// [Server] — reads JSON-RPC messages, dispatches to handlers, and
/// writes responses.
///
/// The loop in [Server.serve] runs to EOF:
/// - [Request] → call the registered handler, write a [Response].
/// - [Notification] → call the registered handler (no response written).
/// - [Response] → ignored.
/// - Framing/parse error → write an error response with a null id.
///
/// Handlers that throw [ResponseError] produce error responses; handlers
/// that throw anything else are caught like the reference's
/// `catch_unwind` and produce `internalErrorCode` — one bad request never
/// kills the server. Write failures are ignored, exactly like the
/// reference's `let _ =`.
library;

import 'errors.dart';
import 'message.dart';
import 'reader.dart';
import 'writer.dart';

/// A handler for [Request] messages.
///
/// Receives the request `id` and `params`; returns the result value, or
/// throws [ResponseError] to fail the call.
typedef RequestHandler = Object? Function(Object? id, Object? params);

/// A handler for [Notification] messages.
///
/// Receives `params`; the return value is ignored.
typedef NotificationHandler = void Function(Object? params);

/// JSON-RPC 2.0 server with method dispatch.
final class Server {
  Server({required MessageReader reader, required MessageWriter writer})
      : _reader = reader,
        _writer = writer;

  final MessageReader _reader;
  final MessageWriter _writer;
  final Map<String, RequestHandler> _requestHandlers = {};
  final Map<String, NotificationHandler> _notificationHandlers = {};

  /// Registers a handler for a named request method, replacing any
  /// previous handler for that method.
  void onRequest(String method, RequestHandler handler) {
    _requestHandlers[method] = handler;
  }

  /// Registers a handler for a named notification method, replacing any
  /// previous handler for that method.
  ///
  /// Unknown notifications are silently dropped per the spec, and
  /// notifications never generate error responses.
  void onNotification(String method, NotificationHandler handler) {
    _notificationHandlers[method] = handler;
  }

  /// Starts the read-dispatch-write loop. Returns at clean EOF.
  void serve() {
    while (true) {
      final Message? message;
      try {
        message = _reader.readMessage();
      } on ResponseError catch (e) {
        // Framing or parse error — the request id is undeterminable, so
        // the error response carries a null id.
        _writeQuietly(Response(id: null, error: e));
        continue;
      }
      if (message == null) return;
      switch (message) {
        case Request(:final id, :final method, :final params):
          _handleRequest(id, method, params);
        case Notification(:final method, :final params):
          _handleNotification(method, params);
        case Response():
          // Responses are for client-side use; a responding server
          // ignores incoming ones.
          break;
      }
    }
  }

  void _handleRequest(Object? id, String method, Object? params) {
    final handler = _requestHandlers[method];
    if (handler == null) {
      _writeQuietly(
          Response(id: id, error: ResponseError.methodNotFound(method)));
      return;
    }
    final Object? result;
    try {
      result = handler(id, params);
    } on ResponseError catch (e) {
      _writeQuietly(Response(id: id, error: e));
      return;
    } catch (_) {
      _writeQuietly(Response(
          id: id, error: ResponseError.internalError('handler panicked')));
      return;
    }
    _writeQuietly(Response(id: id, result: result));
  }

  void _handleNotification(String method, Object? params) {
    final handler = _notificationHandlers[method];
    if (handler == null) return;
    try {
      handler(params);
    } catch (_) {
      // A crashing notification handler must not kill the server; its
      // result (and its failure) are both ignored.
    }
  }

  /// Writes a response, dropping write failures like the reference's
  /// `let _ =` (an in-memory buffer only fails on unencodable handler
  /// output, which must not break the loop either).
  void _writeQuietly(Response response) {
    try {
      _writer.writeMessage(response);
    } catch (_) {
      // Dropped, per the reference.
    }
  }
}
