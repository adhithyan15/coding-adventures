# coding_adventures_zeroize (Dart)

Best-effort in-memory wiping for secrets. The Dart lane of the Rust
`zeroize` crate.

```dart
import 'package:coding_adventures_zeroize/zeroize.dart';

final key = ZeroizingBytes(secretBytes);
try {
  use(key.bytes);
} finally {
  key.dispose(); // wipes secretBytes
}
```

## API

- `zeroizeBytes(bytes)` — overwrites every byte with zero; the
  `ZeroizeBytes` extension offers the same as `bytes.zeroize()`.
- `ZeroizingBytes(bytes)` — owning wrapper: `bytes` accessor,
  idempotent `dispose()`, `takeBytes()` escape hatch (releases
  without wiping), `isDisposed` flag. Post-dispose use throws
  [StateError]. Identity equality only — never value equality on
  secrets.

## Honest platform limits

Dart provides no volatile stores, no compiler fences, and no
destructors, so this lane cannot reproduce the reference's
observable-wipe guarantee: wiping is a plain overwrite that must be
triggered by explicit `dispose()` (use `try`/`finally`), GC copies
may linger, strings cannot be wiped at all (there is deliberately no
string API), and integers are immutable values with no wipe
equivalent. What is guaranteed and tested: after disposal the live
buffer reads back all-zero. See the library docs for the full
statement.

## Validation

- **Unit** (`test/zeroize_test.dart`): mirrors the portable
  reference tests (fill, empty noop, extension, wrapper
  read/mutate, dispose wipes, take opts out, finally-on-exception,
  idempotence, post-dispose errors, identity, view aliasing).
  12 tests.
- **Property** (`test/properties_test.dart`): wipe totality, API
  agreement, the dispose state machine, take semantics, and the
  anti-property pinning that nothing wipes without `dispose()` —
  fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration op-script
  campaign (fill, wipe, wrap, dispose, take, read, mutate) against
  a byte-exact liveness model; only [StateError] escapes. Ad-hoc
  20,000-iteration runs over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
