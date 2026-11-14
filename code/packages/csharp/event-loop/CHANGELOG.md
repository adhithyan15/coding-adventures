# Changelog

## Unreleased

- Fix a latent hanging test (same race as the F# port): A stop requested before `Run()` starts is erased (`Run()` resets the stop flag on entry), so a test that stops the loop from another thread after a fixed 10 ms sleep hangs forever whenever `Run()` starts late. On a loaded CI runner that hung the F# shard for 150 minutes until the step timeout. The stopping thread now waits for a source that signals "the loop is polling" before it stops. Test-only change; the library is unchanged. Verified by injecting a 100 ms delay before `Run()`: the old tests hang, the new ones pass.

## 0.1.0

- Add a C# generic event loop with event sources, handlers, control-flow signaling, and stop handles.
