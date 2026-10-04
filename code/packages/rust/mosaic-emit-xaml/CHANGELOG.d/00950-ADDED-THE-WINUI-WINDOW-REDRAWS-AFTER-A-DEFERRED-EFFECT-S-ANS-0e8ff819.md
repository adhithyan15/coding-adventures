### Added — the WinUI window redraws after a deferred effect's answer

- The runtime-backed WinUI window sets `MosaicRuntimeHost.PropsChanged` right
  after `LoadRequired` and re-applies the required props from its
  `DispatcherQueue` when a deferred effect is answered (a file dialog's
  answer, UI87 §7.6) -- to the root showing in a window that switches
  layouts. Before this the window showed the answer only at the next event.
- The refresh does nothing once the runtime is closed: an answer that raced
  `Close()` no longer reports a stale "no runtime" failure on the status
  line.
