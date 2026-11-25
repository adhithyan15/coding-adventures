# coding_adventures_in_memory_data_store_protocol (Dart)

In-memory data store protocol frames: the Dart lane.

A `CommandFrame` is the engine-side view of a request: an uppercased
command name plus raw byte arguments, converted to and from RESP wire
values. Arguments stay bytes end to end, so binary payloads survive.

## API

```dart
import 'package:coding_adventures_in_memory_data_store_protocol/coding_adventures_in_memory_data_store_protocol.dart';

CommandFrame('set', [bytes]);              // name uppercased, args copied
CommandFrame.fromParts(parts);             // null on empty
commandToResp(frame);                      // RESP array of bulk strings
commandFromResp(value);                    // null on null/empty/non-scalar
respValueToBytes(value);                   // scalars to bytes, else null
protocolError('message');                  // RESP error value
parseFrameText(['set', 'k']);              // text form in
formatFrame(frame);                        // 'SET k' text form out
```

* Conversions reject null arrays, empty arrays, nested arrays, and null
  bulk elements with null (no exceptions on the data path).
* Text helpers UTF-8 encode arguments; `fromParts` uppercases ASCII bytes
  only, matching the byte-oriented sibling lanes.
* Depends only on the in-lane `resp-protocol` package. Pure computation:
  no clock, no I/O, no randomness.

## Tests

```bash
dart pub get
dart test
```

The suite covers frames, both conversion directions, every scalar kind and
rejection, text round-trips, and a seeded 256-frame wire-conversion sweep.
Coverage gate: 95% branch+function.
