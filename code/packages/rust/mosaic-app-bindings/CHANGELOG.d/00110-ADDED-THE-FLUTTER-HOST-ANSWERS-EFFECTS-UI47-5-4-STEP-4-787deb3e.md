### Added -- the Flutter host answers effects (UI47 §5.4 step 4)

The fourth of five host templates, after Qt, SwiftUI and Compose. The host
gains `effectHandler`, `completeEffect`, `deferEffect` and the same bounded
settle loop; the emitted protocol version moves to `EFFECT_PROTOCOL_VERSION`.

Two things are genuinely different here rather than ported:

- **A Dart isolate is single-threaded.** The other three hosts hold a lock
  across the handler and have to warn against blocking, and Qt and SwiftUI both
  need a way to marshal a deferred answer back. None of that applies: an answer
  cannot arrive concurrently with a settle, only on a later turn of the event
  loop. So there is no lock, no deadlock to document, and nothing to marshal --
  and the acceptance's deferred case answers from a later event-loop turn
  rather than from another thread.
- **Two FFI paths, only one of which can be lenient.** The dynamic constructor
  resolves the seventh symbol through `lookupFunction`, which throws when it is
  absent, so it is wrapped and the slot is nullable -- a protocol-1 runtime
  still loads and only fails if an effect actually arrives. The bundled
  constructor uses `@Native`, which binds against the runtime linked into the
  process; a bundled protocol-1 runtime is a generation-time mismatch rather
  than something to recover from, which is the position the other six symbols
  were already in.

`completeEffect` needs two encoded inputs in one call, which no existing helper
covered, so `_invokeInputs` joins `_invokeInput`. Its allocations are made
inside the `try` and freed only if obtained: allocating first and entering the
`try` afterwards leaks everything already obtained if a later `calloc` throws,
and five allocations make that window widest here.

`_invokeInput`, the pre-existing single-input helper, gets the same treatment.
It had the original shape, and its first allocation is the payload buffer --
the arbitrarily large one of the three. Fixing the new helper and leaving its
twin leaking beside it would have been the wrong half of the job.

`_effectId` refuses a non-finite id. `double.infinity` is the one value that
satisfies the integrality test and still cannot be converted -- infinity equals
its own `roundToDouble()`, and `toInt()` then throws `UnsupportedError` -- and
`jsonDecode` produces it from `1e999` without complaint. A throw there escapes
the round loop, `_settleEffects` and `dispatch`, leaving every id already added
to `_awaiting` in that round with nothing to discharge it, and skipping the
warning that would have said so. `_failOutstanding`, the last-ditch clearing
path, calls it too. Qt and SwiftUI both guard finiteness explicitly and this
port had dropped it; NaN was already refused, because NaN compares unequal to
itself. Found by the security review. Reaching it needs a substituted or
corrupt library, since `EffectId` is a `u64` and serde_json emits integers, so
it is **not** exercised by the acceptance.

`_withPersistenceWarning` prefers the sticky effect warning over
`_persistenceWarning`, the fix Compose needed after review: the effect warning
means persistence is off for the rest of the process, while the other is
cleared by the next successful write, so reading only the latter would announce
that saving recovered while the runtime still refuses to snapshot. Written
correctly here from the start rather than found afterwards.

