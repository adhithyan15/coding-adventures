### Fixed — only an invalid environment is held back, and an ignored report writes no state (UI48 ENV4)

Three findings from the security reviews of the Qt, XAML and Flutter ENV4
work, fixed the same way on every host that reports its environment (Qt,
XAML, Flutter, Compose, SwiftUI):

- **A transient failure no longer suppresses a retry.** Qt, XAML and Flutter
  remembered any failed report as refused and never sent it again, so after
  one app error the app could stay on a stale environment until the window
  changed to a third one. Every host now holds an identical report back only
  when the runtime refused it as invalid -- its diagnostic begins with
  `mosaic-app-runtime`'s `INVALID_ENVIRONMENT_DIAGNOSTIC`, which
  `bind_application` writes into each host from the new
  `__MOSAIC_INVALID_ENVIRONMENT__` placeholder (the ABI status cannot tell:
  an invalid payload shares `ProtocolError` with a sequence mismatch). Any
  other failure is answered as before and the same report is sent again.
  Compose and SwiftUI, which remembered no refusal at all and so re-sent an
  invalid report on every change, now hold back the invalid one like the
  others. The Swift host's dispatch moved into a throwing `dispatchEvent`
  that `handleEvent` and `reportEnvironment` share, so the report can see the
  runtime's diagnostic.
- **An ignored report no longer rewrites the state file.** Every host's
  dispatch persisted unconditionally, so a resize storm fsynced the state
  file per report. The dispatch now persists unless the state file already
  holds the answer's revision -- each host remembers the revision of its
  last successful save, so the first answer after launch always writes, even
  an ignored environment on a fresh install -- or an earlier save failed (a
  persistence warning is pending); an unreadable revision persists. An
  environment the app ignored changed nothing the app saves, but a failed save at revision N is retried by the
  next ignored report, so a kill before the next event does not lose N.
  Ordinary events and effect answers always move the revision, so they
  persist as before.
- **A warning an ignored report sets or clears is shown as an event's is.**
  Only such a retried save can change it. Qt, Compose and SwiftUI already
  hand back or push the kept props with the warning folded in; Flutter's
  `reportEnvironment` now answers with the update instead of null when the
  warning changed, and XAML's re-applies the props and answers a status line
  instead of null. With saving healthy, no warning can arise from one.
- Tests: a cross-host string test pins the diagnostic (once, from the
  runtime), the refusal memory behind it, the persist condition and the
  warning surfacing in all five hosts; another pins that the diagnostic is
  only ASCII letters and spaces, so it cannot break out of any host's string
  literal; the Compose, XAML and Flutter test pins follow the new shapes. The Qt driver, the XAML, Flutter and Compose conformance harnesses
  and the Swift driver use the conformance app's new `failEnvironment` switch
  to check that a report that failed transiently is sent again, that an
  invalid one is still held back and holds back nothing else, and (Qt, XAML,
  Flutter, Compose) that an ignored report neither rewrites the state file nor
  raises a warning while an event that cannot persist surfaces one at once,
  and (all five) that once saving works again the next ignored report retries
  the failed save and shows the warning cleared.
  The harnesses no longer read the state file to see whether a report was
  sent; with the switch on, a sent report answers an error.
  The Qt driver also checks that a fresh host whose first answer is an
  ignored report still writes its state file -- the case the Android
  emulator gate caught, where TaskApp's fresh install never saved.
