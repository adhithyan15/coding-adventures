---
category: Mosaic compiler pipeline
---

# Deferred work that answers through a static host facade answers whichever runtime is loaded then, not the one that asked

**What went wrong.** The XAML platform library (UI87 §7.6) deferred each
`files.open` / `files.save`, showed a picker, and answered later through
`MosaicRuntimeHost.CompleteEffect` -- a static facade over "the runtime
loaded now". The generated WinUI window can `Close()` and `LoadRequired()`
again (the startup retry), and a fresh runtime's effect ids restart at 1. So
a picker left open across a retry answered the NEW runtime, and the stale
answer settled its unrelated effect 1. The security review caught it; the
effect driver, run against the conformance runtime with the static call
put back, confirmed it (old id 1, new id 1, the new effect settled by the
stale answer). The Compose and SwiftUI libraries never had the bug because
their hosts are instances the router holds.

**Fix.** `MosaicRuntimeHost.EffectScope`: a handle bound to the runtime
loaded when it is taken. Its `DeferEffect` refuses once that runtime is no
longer loaded, and its `CompleteEffect` throws `ObjectDisposedException`
once it is closed. The library installs through a scope taken at install
time, and its `TryComplete` drops the refused answer.

**Do differently.** When code captures "the host" to answer asynchronously,
check whether the host is an object or a static facade over a resettable
one. A facade re-resolves its target on every call, so anything that
outlives a reset -- a dialog, a timer, a network call -- must hold the
instance (or a generation it can compare), and a test should reset between
the defer and the answer and assert the new instance saw nothing.
