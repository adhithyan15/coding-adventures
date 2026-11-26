/// [MessageWriter] — writes Content-Length-framed JSON-RPC messages to
/// an in-memory byte buffer.
///
/// For each message: serialize the [Message] to JSON bytes, write
/// `"Content-Length: N\r\n\r\n"` with the BYTE length (never the
/// character count), then the payload. Read the accumulated output back
/// with [MessageWriter.bytes].
library;

import 'dart:convert';
import 'dart:typed_data';

import 'json_values.dart';
import 'message.dart';

/// Writes framed JSON-RPC messages into a byte buffer.
final class MessageWriter {
  MessageWriter() : _buffer = BytesBuilder();

  final BytesBuilder _buffer;

  /// A copy of every byte written so far.
  Uint8List get bytes => Uint8List.fromList(_buffer.toBytes());

  /// Serializes [message] and writes it with Content-Length framing.
  ///
  /// Values that `dart:convert` cannot encode (non-finite doubles,
  /// non-JSON objects) propagate its [JsonUnsupportedObjectError] — the
  /// in-memory analogue of the reference's serialization failure. Lone
  /// surrogates fail closed with [ArgumentError], since they would
  /// corrupt any downstream UTF-8 encoding.
  void writeMessage(Message message) {
    final json = messageToJson(message);
    try {
      checkNoLoneSurrogates(json);
    } on FormatException {
      throw ArgumentError.value(
          message, 'message', 'contains a lone surrogate');
    }
    writeRaw(utf8.encode(jsonEncode(json)));
  }

  /// Writes a raw [payload] with Content-Length framing.
  ///
  /// Useful when the caller already serialized the payload or wants a
  /// custom JSON body.
  void writeRaw(List<int> payload) {
    final header = ascii.encode('Content-Length: ${payload.length}\r\n\r\n');
    _buffer.add(header);
    _buffer.add(payload);
  }
}
