---
category: Rust
---

# A supervisor that settles a child as exited must first drain the reader thread's last events

`chief-of-staff-process-supervisor` reads a child's stdout on a background
thread that forwards records (and a final end-of-stream `Failure`) over a
channel.  `refresh` drained the channel, then called `try_wait`; if the child
had exited it went straight to `Exited`, and every later `refresh` returned
early.  When the child exited in the window between the drain and the reader
queueing its EOF failure, that failure was never read: a child that died
before `Ready` looked like a clean exit, and
`wrong_ready_and_exit_before_ready_fail_closed` failed intermittently on
macOS CI ("failure was not observed").

Reproduce such races deterministically before fixing them: a temporary
`thread::sleep` in the reader just before it sends the failure made the test
fail every run, and pass every run after the fix.

The fix: once the process is known to have exited, join the reader thread
(once it returns, everything it will ever send is queued; this is the same
join `finish_exit` already did, so it waits no longer than before)
and drain the channel a second time *before* committing to the terminal
phase.  General rule: any state machine with a terminal state that stops
consulting an event source must flush that source, including producer
threads still in flight, before it enters the terminal state.
