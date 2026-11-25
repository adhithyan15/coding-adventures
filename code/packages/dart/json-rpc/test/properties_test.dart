/// Property tests for `coding_adventures_json_rpc`, using the in-lane
/// `pbt` package: the fail-closed vocabulary over hostile bytes,
/// message and framing round-trips, the server dispatch model, error
/// round-trips, id preservation, and purity — all with fixed seeds.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_json_rpc/json_rpc.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<String> tokenGen(int minLen, int maxLen) {
  const alphabet =
      'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.~';
  return Gen.list(minLen, maxLen,
          Gen.integer(BigInt.zero, BigInt.from(alphabet.length - 1)))
      .map((idx) => idx.map((i) => alphabet[i.toInt()]).join());
}

/// Generates JSON values up to [depth] deep (objects, arrays, strings,
/// small ints, booleans, null — no doubles, no astral, no lone
/// surrogates, so every sample is unambiguously encodable).
Gen<Object?> jsonGen(int depth) {
  final scalar = Gen.oneOf([
    Gen.constant(null),
    Gen.boolean(),
    Gen.integer(BigInt.from(-1000000), BigInt.from(1000000))
        .map((i) => i.toInt()),
    tokenGen(0, 12),
  ]);
  if (depth <= 0) return scalar;
  return Gen.oneOf([
    scalar,
    Gen.list(0, 4, jsonGen(depth - 1)).map((l) => l),
    Gen.list(0, 4, Gen.tuple2(tokenGen(1, 8), jsonGen(depth - 1)))
        .map((pairs) => {for (final p in pairs) p.$1: p.$2}),
  ]);
}

Gen<Object?> idGen() => Gen.oneOf([
      Gen.integer(BigInt.from(-1000000), BigInt.from(1000000))
          .map((i) => i.toInt()),
      tokenGen(1, 12).map((s) => s),
    ]);

Gen<Message> messageGen() {
  final request = Gen.tuple3(idGen(), tokenGen(1, 16), jsonGen(2))
      .map((t) => Request(id: t.$1, method: 'm/${t.$2}', params: t.$3));
  final response = Gen.tuple2(idGen(), jsonGen(2))
      .map((t) => Response(id: t.$1, result: t.$2));
  final errorResponse = Gen.tuple3(
    idGen(),
    Gen.integer(BigInt.from(-32700), BigInt.from(-32000)).map((i) => i.toInt()),
    Gen.tuple2(tokenGen(1, 12), jsonGen(1)),
  ).map((t) => Response(
      id: t.$1,
      error: ResponseError(code: t.$2, message: t.$3.$1, data: t.$3.$2)));
  final notification = Gen.tuple2(tokenGen(1, 16), jsonGen(2))
      .map((t) => Notification(method: 'n/${t.$1}', params: t.$2));
  return Gen.oneOf([request, response, errorResponse, notification]);
}

Uint8List frameMessage(Message message) {
  final writer = MessageWriter();
  writer.writeMessage(message);
  return writer.bytes;
}

void main() {
  group('properties', () {
    test('readers fail closed on hostile bytes', () {
      check(
        'vocabulary',
        checkProperty<Uint8List>(
          BigInt.from(1301),
          PbtOptions(numTests: 300, seed: BigInt.from(1301)),
          (rng) => Gen.bytes(0, 300).sample(rng),
          (input) {
            for (var round = 0; round < 2; round++) {
              try {
                parseMessage(input);
              } on ResponseError {
                // Expected vocabulary.
              } catch (_) {
                return false;
              }
              try {
                final reader = MessageReader(input);
                var guard = input.length + 2;
                while (reader.readMessage() != null) {
                  if (--guard < 0) return false; // must always advance
                }
              } on ResponseError {
                // Expected vocabulary.
              } catch (_) {
                return false;
              }
            }
            return true;
          },
        ),
      );
    });

    test('messages round-trip through JSON', () {
      check(
        'message round-trip',
        checkProperty<Message>(
          BigInt.from(1302),
          PbtOptions(numTests: 150, seed: BigInt.from(1302)),
          (rng) => messageGen().sample(rng),
          (message) {
            final text = jsonEncode(messageToJson(message));
            return parseMessage(utf8.encode(text)) == message;
          },
        ),
      );
    });

    test('messages round-trip through framing', () {
      check(
        'framing round-trip',
        checkProperty<Message>(
          BigInt.from(1303),
          PbtOptions(numTests: 150, seed: BigInt.from(1303)),
          (rng) => messageGen().sample(rng),
          (message) {
            final reader = MessageReader(frameMessage(message));
            final back = reader.readMessage();
            if (back != message) return false;
            return reader.readMessage() == null;
          },
        ),
      );
    });

    test('server follows the dispatch model', () {
      // Streams of requests (half known, half unknown) plus
      // notifications: the model predicts every response byte.
      final item = Gen.oneOf([
        Gen.tuple2(idGen(), tokenGen(1, 8))
            .map((t) => Request(id: t.$1, method: 'known/${t.$2}')),
        Gen.tuple2(idGen(), tokenGen(1, 8))
            .map((t) => Request(id: t.$1, method: 'unknown/${t.$2}')),
        tokenGen(1, 8).map((m) => Notification(method: 'n/$m')),
      ]);
      check(
        'dispatch model',
        checkProperty<List<Message>>(
          BigInt.from(1304),
          PbtOptions(numTests: 120, seed: BigInt.from(1304)),
          (rng) => Gen.list(0, 8, item).sample(rng),
          (messages) {
            final input = BytesBuilder();
            for (final message in messages) {
              input.add(frameMessage(message));
            }
            final reader = MessageReader(input.toBytes());
            final writer = MessageWriter();
            final server = Server(reader: reader, writer: writer);
            server.onRequest('known/echo', (id, params) => params);
            // Register every known method in the stream.
            final known = {
              for (final m in messages)
                if (m is Request && m.method.startsWith('known/')) m.method
            };
            for (final method in known) {
              server.onRequest(method, (id, params) => params);
            }
            server.serve();

            // Model: one response per request, none per notification.
            final expected = <Response>[];
            for (final message in messages) {
              if (message is Request) {
                if (message.method.startsWith('known/')) {
                  expected
                      .add(Response(id: message.id, result: message.params));
                } else {
                  expected.add(Response(
                      id: message.id,
                      error: ResponseError.methodNotFound(message.method)));
                }
              }
            }
            final actual = <Message>[];
            final out = MessageReader(writer.bytes);
            while (true) {
              final message = out.readMessage();
              if (message == null) break;
              actual.add(message);
            }
            if (actual.length != expected.length) return false;
            for (var i = 0; i < actual.length; i++) {
              if (actual[i] != expected[i]) return false;
            }
            return true;
          },
        ),
      );
    });

    test('errors round-trip through JSON', () {
      check(
        'error round-trip',
        checkProperty<(int, String, Object?)>(
          BigInt.from(1305),
          PbtOptions(numTests: 150, seed: BigInt.from(1305)),
          (rng) => Gen.tuple3(
            Gen.integer(BigInt.from(-32768), BigInt.from(32767))
                .map((i) => i.toInt()),
            tokenGen(1, 16),
            jsonGen(2),
          ).sample(rng),
          (input) {
            final error = ResponseError(
                code: input.$1, message: input.$2, data: input.$3);
            return ResponseError.fromJson(error.toJson()) == error;
          },
        ),
      );
    });

    test('ids survive parse and serialize exactly', () {
      check(
        'id preservation',
        checkProperty<Object?>(
          BigInt.from(1306),
          PbtOptions(numTests: 150, seed: BigInt.from(1306)),
          (rng) => idGen().sample(rng),
          (id) {
            final back = parseMessage(utf8.encode(
                jsonEncode(messageToJson(Request(id: id, method: 'm')))));
            return back == Request(id: id, method: 'm');
          },
        ),
      );
    });

    test('parsing is pure', () {
      check(
        'purity',
        checkProperty<Uint8List>(
          BigInt.from(1307),
          PbtOptions(numTests: 150, seed: BigInt.from(1307)),
          (rng) => Gen.bytes(0, 300).sample(rng),
          (input) {
            Message? first;
            ResponseError? firstError;
            try {
              first = parseMessage(input);
            } on ResponseError catch (e) {
              firstError = e;
            }
            try {
              final second = parseMessage(input);
              if (firstError != null || first != second) return false;
            } on ResponseError catch (e) {
              if (firstError == null || e != firstError) return false;
            }
            return true;
          },
        ),
      );
    });
  });
}
