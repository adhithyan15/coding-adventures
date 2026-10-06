### Added — the XAML host tells the window when a deferred effect is answered

- `MosaicRuntimeHost.PropsChanged` (XAML): raised once per deferred effect
  answered, on the thread that answered, after the host's lock is released.
  A deferred answer moves the app with no call from the window, so before
  this the window saw it only at the next dispatch or environment report
  (UI87 §7.6 known gap). An answer inside the handler that was offered the
  effect raises nothing -- the dispatch that minted it returns it. A handler
  that throws is logged, never turned into a failed answer; closing the
  runtime drops the handler. Compose's `propsChangedHandler` and Flutter's
  `setPropsChangedHandler`, for WinUI.
- The XAML effect driver checks it is raised once for a deferred answer,
  and not for the deferral or for an answer given in the handler.
