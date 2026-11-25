# coding_adventures_json_rpc (Dart)

JSON-RPC 2.0 transport layer with Content-Length framing. The Dart lane
of the Rust `json-rpc` crate.

```dart
import 'package:coding_adventures_json_rpc/json_rpc.dart';

final server = Server(
  reader: MessageReader(bytes),
  writer: MessageWriter(),
);
server.onRequest('ping', (id, params) => 'pong');
server.onNotification('tick', (params) => print('tick'));
server.serve(); // runs to EOF
```

## API

- `parseMessage` parses JSON bytes into `Message` (`Request` /
  `Response` / `Notification`); `messageToJson` serializes back with a
  fixed key order. Failures throw `ResponseError`.
- `MessageReader` reads framed messages from bytes (`readMessage` /
  `readRaw`, `null` at clean EOF); `MessageWriter` writes them
  (`writeMessage` / `writeRaw`, output via `bytes`).
- `Server` dispatches to `onRequest` / `onNotification` handlers.
  Request handlers return the result or throw `ResponseError`;
  anything else they throw becomes `internalErrorCode`, like the
  reference's `catch_unwind`.
- `ResponseError` carries the spec code table (`parseErrorCode` …
  `internalErrorCode`) with throwing factories, strict `fromJson`,
  and `toJson` omitting absent `data`.

JSON values are plain `Object?` (`dart:convert`, the established
in-lane story); equality is deep with serde number semantics (`1`
does not equal `1.0`).

## Deliberate divergences from the reference

- Reader, writer, and server work over in-memory bytes, not generic
  `BufRead`/`Write` streams: a stream-backed reader for live stdio is
  future work for the LSP-server lane. Framing behavior is identical,
  including `read_exact` consuming short payloads.
- Integers outside int64 fail closed with a parse error: `dart:convert`
  would decode them as lossy doubles where serde keeps exact u64.
- Lone UTF-16 surrogates fail closed on both paths (parse error on
  read, `ArgumentError` on write): `dart:convert` admits `\uD800`
  escapes that serde rejects, and they corrupt `utf8.encode`.
- A leading BOM fails closed with a parse error (serde rejects it;
  `utf8.decode` would strip it) — except in framing headers, where
  the BOM is preserved like `read_line` so the prefix match fails
  exactly as it does in the reference.
- Serialization failures in `writeMessage` propagate
  `JsonUnsupportedObjectError` (no IO layer exists to wrap them);
  `serve` drops them like the reference's `let _ =`.
- Short-payload and header error `data` texts mirror the reference's
  io-error wordings; codec-derived texts are platform-diagnostic.

## Validation

- **Unit** (`test/json_rpc_test.dart`): all 5 reference server tests
  plus the full matrix the reference leaves untested (message
  discrimination, errors, framing edges, writer exactness, values).
- **Unit** (`test/json_values_test.dart`): the shared JSON machinery
  (deep equality, int64 scan, surrogate rejection).
- **Property** (`test/properties_test.dart`): fail-closed vocabulary,
  message/framing round-trips, the server dispatch model, error
  round-trips, id preservation, purity — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  with read/write round-trip integrity; only `ResponseError` escapes.
- **Coverage**: 100% lines, branches, and functions.
