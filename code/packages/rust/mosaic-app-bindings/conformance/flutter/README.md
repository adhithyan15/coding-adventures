# Flutter runtime conformance

This headless Dart harness compiles the exact `mosaic_host.dart` emitted into a
generated Flutter project. It loads the shared `mosaic-app-conformance` native
library and verifies startup, revisions, prop projection, semantic dispatch,
snapshot/restore, notification, buffer ownership, and teardown.

CI generates the complete TaskApp project, copies its generated binding into
this package in a temporary workspace, and runs it with the Dart VM. The binding
itself is deliberately not duplicated here, and no Flutter engine is required.

CI relaunches it with the same explicit state path, then with an incompatible
snapshot, to prove restart restoration and recoverable quarantine.

It also checks UI48 ENV4 (environment reporting, spec §7.8) against the
conformance runtime, which ignores `environmentChanged`: the six reported
values and thresholds, that an ignored report has nothing new to show and keeps
the props and revision, and that an invalid report comes back as an `error`
answer (never thrown) and the identical one is held back. With the conformance
app's `failEnvironment` switch on, every report that reaches the app fails, so
an `error` answer proves a report was sent and null that it was held back: the
refused report and an unchanged one are not sent, a changed one is, and one
that failed for the app's own reasons is sent again rather than held back.
With `MOSAIC_APP_STATE_PATH` set it also checks that an ignored report does not
rewrite the state file, and -- with the state path made a directory -- raises no
persistence warning while an event that cannot persist surfaces one at once,
and that once the path is writable again the next ignored report retries the
failed save and shows the warning cleared.
