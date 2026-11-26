/// JSON-RPC 2.0 message types: [Request], [Response], [Notification],
/// and the [Message] discriminated union, with [parseMessage] and
/// [messageToJson].
///
/// | Fields present          | Type         |
/// |-------------------------|--------------|
/// | `id` + `method`         | Request      |
/// | `method` only (no `id`) | Notification |
/// | `result` or `error`     | Response     |
///
/// Dynamic JSON values (`id`, `params`, `result`, `data`) use plain
/// `Object?`: maps, lists, strings, ints, doubles, bools, and null, as
/// produced by `dart:convert` — the established in-lane JSON story.
library;

import 'dart:convert';

import 'errors.dart';
import 'json_values.dart';

/// A JSON-RPC message id — string, integer, or null.
///
/// Like the reference, ids are carried unvalidated: even `null` ids on
/// requests parse (the "must not be null" rule is documented, not
/// enforced).
typedef JsonRpcId = Object?;

/// A JSON-RPC Request: `id`, `method`, and optional `params`.
final class Request extends Message {
  const Request({required this.id, required this.method, this.params});

  /// Correlates the response back to this request.
  final JsonRpcId id;

  /// The procedure name, e.g. `"textDocument/hover"`.
  final String method;

  /// Optional parameters — anything decodable.
  final Object? params;

  @override
  bool operator ==(Object other) =>
      other is Request &&
      jsonValuesEqual(other.id, id) &&
      other.method == method &&
      jsonValuesEqual(other.params, params);

  @override
  int get hashCode =>
      Object.hash(jsonValueHash(id), method, jsonValueHash(params));

  @override
  String toString() => 'Request($id, $method, $params)';
}

/// A JSON-RPC Response: `id` plus a `result` or an `error`.
///
/// The `id` must match the originating Request; it may be null only when
/// the server could not determine the request id.
final class Response extends Message {
  const Response({required this.id, this.result, this.error});

  /// The id from the originating Request.
  final JsonRpcId id;

  /// The success result. Mutually exclusive with [error].
  final Object? result;

  /// The error object. Mutually exclusive with [result].
  final ResponseError? error;

  @override
  bool operator ==(Object other) =>
      other is Response &&
      jsonValuesEqual(other.id, id) &&
      jsonValuesEqual(other.result, result) &&
      other.error == error;

  @override
  int get hashCode =>
      Object.hash(jsonValueHash(id), jsonValueHash(result), error);

  @override
  String toString() => 'Response($id, $result, $error)';
}

/// A JSON-RPC Notification: `method` but no `id`.
///
/// The server must not send a response to a Notification.
final class Notification extends Message {
  const Notification({required this.method, this.params});

  /// The event name, e.g. `"textDocument/didOpen"`.
  final String method;

  /// Optional parameters.
  final Object? params;

  @override
  bool operator ==(Object other) =>
      other is Notification &&
      other.method == method &&
      jsonValuesEqual(other.params, params);

  @override
  int get hashCode => Object.hash(method, jsonValueHash(params));

  @override
  String toString() => 'Notification($method, $params)';
}

/// A JSON-RPC message — one of [Request], [Response], or [Notification].
sealed class Message {
  const Message();
}

/// Parses JSON bytes into a typed [Message].
///
/// Throws [ResponseError]: `parseErrorCode` when the bytes are not valid
/// JSON (or carry out-of-range integers or lone surrogates, which fail
/// closed), `invalidRequestCode` when the JSON is not a JSON-RPC object.
///
/// Discrimination: `method` + `id` → [Request]; `method` without `id` →
/// [Notification]; `result` or `error` → [Response]; otherwise invalid.
/// The `jsonrpc` marker itself is never checked, exactly like the
/// reference.
Message parseMessage(List<int> bytes) {
  // serde rejects a leading BOM while utf8.decode strips it: fail closed
  // to match.
  if (bytes.length >= 3 &&
      bytes[0] == 0xEF &&
      bytes[1] == 0xBB &&
      bytes[2] == 0xBF) {
    throw ResponseError.parseError('unexpected byte-order mark');
  }
  final Object? value;
  try {
    final text = utf8.decode(bytes, allowMalformed: false);
    checkJsonIntegers(text);
    value = jsonDecode(text);
    checkNoLoneSurrogates(value);
  } on FormatException catch (e) {
    throw ResponseError.parseError(e.message);
  }
  if (value is! Map<String, Object?>) {
    throw ResponseError.invalidRequest('expected JSON object');
  }
  final obj = value;

  final hasMethod = obj.containsKey('method');
  final hasId = obj.containsKey('id');
  final hasResult = obj.containsKey('result');
  final hasError = obj.containsKey('error');

  if (hasMethod && hasId) {
    final method = obj['method'];
    if (method is! String) {
      throw ResponseError.invalidRequest('method must be a string');
    }
    return Request(
      id: obj['id'],
      method: method,
      params: obj.containsKey('params') ? obj['params'] : null,
    );
  }
  if (hasMethod) {
    final method = obj['method'];
    if (method is! String) {
      throw ResponseError.invalidRequest('method must be a string');
    }
    return Notification(
      method: method,
      params: obj.containsKey('params') ? obj['params'] : null,
    );
  }
  if (hasResult || hasError) {
    final errorJson = obj.containsKey('error') ? obj['error'] : null;
    ResponseError? error;
    if (obj.containsKey('error')) {
      try {
        error = ResponseError.fromJson(errorJson);
      } on FormatException catch (e) {
        throw ResponseError.parseError('invalid error object: ${e.message}');
      }
    }
    return Response(
      id: obj.containsKey('id') ? obj['id'] : null,
      result: obj.containsKey('result') ? obj['result'] : null,
      error: error,
    );
  }
  throw ResponseError.invalidRequest(
      "missing 'method', 'result', or 'error' field");
}

/// Converts a typed [Message] to a JSON-ready map for serialization.
///
/// Always injects `"jsonrpc": "2.0"`. Key order is fixed
/// (jsonrpc/id/method/params, jsonrpc/id/error-or-result) so framed
/// bytes are byte-reproducible.
Map<String, Object?> messageToJson(Message message) {
  switch (message) {
    case Request(:final id, :final method, :final params):
      return {
        'jsonrpc': '2.0',
        'id': id,
        'method': method,
        if (params != null) 'params': params,
      };
    case Response(:final id, :final result, :final error):
      if (error != null) {
        return {
          'jsonrpc': '2.0',
          'id': id,
          'error': error.toJson(),
        };
      }
      return {
        'jsonrpc': '2.0',
        'id': id,
        'result': result,
      };
    case Notification(:final method, :final params):
      return {
        'jsonrpc': '2.0',
        'method': method,
        if (params != null) 'params': params,
      };
  }
}
