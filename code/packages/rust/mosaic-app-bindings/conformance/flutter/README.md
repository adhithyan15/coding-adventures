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
the props and revision, that an invalid report comes back as an `error` answer
(never thrown) and the identical one is held back, and -- with
`MOSAIC_APP_STATE_PATH` set, since every dispatch rewrites the state file --
that an unchanged report is not sent and a changed one is.
