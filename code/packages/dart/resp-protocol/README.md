# coding_adventures_resp_protocol (Dart)

RESP wire protocol: the Dart lane of
[DT23](../../../specs/DT23-resp-protocol.md).

RESP frames every Redis request and reply as typed values: short status and
error lines, integers, counted binary-safe bulk strings, and nested arrays.
A streaming decoder reassembles messages from arbitrary TCP chunks.

## API

```dart
import 'package:coding_adventures_resp_protocol/coding_adventures_resp_protocol.dart';

// Values
simpleString('OK'); errorValue('ERR boom'); integer(42);
bulkString('foo'); bulkString(bytes); bulkString(null);
array([bulkString('SET'), bulkString('k')]); array(null);

// Encoding (total)
Uint8List wire = encode(value);

// Decoding (null = truncated feed more; throws RespDecodeError = malformed)
RespDecodeResult? one = decode(wire);
RespDecodeAllResult many = decodeAll(pipelined);

// Streaming
final decoder = RespDecoder();
decoder.feed(chunk1); decoder.feed(chunk2);
while (decoder.hasMessage()) { handle(decoder.getMessage()); }
```

* Integer and length parsing is strict (optional minus, digits only, 64-bit
  range), matching the strict sibling lanes; `int.tryParse` alone would admit
  whitespace and `+`.
* Decoders copy payload bytes out: later input never aliases a value.
* Malformation throws out of `feed` immediately, like the sibling lanes.
* Hardening beyond the reference: overflow-safe bulk bounds (near-max
  lengths truncate instead of indexing out of range), a 128-deep nesting
  cap (deeper frames throw instead of overflowing the stack), UTF-8
  failures mapped to `RespDecodeError`, and copy-on-construct arrays.
* Zero dependencies. Pure computation: no clock, no I/O, no randomness.

## Tests

```bash
dart pub get
dart test
```

The suite mirrors the sibling battery (exact wire bytes, command arrays,
inline commands, incremental buffering, pipelining), pins null forms,
nesting, CRLF-bearing binary payloads, and every-prefix truncation, runs a
malformed-input battery, and sweeps 512 seeded values through
encode/decode. Coverage gate: 95% branch+function.
