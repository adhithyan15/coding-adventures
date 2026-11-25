/// Standard JSON-RPC 2.0 error codes and the [ResponseError] type.
///
/// | Code          | Constant            | Meaning                                  |
/// |---------------|---------------------|------------------------------------------|
/// | `-32700`      | [parseErrorCode]    | The framed bytes are not valid JSON    |
/// | `-32600`      | [invalidRequestCode]| Valid JSON but not a valid request     |
/// | `-32601`      | [methodNotFoundCode]| No handler registered for the method   |
/// | `-32602`      | [invalidParamsCode] | Handler rejected the params            |
/// | `-32603`      | [internalErrorCode] | Unexpected error inside the handler    |
/// | `-32000…-99`  | *(reserved)*        | Implementation-defined server errors   |
///
/// The LSP range (`-32899`…`-32800`) belongs to the LSP layer above and is
/// never produced here.
library;

import 'json_values.dart';

/// The framed bytes are not valid JSON.
const int parseErrorCode = -32700;

/// Valid JSON, but not a valid JSON-RPC Request object.
const int invalidRequestCode = -32600;

/// No handler is registered for the requested method.
const int methodNotFoundCode = -32601;

/// The handler rejected the method parameters as invalid.
const int invalidParamsCode = -32602;

/// An unexpected error occurred inside the handler.
const int internalErrorCode = -32603;

/// A JSON-RPC error object, carried inside a [Response] when handling fails.
///
/// ```json
/// {
///   "code": -32601,
///   "message": "Method not found",
///   "data": "textDocument/hover is not registered"
/// }
/// ```
///
/// `data` is optional and omitted from the wire when absent. Implements
/// [Exception] because the parse and framing paths throw it.
final class ResponseError implements Exception {
  const ResponseError({required this.code, required this.message, this.data});

  /// Integer error code from the reserved table above.
  final int code;

  /// Short, human-readable description, stable across versions.
  final String message;

  /// Optional additional context — any JSON value.
  final Object? data;

  /// Creates a Parse error (-32700) with optional [data].
  factory ResponseError.parseError([Object? data]) =>
      ResponseError(code: parseErrorCode, message: 'Parse error', data: data);

  /// Creates an Invalid Request (-32600) error with optional [data].
  factory ResponseError.invalidRequest([Object? data]) => ResponseError(
      code: invalidRequestCode, message: 'Invalid Request', data: data);

  /// Creates a Method not found (-32601) error.
  ///
  /// [method] rides along as `data` so the client can display it.
  factory ResponseError.methodNotFound(String method) => ResponseError(
      code: methodNotFoundCode, message: 'Method not found', data: method);

  /// Creates an Invalid params (-32602) error with optional [data].
  factory ResponseError.invalidParams([Object? data]) => ResponseError(
      code: invalidParamsCode, message: 'Invalid params', data: data);

  /// Creates an Internal error (-32603) with optional [data].
  factory ResponseError.internalError([Object? data]) => ResponseError(
      code: internalErrorCode, message: 'Internal error', data: data);

  /// Parses an error object from decoded JSON.
  ///
  /// Requires an object with an integer `code` and a string `message`
  /// (like serde's strict struct decode: floats, strings-as-codes, and
  /// missing fields all fail); `data` may be anything or absent, and
  /// unknown fields are ignored. Throws [FormatException] when invalid.
  factory ResponseError.fromJson(Object? json) {
    if (json is! Map) {
      throw FormatException(
          'invalid error object: expected object, got ${_describe(json)}');
    }
    final code = json['code'];
    final message = json['message'];
    if (code is! int) {
      throw FormatException(
          'invalid error object: code must be an integer, got ${_describe(code)}');
    }
    if (message is! String) {
      throw FormatException(
          'invalid error object: message must be a string, got ${_describe(message)}');
    }
    return ResponseError(
        code: code,
        message: message,
        data: json.containsKey('data') ? json['data'] : null);
  }

  /// Serializes to a JSON-ready map, omitting `data` when absent.
  Map<String, Object?> toJson() {
    final map = <String, Object?>{'code': code, 'message': message};
    if (data != null) {
      map['data'] = data;
    }
    return map;
  }

  @override
  bool operator ==(Object other) =>
      other is ResponseError &&
      other.code == code &&
      other.message == message &&
      jsonValuesEqual(other.data, data);

  @override
  int get hashCode => Object.hash(code, message, jsonValueHash(data));

  @override
  String toString() => 'JSON-RPC error $code: $message';
}

/// One-word shape description for error texts.
String _describe(Object? value) {
  if (value == null) return 'null';
  if (value is String) return 'string';
  if (value is int) return 'integer';
  if (value is double) return 'number';
  if (value is bool) return 'boolean';
  if (value is List) return 'array';
  if (value is Map) return 'object';
  return 'value';
}
