/// Hostile-input fuzz campaign for `coding_adventures_json_rpc`, using
/// the in-lane `fuzz` package: raw byte strings drive the reader to EOF,
/// every parsed message round-trips through the writer and back, and a
/// `serve` smoke run with echo handlers must never throw. Only
/// [ResponseError] escapes the read path. Fixed seed, so the campaign
/// reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:coding_adventures_json_rpc/json_rpc.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  cov.recordEdge(input.length % 64);
  try {
    // Drain the reader: every message must re-serialize and re-parse to
    // an equal message, and the cursor must always advance.
    final reader = MessageReader(input);
    var guard = input.length + 2;
    var count = 0;
    while (true) {
      Message? message;
      try {
        message = reader.readMessage();
      } on ResponseError {
        cov.recordEdge(100 + (count % 4));
        break;
      }
      if (message == null) break;
      if (--guard < 0) {
        return const OutcomeCrash('model-divergence', ['no-progress']);
      }
      count++;
      final writer = MessageWriter();
      try {
        writer.writeMessage(message);
      } on JsonUnsupportedObjectError {
        // Only documented case: non-finite doubles like 1e999 parse
        // (both lanes) but cannot serialize (both lanes fail the write).
        // Nothing to round-trip; the reader already proved the parse.
        cov.recordEdge(105);
        continue;
      } catch (e) {
        return OutcomeCrash('unexpected-escape', ['write:${e.runtimeType}']);
      }
      final reread = MessageReader(writer.bytes).readMessage();
      if (reread != message) {
        return const OutcomeCrash('model-divergence', ['round-trip']);
      }
    }
    cov.recordEdge(110 + (count % 8));

    // The server loop never throws on hostile input.
    final server =
        Server(reader: MessageReader(input), writer: MessageWriter());
    server.onRequest('echo', (id, params) => params);
    server.onNotification('n', (_) {});
    try {
      server.serve();
    } catch (e) {
      return OutcomeCrash('unexpected-escape', ['serve:${e.runtimeType}']);
    }
    cov.recordEdge(120);
    return const OutcomePass();
  } on ResponseError {
    cov.recordEdge(130);
    return const OutcomePass();
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['read:${e.runtimeType}']);
  }
}

void main() {
  test('hostile campaign finds no escapes or divergences', () {
    Uint8List frame(String json) {
      final payload = utf8.encode(json);
      return Uint8List.fromList([
        ...ascii.encode('Content-Length: ${payload.length}\r\n\r\n'),
        ...payload,
      ]);
    }

    final seeds = <Uint8List>[
      Uint8List(0),
      Uint8List.fromList([0xFF, 0xFE]),
      frame('{"jsonrpc":"2.0","id":1,"method":"ping"}'),
      frame('{"jsonrpc":"2.0","id":1,"result":{"ok":true}}'),
      frame(
          '{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}'),
      frame('{"jsonrpc":"2.0","method":"n","params":[1,"x"]}'),
      Uint8List.fromList([
        ...frame('{"jsonrpc":"2.0","id":1,"method":"a"}'),
        ...frame('{"jsonrpc":"2.0","method":"b"}'),
      ]),
      Uint8List.fromList(utf8.encode('Content-Length: 5\r\n\r\nabc')),
      Uint8List.fromList(utf8.encode('Content-Length: nope\r\n\r\n{}')),
      Uint8List.fromList(utf8.encode('Content-Length: 2')),
      Uint8List.fromList(
        utf8.encode('not framed at all'),
      ),
      frame('{"jsonrpc":"2.0","id":18446744073709551615,"method":"m"}'),
      frame('{"jsonrpc":"2.0","id":1,"method":"\\ud800"}'),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(1308)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
