### Added -- an execution acceptance for the Flutter host

`tests/flutter_effect_completion.rs` emits the host into a temporary Dart
package, resolves `ffi` from the local pub cache with `dart pub get --offline`
so the test never depends on pub.dev, and runs it against the conformance
runtime -- one process per scenario, each with its own state file, because the
host reads `MOSAIC_APP_STATE_PATH` once at load.

Eight scenarios, matching the Compose set: an unanswered await, an answered
one, a fully-answered chaining batch, a partly-answered batch, a throwing
handler, an unconvertible result, a runaway chain, and defer-then-answer-later.
It skips when `dart` is absent or the cache cannot resolve `ffi`.

Mutation-tested: removing the handler-throw guard fails the `throwing` case
with the exception escaping `_settleEffects` into `dispatch`, and making the
round-exhaustion path give up quietly fails "a runaway chain is reported rather
than abandoned quietly".

