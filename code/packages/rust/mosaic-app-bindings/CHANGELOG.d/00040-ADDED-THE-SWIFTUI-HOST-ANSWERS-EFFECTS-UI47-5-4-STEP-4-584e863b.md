### Added -- the SwiftUI host answers effects (UI47 §5.4 step 4)

The second of five host templates, after Qt. `MosaicRuntimeHost` gains
`effectHandler`, `completeEffect(_:_:)` and a settle loop; the C shim resolves
`mosaic_app_complete_effect` leniently and reports its absence as a distinct
status, so "this runtime cannot complete effects" is not mistaken for "the
completion failed". The host declares protocol 2, and the assertion pinning the
version pins the capability beside it.

Written with the failure modes the Qt template surfaced already in hand --
accumulate at the write site, discharge on both runaway guards, bound the
nesting -- so those did not have to be rediscovered. Two things differed, and
both are recorded because the difference is the interesting part:

- **`NSRecursiveLock` makes re-entrant completion safe** where Qt needed care:
  a handler answering from inside the callback re-enters the lock on the same
  thread. Answering from another thread is not useful -- that call blocks until
  the settle finishes, by which point the effect has been failed as unanswered.
- **No effect-id validation is needed.** Qt's `completeEffect` is `Q_INVOKABLE`
  and takes a `QVariant`, so QML could hand it `3.5` and `toULongLong` would
  truncate to `4`, answering a *different* outstanding effect. Swift's signature
  is `UInt64`; the same mistake does not compile.

Bugs found by running it, none of which a text-level assertion could see:

- The re-entrant branch was gated on `latestAnswer != nil`, but that starts
  `nil` each frame and is only set *by* that branch -- so it was unreachable
  and every handler's answer was discarded. A direct port of Qt's
  pointer-non-null check into Swift, where the value itself is the Optional.
- **Answering with a `URL` aborted the process.** `JSONSerialization` raises an
  ObjC `NSInvalidArgumentException` for a `URL`, `Date`, `Data`, or a
  non-finite `Double`. Swift cannot catch that, so the surrounding `do/catch`
  was inert and the app terminated with the lock held -- on the first thing a
  real file-dialog handler would write. Now refused with a message.
- A JSON `true` bridges to `NSNumber` and coerced to id **1**, answering effect
  1. Booleans are refused.
- An `await` whose id could not be read was skipped in silence, leaving it
  pending forever. It is now counted and surfaced.
- Either runaway guard returned `["props": [:]]`, blanking every binding in the
  UI until the next event. The error is now attached to the last known good
  update instead of replacing it.
- The warning about abandoned effects was written to `persistenceWarning`,
  which `persistSnapshot()` clears on every successful write -- so it was
  overwritten microseconds later and never reached anyone. It has its own
  field now, because an abandoned effect is a standing condition rather than a
  write failure.

