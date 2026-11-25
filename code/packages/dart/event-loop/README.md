# coding_adventures_event_loop (Dart)

Pluggable generic event loop. The Dart lane of the Rust
`event-loop` crate.

```dart
import 'package:coding_adventures_event_loop/event_loop.dart';

final loop = EventLoop<String>();
loop.addSource(_MySource());
loop.onEvent((event) {
  print(event);
  return event == 'quit' ? ControlFlow.exit : ControlFlow.continueLoop;
});
loop.run(); // blocks until exit
```

## API

- `EventLoop<E>` — `addSource` (polled in registration order),
  `onEvent`, `stopHandle`, `stop`, and blocking `run` with
  collect → dispatch → idle phases.
- `EventSource<E>` — `poll()` returning available events,
  immediately, never blocking.
- `ControlFlow.continueLoop` / `ControlFlow.exit` (`continue`
  is a Dart keyword, hence the name).
- `StopHandle.stop()` — exits on the next iteration; the flag is
  checked at the top of each loop, so in-flight dispatch
  finishes first.

`run()` resets the stop flag on entry, so loops re-run cleanly.
Unlike the reference, the idle phase cannot yield to the OS
from synchronous Dart code; this deliberate difference is the
only behavioral gap from the reference.

## Validation

- **Unit** (`test/event_loop_test.dart`): mirrors all 8
  reference tests (the two delivery tests merge — Dart closures
  capture mutably), plus handler short-circuit, source order,
  dispatch-then-stop, re-run, handle sharing, and throw
  propagation. 13 tests.
- **Property** (`test/properties_test.dart`): delivery order,
  exit position, fan-out, source order, determinism, and
  stop-handle termination — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile multi-source batch scripts (an exit
  sentinel always terminates the script); the API is total, so
  no errors escape. Ad-hoc 20,000-iteration runs over two more
  seeds: zero crashes.
- **Coverage**: 100% lines; functions 8/9 (the synthetic enum
  constructor is uncoverable, as in the RWS lane).
