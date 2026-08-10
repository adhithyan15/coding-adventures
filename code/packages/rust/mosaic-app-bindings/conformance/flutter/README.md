# Flutter runtime conformance

This headless Dart harness compiles the exact `mosaic_host.dart` emitted into a
generated Flutter project. It loads the shared `mosaic-app-conformance` native
library and verifies startup, revisions, prop projection, semantic dispatch,
snapshot/restore, notification, buffer ownership, and teardown.

CI generates the complete TaskApp project, copies its generated binding into
this package in a temporary workspace, and runs it with the Dart VM. The binding
itself is deliberately not duplicated here, and no Flutter engine is required.
