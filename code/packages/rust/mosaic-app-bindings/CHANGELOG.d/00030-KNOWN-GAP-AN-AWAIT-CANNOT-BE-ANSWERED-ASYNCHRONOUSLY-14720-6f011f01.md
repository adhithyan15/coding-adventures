### Known gap -- an `Await` cannot be answered asynchronously (#14720)

Both this host and Qt's call the handler synchronously and fail any `await` not
answered by the time the call returns. Answering later is rejected as already
completed; blocking on another thread deadlocks against the lock. That rules out
an asynchronous file dialog, which is the motivating case for `await` effects --
so **step 5 cannot be completed on this shape**. Both templates now state the
constraint rather than implying a handler may answer whenever it likes.

`tests/swift_effect_completion.rs` emits the host, compiles it with `clang` +
`swiftc`, links the real conformance runtime and runs it -- eleven checks
including a batch where the handler answers both effects with chaining, which is
the shape that caught the equivalent Qt bug.

