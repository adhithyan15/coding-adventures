### Fixed — the XAML platform library answers only the runtime that asked (UI87 §7.6)

- `MosaicRuntimeHost` (XAML) gains `EffectScope`: a handle on the runtime
  loaded when it was taken (`EffectScope.Current()`), whose `DeferEffect`
  refuses once that runtime is no longer loaded and whose `CompleteEffect`
  throws `ObjectDisposedException` once it is closed. The static
  `CompleteEffect` answers whichever runtime is loaded, and a retried start
  (`Close`, then `LoadRequired`) restarts effect ids at 1 -- so a picker
  left open across the retry settled the new runtime's unrelated effect 1
  with a stale answer. `MosaicRuntimeHostEffects` now wraps the scope taken
  at install (it is no longer a singleton over the statics), so the late
  answer meets the closed runtime and is dropped. The effect-completion
  driver's new `scoped` case checks this against the conformance runtime
  (it fails with the static call: "unknown or completed Mosaic effect").
- The router copies the payload before marking itself busy, and releases
  the busy flag if deferral throws, so nothing can strand it.
- The chosen file is read (and base64-encoded) or written and flushed on the
  thread pool, not the UI thread; the answer is still given on the UI thread.
- On Windows, saving over an existing file uses `File.Replace`
  (`ignoreMetadataErrors`), keeping its ACL, attributes and alternate
  streams, and falls back to `File.Move` for a new file.
- The headless harness gains a runtime-swap case and waits for answers that
  now arrive from the thread pool (219 checks).

