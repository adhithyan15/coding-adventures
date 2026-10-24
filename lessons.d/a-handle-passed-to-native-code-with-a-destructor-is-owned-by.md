---
category: C#
---

# A handle passed to native code with a destructor is owned by native code; freeing your copy too is a silent double free

`conduit-capi`'s C ABI takes a `ctx` and a `ctx_free` for every
registered callback, and calls `ctx_free(ctx)` exactly once when the
callback drops. That happens in `conduit_app_free`, or in
`conduit_server_bind`, which consumes the app whether the bind succeeds
or fails. The C# and F# bindings passed a `GCHandle` as `ctx` with a
`CtxFree` that calls `GCHandle.Free()`. Then `Application.Dispose`
*also* looped over its own list of the same handles and freed them,
"in case Bind failed".

Nothing reported this. `GCHandle` is a struct, so the copy in the list
still said `IsAllocated`, and .NET's `Free()` does not detect a second
free. The slot was simply returned to the handle table twice. Two later
`GCHandle.Alloc` calls then got the same slot, and a trampoline cast a
handle to the wrong delegate. Tests saw intermittent wrong HTTP
statuses, which a comment blamed on a "cold-start Tokio race" (the capi
doesn't use Tokio), and CI saw "Test host process crashed" (PR #18005,
shard 15).

The after-hook trampolines (C#, F#, Dart) also freed the `current`
response and then freed it again in their `catch` when the hook threw.
That is a native double free.

**Fix:** after a handle is passed with its destructor, managed code never
frees it. `Dispose` only frees the app. Each trampoline clears its
`current` once freed and frees it in `catch` only while still owned.
Regression test: dispose an app, then check that two fresh
`GCHandle.Alloc`s differ.

**Do differently:** when an FFI API takes a destructor, write down which
side owns the resource and audit every release path against it, the
"just in case" cleanup paths especially. Don't accept a "known race" note
that blames timing for wrong results without a mechanism. Check the
other bindings of the same C ABI too: here Go, Swift, C++ and Haskell
were already right, and Dart had the trampoline bug.
