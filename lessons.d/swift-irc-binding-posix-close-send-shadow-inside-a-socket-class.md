# Swift IRC binding — POSIX `close`/`send` shadow inside a socket class

**Date:** 2026-06-14

The Swift IRC port (`IrcServerNative`, on the reusable `irc-server-capi` C ABI —
the same pattern as `conduit-capi`) reuses Conduit's raw POSIX-socket test client.
Gotcha: a class with a method named `close()` (or `send()`) **shadows the global C
`close`/`send`** inside its own body — `close(fd)` resolves to the instance method
and fails to compile (`use of 'close' refers to instance method rather than global
function`). Even inside `init`, before the method exists conceptually, the name
resolves to the member. Fix: wrap the C calls in free functions
(`private func cclose(_ fd: Int32) -> Int32 { close(fd) }`) and call those from the
class. Same `SOCK_STREAM` enum-vs-macro normalization as Conduit applies.

This completed the all-Rust IRC engine port to **all 8 targets** (Rust engine +
Python/Ruby/Node/Java/Elixir/Perl/Swift). The native-binding effort surfaced four
real engine bugs, each fixed engine-wide with a regression test: panic containment
(`catch_unwind` + poison-tolerant locks), RST survival (close-now instead of `?`
propagation), the stop-before-serve race (don't reset the stop flag at loop entry),
and an Elixir concurrent-resource data race (`Mutex` around the inner handle).
---
