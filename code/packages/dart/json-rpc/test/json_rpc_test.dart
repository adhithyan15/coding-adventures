/// Unit tests for `coding_adventures_json_rpc`: the 5 reference server
/// tests plus the full matrix the reference leaves untested — message
/// discrimination, error values, framing edges, writer exactness, and
/// value semantics.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_json_rpc/json_rpc.dart';
import 'package:test/test.dart';

Uint8List bytes(String text) => Uint8List.fromList(utf8.encode(text));

/// Frames [json] exactly like the writer does.
Uint8List frame(String json) {
  final payload = utf8.encode(json);
  return Uint8List.fromList([
    ...ascii.encode('Content-Length: ${payload.length}\r\n\r\n'),
    ...payload,
  ]);
}

/// Builds a server over framed [input], runs [setup], serves to EOF,
/// and returns the writer for output inspection.
(MessageReader, MessageWriter, Server) runServer(
    Uint8List input, void Function(Server server) setup) {
  final reader = MessageReader(input);
  final writer = MessageWriter();
  final server = Server(reader: reader, writer: writer);
  setup(server);
  server.serve();
  return (reader, writer, server);
}

void main() {
  group('server', () {
    test('known request is dispatched', () {
      Object? seenId;
      Object? seenParams;
      final (_, writer, _) = runServer(
          frame('{"jsonrpc":"2.0","id":1,"method":"ping","params":{"a":1}}'),
          (server) => server.onRequest('ping', (id, params) {
                seenId = id;
                seenParams = params;
                return 'pong';
              }));
      expect(seenId, 1);
      expect(seenParams, {'a': 1});
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      final response = messages[0] as Response;
      expect(response.id, 1);
      expect(response.result, 'pong');
      expect(response.error, isNull);
    });

    test('unknown request answers method not found', () {
      final (_, writer, _) = runServer(
          frame('{"jsonrpc":"2.0","id":2,"method":"unknown"}'), (_) {});
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      final response = messages[0] as Response;
      expect(response.id, 2);
      expect(response.error!.code, methodNotFoundCode);
      expect(response.error!.data, 'unknown');
    });

    test('notification produces no response', () {
      var notified = false;
      Object? seenParams;
      final (_, writer, _) = runServer(
          frame('{"jsonrpc":"2.0","method":"ping","params":[1]}'),
          (server) => server.onNotification('ping', (params) {
                notified = true;
                seenParams = params;
              }));
      expect(writer.bytes, isEmpty);
      expect(notified, isTrue);
      expect(seenParams, [1]);
    });

    test('unknown notification is silent', () {
      final (_, writer, _) =
          runServer(frame('{"jsonrpc":"2.0","method":"unknown"}'), (_) {});
      expect(writer.bytes, isEmpty);
    });

    test('handler error is returned', () {
      final (_, writer, _) = runServer(
          frame('{"jsonrpc":"2.0","id":3,"method":"fail"}'),
          (server) => server.onRequest('fail', (id, params) {
                throw ResponseError.invalidParams('bad');
              }));
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      final response = messages[0] as Response;
      expect(response.id, 3);
      expect(response.error!.code, invalidParamsCode);
      expect(response.error!.data, 'bad');
    });

    test('framing error answers null-id parse error', () {
      final (_, writer, _) = runServer(
          Uint8List.fromList([
            ...ascii.encode('Content-Length: 5\r\n\r\n'),
            ...ascii.encode('abc'), // 2 bytes short
          ]),
          (_) {});
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      final response = messages[0] as Response;
      expect(response.id, isNull);
      expect(response.error!.code, parseErrorCode);
    });

    test('panicking request handler answers internal error', () {
      final (_, writer, _) = runServer(
          frame('{"jsonrpc":"2.0","id":4,"method":"boom"}'),
          (server) => server.onRequest('boom', (id, params) {
                throw StateError('handler blew up');
              }));
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      final response = messages[0] as Response;
      expect(response.id, 4);
      expect(response.error!.code, internalErrorCode);
      expect(response.error!.data, 'handler panicked');
    });

    test('panicking notification handler is swallowed', () {
      final input = Uint8List.fromList([
        ...frame('{"jsonrpc":"2.0","method":"boom"}'),
        ...frame('{"jsonrpc":"2.0","id":5,"method":"ping"}'),
      ]);
      final (_, writer, _) = runServer(input, (server) {
        server.onNotification('boom', (_) => throw StateError('x'));
        server.onRequest('ping', (id, params) => 'pong');
      });
      // The notification wrote nothing, the later request still answered.
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      expect((messages[0] as Response).result, 'pong');
    });

    test('incoming responses are ignored', () {
      final (_, writer, _) =
          runServer(frame('{"jsonrpc":"2.0","id":1,"result":"late"}'), (_) {});
      expect(writer.bytes, isEmpty);
    });

    test('re-registering replaces the handler', () {
      final (_, writer, _) =
          runServer(frame('{"jsonrpc":"2.0","id":1,"method":"m"}'), (server) {
        server.onRequest('m', (id, params) => 'first');
        server.onRequest('m', (id, params) => 'second');
      });
      expect((parseAll(writer.bytes)[0] as Response).result, 'second');
    });

    test('unencodable handler output is dropped, loop survives', () {
      final input = Uint8List.fromList([
        ...frame('{"jsonrpc":"2.0","id":1,"method":"nan"}'),
        ...frame('{"jsonrpc":"2.0","id":2,"method":"ping"}'),
      ]);
      final (_, writer, _) = runServer(input, (server) {
        server.onRequest('nan', (id, params) => double.nan);
        server.onRequest('ping', (id, params) => 'pong');
      });
      // The NaN response failed serialization and vanished; ping answered.
      final messages = parseAll(writer.bytes);
      expect(messages.length, 1);
      final response = messages[0] as Response;
      expect(response.id, 2);
      expect(response.result, 'pong');
    });

    test('serve stops at clean EOF', () {
      final (reader, writer, _) = runServer(Uint8List(0), (_) {});
      expect(writer.bytes, isEmpty);
      expect(reader.isAtEnd, isTrue);
    });
  });

  group('messages', () {
    test('parses a request', () {
      final message = parseMessage(bytes(
          '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":null}}'));
      expect(message,
          Request(id: 1, method: 'initialize', params: {'rootUri': null}));
    });

    test('parses a success response', () {
      final message =
          parseMessage(bytes('{"jsonrpc":"2.0","id":1,"result":{"ok":true}}'));
      expect(message, Response(id: 1, result: {'ok': true}));
    });

    test('parses an error response', () {
      final message = parseMessage(bytes(
          '{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}'));
      expect(
          message,
          Response(
              id: 1,
              error: ResponseError(
                  code: methodNotFoundCode, message: 'Method not found')));
    });

    test('parses a notification', () {
      final message = parseMessage(
          bytes('{"jsonrpc":"2.0","method":"textDocument/didChange"}'));
      expect(message, Notification(method: 'textDocument/didChange'));
    });

    test('string ids survive', () {
      final message =
          parseMessage(bytes('{"jsonrpc":"2.0","id":"abc","method":"ping"}'));
      expect((message as Request).id, 'abc');
    });

    test('params accept any JSON type', () {
      // The reference clones params unvalidated — even scalars.
      final scalar = parseMessage(
          bytes('{"jsonrpc":"2.0","id":1,"method":"m","params":5}'));
      expect((scalar as Request).params, 5);
      final array = parseMessage(
          bytes('{"jsonrpc":"2.0","method":"m","params":[1,"x"]}'));
      expect((array as Notification).params, [1, 'x']);
    });

    test('null request id parses unenforced', () {
      // Documented as invalid but never checked — reference parity.
      final message =
          parseMessage(bytes('{"jsonrpc":"2.0","id":null,"method":"m"}'));
      expect((message as Request).id, isNull);
    });

    test('missing response id reads null', () {
      final message = parseMessage(bytes('{"jsonrpc":"2.0","result":1}'));
      expect((message as Response).id, isNull);
    });

    test('result and error together are both kept', () {
      final message = parseMessage(bytes(
          '{"jsonrpc":"2.0","id":1,"result":1,"error":{"code":-32603,"message":"Internal error"}}'));
      final response = message as Response;
      expect(response.result, 1);
      expect(response.error!.code, internalErrorCode);
    });

    test('explicit null result reads null', () {
      final message =
          parseMessage(bytes('{"jsonrpc":"2.0","id":1,"result":null}'));
      final response = message as Response;
      expect(response.result, isNull);
      expect(response.error, isNull);
    });

    test('non-string methods are invalid requests', () {
      expect(() => parseMessage(bytes('{"jsonrpc":"2.0","id":1,"method":7}')),
          throwsA(ResponseError.invalidRequest('method must be a string')));
      expect(() => parseMessage(bytes('{"jsonrpc":"2.0","method":null}')),
          throwsA(ResponseError.invalidRequest('method must be a string')));
    });

    test('non-object top levels are invalid requests', () {
      for (final text in ['[]', '1', '"x"', 'true', 'null']) {
        expect(() => parseMessage(bytes(text)),
            throwsA(ResponseError.invalidRequest('expected JSON object')),
            reason: text);
      }
    });

    test('fieldless objects name the missing fields', () {
      expect(
          () => parseMessage(bytes('{"jsonrpc":"2.0"}')),
          throwsA(ResponseError.invalidRequest(
              "missing 'method', 'result', or 'error' field")));
    });

    test('malformed JSON is a parse error', () {
      final error = throwsResponseError(bytes('not json'));
      expect(error.code, parseErrorCode);
      expect(error.data, isA<String>());
    });

    test('malformed UTF-8 is a parse error', () {
      final error = throwsResponseError(Uint8List.fromList([0xFF, 0xFE]));
      expect(error.code, parseErrorCode);
    });

    test('BOM-prefixed JSON is a parse error', () {
      final error = throwsResponseError(
          Uint8List.fromList([0xEF, 0xBB, 0xBF, ...utf8.encode('{}')]));
      expect(error.code, parseErrorCode);
      expect(error.data, 'unexpected byte-order mark');
    });

    test('out-of-range integers fail closed', () {
      for (final literal in ['18446744073709551615', '-9223372036854775809']) {
        final error = throwsResponseError(
            bytes('{"jsonrpc":"2.0","id":$literal,"method":"m"}'));
        expect(error.code, parseErrorCode, reason: literal);
      }
      // int64 extremes still parse.
      expect(
          (parseMessage(bytes(
                      '{"jsonrpc":"2.0","id":9223372036854775807,"method":"m"}'))
                  as Request)
              .id,
          9223372036854775807);
    });

    test('lone surrogates fail closed', () {
      final error = throwsResponseError(
          bytes('{"jsonrpc":"2.0","id":1,"method":"\\ud800"}'));
      expect(error.code, parseErrorCode);
    });

    test('deep nesting fails closed at the serde limit', () {
      final deep = '${'[' * 200}${']' * 200}';
      final error = throwsResponseError(
          bytes('{"jsonrpc":"2.0","id":1,"method":"m","params":$deep}'));
      expect(error.code, parseErrorCode);
      expect(error.data, 'JSON nesting too deep');
      // Exactly at the limit still parses.
      final ok = '${'[' * 127}1${']' * 127}';
      final message = parseMessage(
          bytes('{"jsonrpc":"2.0","id":1,"method":"m","params":$ok}'));
      expect(message, isA<Request>());
    });

    test('doubles with big fractions parse', () {
      // The integer scan must skip whole doubles, not re-scan their
      // tails as integers.
      for (final literal in [
        '1.9223372036854775808',
        '1e9223372036854775808',
        '-2.9223372036854775808',
        '1.5e-3',
      ]) {
        final message =
            parseMessage(bytes('{"jsonrpc":"2.0","id":$literal,"method":"m"}'));
        expect((message as Request).id, isA<double>(), reason: literal);
      }
    });

    test('the jsonrpc marker is never checked', () {
      expect(parseMessage(bytes('{"id":1,"method":"m"}')), isA<Request>());
      expect(parseMessage(bytes('{"jsonrpc":"1.0","id":1,"method":"m"}')),
          isA<Request>());
    });

    test('invalid error objects are parse errors', () {
      final bad = [
        '{"jsonrpc":"2.0","id":1,"error":null}',
        '{"jsonrpc":"2.0","id":1,"error":5}',
        '{"jsonrpc":"2.0","id":1,"error":{"code":1.5,"message":"m"}}',
        '{"jsonrpc":"2.0","id":1,"error":{"code":"-32603","message":"m"}}',
        '{"jsonrpc":"2.0","id":1,"error":{"code":-32603}}',
        '{"jsonrpc":"2.0","id":1,"error":{"message":"m"}}',
      ];
      for (final text in bad) {
        final error = throwsResponseError(bytes(text));
        expect(error.code, parseErrorCode, reason: text);
        expect(
            (error.data as String).startsWith('invalid error object: '), isTrue,
            reason: text);
      }
    });

    test('error objects ignore extra fields', () {
      final message = parseMessage(bytes(
          '{"jsonrpc":"2.0","id":1,"error":{"code":-32603,"message":"m","x":1}}'));
      expect((message as Response).error!.code, internalErrorCode);
    });
  });

  group('messageToJson', () {
    test('request round-trips with fixed key order', () {
      const request = Request(id: 1, method: 'm', params: {
        'a': [1, null]
      });
      final text = jsonEncode(messageToJson(request));
      expect(text,
          '{"jsonrpc":"2.0","id":1,"method":"m","params":{"a":[1,null]}}');
      expect(parseMessage(bytes(text)), request);
    });

    test('request without params omits the key', () {
      const request = Request(id: 'a', method: 'm');
      expect(jsonEncode(messageToJson(request)),
          '{"jsonrpc":"2.0","id":"a","method":"m"}');
    });

    test('success response round-trips', () {
      const response = Response(id: 1, result: {'ok': true});
      final text = jsonEncode(messageToJson(response));
      expect(text, '{"jsonrpc":"2.0","id":1,"result":{"ok":true}}');
      expect(parseMessage(bytes(text)), response);
    });

    test('error response round-trips with data omitted when absent', () {
      final withData =
          Response(id: 1, error: ResponseError.methodNotFound('m'));
      expect(jsonEncode(messageToJson(withData)),
          '{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found","data":"m"}}');
      expect(
          parseMessage(bytes(jsonEncode(messageToJson(withData)))), withData);
      final withoutData = Response(
          id: null,
          error: ResponseError(code: internalErrorCode, message: 'x'));
      expect(jsonEncode(messageToJson(withoutData)),
          '{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"x"}}');
    });

    test('error wins over result on the wire', () {
      final both = Response(
          id: 1,
          result: 'ignored',
          error: ResponseError(code: internalErrorCode, message: 'x'));
      expect(jsonEncode(messageToJson(both)),
          '{"jsonrpc":"2.0","id":1,"error":{"code":-32603,"message":"x"}}');
    });

    test('null result emits explicit null', () {
      const response = Response(id: 1);
      expect(jsonEncode(messageToJson(response)),
          '{"jsonrpc":"2.0","id":1,"result":null}');
    });

    test('notification round-trips', () {
      const notification = Notification(method: 'm', params: [1]);
      final text = jsonEncode(messageToJson(notification));
      expect(text, '{"jsonrpc":"2.0","method":"m","params":[1]}');
      expect(parseMessage(bytes(text)), notification);
    });
  });

  group('errors', () {
    test('factories carry codes, messages, and data', () {
      expect(ResponseError.parseError(),
          ResponseError(code: -32700, message: 'Parse error'));
      expect(ResponseError.parseError('d').data, 'd');
      expect(ResponseError.invalidRequest().code, invalidRequestCode);
      expect(ResponseError.invalidRequest().message, 'Invalid Request');
      expect(ResponseError.methodNotFound('m'),
          ResponseError(code: -32601, message: 'Method not found', data: 'm'));
      expect(ResponseError.invalidParams().code, invalidParamsCode);
      expect(ResponseError.invalidParams('d').data, 'd');
      expect(ResponseError.internalError().code, internalErrorCode);
      expect(ResponseError.internalError().message, 'Internal error');
    });

    test('constants match the spec table', () {
      expect(parseErrorCode, -32700);
      expect(invalidRequestCode, -32600);
      expect(methodNotFoundCode, -32601);
      expect(invalidParamsCode, -32602);
      expect(internalErrorCode, -32603);
    });

    test('fromJson requires an object, int code, string message', () {
      expect(
          ResponseError.fromJson({
            'code': -32603,
            'message': 'm',
            'data': [1],
            'x': 0
          }),
          ResponseError(code: -32603, message: 'm', data: [1]));
      expect(ResponseError.fromJson({'code': 1, 'message': 'm'}).data, isNull);
      for (final bad in [
        null,
        5,
        'x',
        [],
        {'code': 1.5, 'message': 'm'},
        {'code': true, 'message': 'm'},
        {'code': {}, 'message': 'm'},
        {'code': 1},
        {'message': 'm'},
        {'code': 1, 'message': 2},
      ]) {
        expect(() => ResponseError.fromJson(bad), throwsFormatException,
            reason: '$bad');
      }
    });

    test('toJson omits absent data', () {
      expect(ResponseError(code: 1, message: 'm', data: {'a': 1}).toJson(), {
        'code': 1,
        'message': 'm',
        'data': {'a': 1}
      });
      expect(ResponseError(code: 1, message: 'm').toJson(),
          {'code': 1, 'message': 'm'});
    });

    test('errors compare by value and render the reference text', () {
      const a = ResponseError(code: 1, message: 'm', data: {
        'a': [1]
      });
      expect(
          a,
          ResponseError(code: 1, message: 'm', data: {
            'a': [1]
          }));
      expect(
          a.hashCode,
          ResponseError(code: 1, message: 'm', data: {
            'a': [1]
          }).hashCode);
      expect(a.toString(), 'JSON-RPC error 1: m');
      expect(a, isA<Exception>());
      expect(a, isNot(ResponseError(code: 2, message: 'm')));
      expect(
          a,
          isNot(ResponseError(code: 1, message: 'm', data: {
            'a': [2]
          })));
      // int 1 and double 1.0 differ, mirroring serde number equality.
      expect(ResponseError(code: 1, message: 'm', data: 1),
          isNot(ResponseError(code: 1, message: 'm', data: 1.0)));
      expect(a, isNot('not an error'));
    });
  });

  group('reader', () {
    test('reads a framed message and tracks the cursor', () {
      final input = frame('{"jsonrpc":"2.0","id":1,"method":"ping"}');
      final reader = MessageReader(input);
      expect(reader.isAtEnd, isFalse);
      final message = reader.readMessage();
      expect(message, Request(id: 1, method: 'ping'));
      expect(reader.bytesConsumed, input.length);
      expect(reader.isAtEnd, isTrue);
      expect(reader.readMessage(), isNull);
    });

    test('reads back-to-back messages', () {
      final input = Uint8List.fromList([
        ...frame('{"jsonrpc":"2.0","id":1,"method":"a"}'),
        ...frame('{"jsonrpc":"2.0","method":"b"}'),
      ]);
      final reader = MessageReader(input);
      expect(reader.readMessage(), isA<Request>());
      expect(reader.readMessage(), isA<Notification>());
      expect(reader.readMessage(), isNull);
    });

    test('readRaw returns payload bytes verbatim', () {
      final reader = MessageReader(frame('{"a":1}'));
      expect(utf8.decode(reader.readRaw()!), '{"a":1}');
      expect(reader.readRaw(), isNull);
    });

    test('empty input is clean EOF', () {
      expect(MessageReader(Uint8List(0)).readMessage(), isNull);
      expect(MessageReader(Uint8List(0)).readRaw(), isNull);
    });

    test('EOF mid-headers names the condition', () {
      final reader = MessageReader(bytes('Content-Length: 2\r\n'));
      final error = throwsReadError(reader);
      expect(error.code, parseErrorCode);
      expect(error.data, 'EOF in header block');
    });

    test('unterminated header line is EOF mid-headers', () {
      final reader = MessageReader(bytes('Content-Length: 2'));
      final error = throwsReadError(reader);
      expect(error.code, parseErrorCode);
      expect(error.data, 'EOF in header block');
    });

    test('missing content length names the condition', () {
      final reader =
          MessageReader(bytes('Content-Type: application/json\r\n\r\n{}'));
      final error = throwsReadError(reader);
      expect(error.code, parseErrorCode);
      expect(error.data, 'missing Content-Length header');
    });

    test('malformed lengths echo the raw value', () {
      for (final value in [
        'nope',
        '',
        '+5',
        '5x',
        '-3',
        '99999999999999999999999'
      ]) {
        final reader = MessageReader(bytes('Content-Length: $value\r\n\r\n{}'));
        final error = throwsReadError(reader);
        expect(error.code, parseErrorCode, reason: '[$value]');
        expect(error.data, "invalid Content-Length value: '$value'",
            reason: '[$value]');
      }
    });

    test('length tolerates surrounding spaces', () {
      final reader = MessageReader(bytes('Content-Length:   2  \r\n\r\n{}'));
      expect(utf8.decode(reader.readRaw()!), '{}');
    });

    test('prefix requires the exact space', () {
      // `Content-Length:5` is some other header per the reference's
      // case-sensitive `strip_prefix("Content-Length: ")`.
      final reader = MessageReader(bytes('Content-Length:5\r\n\r\n{}'));
      final error = throwsReadError(reader);
      expect(error.data, 'missing Content-Length header');
    });

    test('prefix is case-sensitive', () {
      final reader = MessageReader(bytes('content-length: 2\r\n\r\n{}'));
      final error = throwsReadError(reader);
      expect(error.data, 'missing Content-Length header');
    });

    test('other headers are ignored', () {
      final reader = MessageReader(bytes(
          'Content-Type: application/json\r\nContent-Length: 2\r\nX: y\r\n\r\n{}'));
      expect(utf8.decode(reader.readRaw()!), '{}');
    });

    test('last content length wins', () {
      final reader = MessageReader(
          bytes('Content-Length: 99\r\nContent-Length: 2\r\n\r\n{}'));
      expect(utf8.decode(reader.readRaw()!), '{}');
    });

    test('near-int64 length cannot overflow the bounds check', () {
      // Security review: `_cursor + contentLength` would wrap negative
      // and bypass into a RangeError. The subtracted form fails closed.
      for (final length in ['9223372036854775807', '9223372036854775806']) {
        final reader =
            MessageReader(bytes('Content-Length: $length\r\n\r\n{}'));
        final error = throwsReadError(reader);
        expect(error.code, parseErrorCode, reason: length);
        expect(error.data,
            'I/O error reading payload: failed to fill whole buffer');
      }
    });

    test('short payload names the condition', () {
      final reader = MessageReader(bytes('Content-Length: 5\r\n\r\nabc'));
      final error = throwsReadError(reader);
      expect(error.code, parseErrorCode);
      expect(
          error.data, 'I/O error reading payload: failed to fill whole buffer');
      // The short bytes were consumed: no retry, clean EOF next.
      expect(reader.readRaw(), isNull);
    });

    test('non-UTF8 headers name the condition', () {
      final reader = MessageReader(
          Uint8List.fromList([0xFF, ...utf8.encode('\r\n\r\n{}')]));
      final error = throwsReadError(reader);
      expect(error.code, parseErrorCode);
      expect(error.data,
          'I/O error reading header: stream did not contain valid UTF-8');
    });

    test('BOM-prefixed header is not a content length', () {
      // The BOM survives decoding (like `read_line` into a String), so
      // the prefix match fails and the header is ignored.
      final reader = MessageReader(Uint8List.fromList([
        0xEF,
        0xBB,
        0xBF,
        ...utf8.encode('Content-Length: 2\r\n\r\n{}'),
      ]));
      final error = throwsReadError(reader);
      expect(error.data, 'missing Content-Length header');
    });

    test('double carriage return still ends headers', () {
      // `trim_end_matches` strips every trailing `\r`, so even
      // `"\r\r\n"` is a blank line — reference quirk, pinned.
      final reader = MessageReader(bytes('Content-Length: 2\r\n\r\r\n{}'));
      expect(utf8.decode(reader.readRaw()!), '{}');
    });

    test('LF-only framing works', () {
      final reader = MessageReader(bytes('Content-Length: 2\n\n{}'));
      expect(utf8.decode(reader.readRaw()!), '{}');
    });

    test('long carriage-return runs strip linearly and correctly', () {
      final carriageReturns = List.filled(5000, '\r').join();
      final reader =
          MessageReader(bytes('Content-Length: 2$carriageReturns\n\r\n{}'));
      expect(utf8.decode(reader.readRaw()!), '{}');
    });

    test('zero-length payload reads empty, not EOF', () {
      final reader = MessageReader(bytes('Content-Length: 0\r\n\r\n'));
      final raw = reader.readRaw();
      expect(raw, isNotNull);
      expect(raw, isEmpty);
    });
  });

  group('writer', () {
    test('writes exact framed bytes', () {
      final writer = MessageWriter();
      writer.writeMessage(const Request(id: 1, method: 'ping'));
      expect(utf8.decode(writer.bytes),
          'Content-Length: 40\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"ping"}');
    });

    test('length counts bytes, not characters', () {
      final writer = MessageWriter();
      writer.writeMessage(const Notification(method: 'm', params: 'é'));
      final text = utf8.decode(writer.bytes);
      // 'é' is 2 bytes: 43 characters + 1 extra byte = 44.
      expect(text.startsWith('Content-Length: 44\r\n\r\n'), isTrue);
    });

    test('writeRaw frames verbatim', () {
      final writer = MessageWriter();
      writer.writeRaw(utf8.encode('{}'));
      expect(utf8.decode(writer.bytes), 'Content-Length: 2\r\n\r\n{}');
    });

    test('writes accumulate and bytes detach', () {
      final writer = MessageWriter();
      writer.writeRaw([1]);
      writer.writeRaw([2, 3]);
      final first = writer.bytes;
      expect(
          first.length,
          'Content-Length: 1\r\n\r\n'.length +
              1 +
              'Content-Length: 2\r\n\r\n'.length +
              2);
      first[0] = 0xFF;
      expect(writer.bytes[0], utf8.encode('C')[0]);
    });

    test('unencodable values propagate the codec error', () {
      final writer = MessageWriter();
      expect(
          () => writer.writeMessage(const Response(id: 1, result: double.nan)),
          throwsA(isA<JsonUnsupportedObjectError>()));
      expect(
          () =>
              writer.writeMessage(Response(id: 1, result: DateTime.utc(2026))),
          throwsA(isA<JsonUnsupportedObjectError>()));
    });

    test('lone surrogates fail closed with ArgumentError', () {
      final writer = MessageWriter();
      expect(
          () => writer.writeMessage(
              Notification(method: String.fromCharCodes([0xD800]))),
          throwsArgumentError);
    });

    test('deep values fail closed with ArgumentError', () {
      Object? deep = 1;
      for (var i = 0; i < 200; i++) {
        deep = [deep];
      }
      final writer = MessageWriter();
      final value = deep;
      expect(() => writer.writeMessage(Response(id: 1, result: value)),
          throwsArgumentError);
    });
  });

  group('values', () {
    test('messages compare by value across variants', () {
      const request = Request(id: 1, method: 'm', params: {
        'a': [1]
      });
      expect(
          request,
          Request(id: 1, method: 'm', params: {
            'a': [1]
          }));
      expect(
          request.hashCode,
          Request(id: 1, method: 'm', params: {
            'a': [1]
          }).hashCode);
      expect(request.toString(), contains('m'));
      expect(request, isNot(Request(id: 2, method: 'm')));
      expect(request, isNot(Notification(method: 'm')));
      expect(request, isNot('not a message'));

      const response = Response(id: 1, result: [1]);
      expect(response, Response(id: 1, result: [1]));
      expect(response.hashCode, Response(id: 1, result: [1]).hashCode);
      expect(response.toString(), contains('1'));
      expect(response, isNot(Response(id: 1, result: [2])));
      expect(response, isNot('not a message'));

      const notification = Notification(method: 'm');
      expect(notification, Notification(method: 'm'));
      expect(notification.hashCode, Notification(method: 'm').hashCode);
      expect(notification.toString(), contains('m'));
      expect(notification, isNot(Notification(method: 'n')));
      expect(notification, isNot('not a message'));
    });
  });
}

/// Runs [reader.readMessage] expecting a [ResponseError] throw.
ResponseError throwsReadError(MessageReader reader) {
  try {
    reader.readMessage();
  } on ResponseError catch (e) {
    return e;
  }
  fail('expected ResponseError, read fine');
}

ResponseError throwsResponseError(Uint8List input) {
  try {
    parseMessage(input);
  } on ResponseError catch (e) {
    return e;
  }
  fail('expected ResponseError, parsed fine');
}

/// Parses every framed message in [output].
List<Message> parseAll(Uint8List output) {
  final messages = <Message>[];
  final reader = MessageReader(output);
  while (true) {
    final message = reader.readMessage();
    if (message == null) return messages;
    messages.add(message);
  }
}
