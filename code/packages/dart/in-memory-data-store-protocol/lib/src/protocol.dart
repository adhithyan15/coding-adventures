/// In-memory data store protocol frames: the Dart lane.
///
/// The bridge between RESP wire values and engine commands: a [CommandFrame]
/// is an uppercased command name plus raw byte arguments, converted to and
/// from RESP arrays of bulk strings. Binary-safe throughout — arguments are
/// bytes, never decoded except by explicit text helpers.
///
/// ```dart
/// import 'package:coding_adventures_in_memory_data_store_protocol/coding_adventures_in_memory_data_store_protocol.dart';
/// import 'package:coding_adventures_resp_protocol/coding_adventures_resp_protocol.dart';
///
/// void main() {
///   final frame = CommandFrame('set', [utf8.encode('k'), utf8.encode('v')]);
///   print(frame.command); // SET
///   final wire = encode(commandToResp(frame));
///   final back = commandFromResp(decode(wire)!.value)!;
///   assert(back.command == 'SET');
/// }
/// ```
library in_memory_data_store_protocol;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_resp_protocol/coding_adventures_resp_protocol.dart'
    as resp;

/// A store command: uppercased name plus raw byte arguments.
class CommandFrame {
  /// Uppercase command name (`SET`, `GET`, ...).
  final String command;

  /// Raw argument bytes (copied on the way in).
  final List<Uint8List> args;

  /// Builds a frame, uppercasing [command] and copying [args].
  factory CommandFrame(String command, List<Uint8List> args) =>
      CommandFrame._raw(
        command.trim().toUpperCase(),
        [for (final arg in args) Uint8List.fromList(arg)],
      );

  const CommandFrame._raw(this.command, this.args);

  /// Builds a frame from wire parts (first part is the command).
  ///
  /// Returns null for an empty part list. The command uppercases ASCII
  /// bytes; arguments are copied.
  static CommandFrame? fromParts(List<Uint8List> parts) {
    if (parts.isEmpty) return null;
    return CommandFrame._raw(
      _asciiUpper(parts.first),
      [for (final arg in parts.skip(1)) Uint8List.fromList(arg)],
    );
  }

  @override
  bool operator ==(Object other) {
    if (other is! CommandFrame) return false;
    if (command != other.command || args.length != other.args.length) {
      return false;
    }
    for (var i = 0; i < args.length; i++) {
      if (!_bytesEqual(args[i], other.args[i])) return false;
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(command, args.length);

  @override
  String toString() => 'CommandFrame($command, ${args.length} args)';
}

bool _bytesEqual(Uint8List a, Uint8List b) {
  if (identical(a, b)) return true;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Uppercases ASCII letters byte-wise (non-ASCII bytes pass through as
/// latin-1 code points, matching the byte-oriented sibling lanes).
String _asciiUpper(Uint8List bytes) {
  final out = StringBuffer();
  for (final byte in bytes) {
    out.writeCharCode(byte >= 0x61 && byte <= 0x7a ? byte - 0x20 : byte);
  }
  return out.toString();
}

/// Converts a RESP array value into a command frame.
///
/// Returns null for null/empty arrays and for elements that are not
/// scalar (nested arrays and null bulks have no byte form).
CommandFrame? commandFromResp(resp.RespValue value) {
  if (value is! resp.RespArray || value.value == null) return null;
  final elements = value.value!;
  if (elements.isEmpty) return null;
  final parts = <Uint8List>[];
  for (final element in elements) {
    final part = respValueToBytes(element);
    if (part == null) return null;
    parts.add(part);
  }
  return CommandFrame.fromParts(parts);
}

/// Converts a command frame into a RESP array of bulk strings.
resp.RespArray commandToResp(CommandFrame command) => resp.array([
      resp.bulkString(command.command),
      for (final arg in command.args) resp.bulkString(arg),
    ]);

/// Renders a scalar RESP value as bytes, or null when it has no byte form.
///
/// Simple strings and errors encode as UTF-8, integers as decimal text,
/// bulk strings pass through (null stays null), and arrays return null.
Uint8List? respValueToBytes(resp.RespValue value) {
  switch (value) {
    case resp.RespSimpleString(:final value):
      return Uint8List.fromList(utf8.encode(value));
    case resp.RespErrorValue(:final value):
      return Uint8List.fromList(utf8.encode(value));
    case resp.RespInteger(:final value):
      return Uint8List.fromList(utf8.encode(value.toString()));
    case resp.RespBulkString(:final value):
      return value == null ? null : Uint8List.fromList(value);
    case resp.RespArray():
      return null;
  }
}

/// Builds a protocol error value carrying [message].
resp.RespErrorValue protocolError(String message) => resp.errorValue(message);

/// Parses text parts into a frame (first part is the command).
///
/// Throws [ArgumentError] on an empty part list.
CommandFrame parseFrameText(List<String> parts) {
  if (parts.isEmpty) {
    throw ArgumentError('command frame cannot be empty');
  }
  return CommandFrame(parts.first, [
    for (final part in parts.skip(1)) Uint8List.fromList(utf8.encode(part)),
  ]);
}

/// Formats a frame as space-joined text (arguments decoded as UTF-8).
String formatFrame(CommandFrame frame) => [
      frame.command,
      for (final arg in frame.args) utf8.decode(arg),
    ].join(' ');
