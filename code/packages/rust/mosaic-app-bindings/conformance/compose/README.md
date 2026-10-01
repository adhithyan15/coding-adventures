# Compose runtime conformance

This JVM console harness compiles the exact `MosaicRuntimeHost.kt` emitted into a
generated Compose Desktop project. It loads the shared `mosaic-app-conformance`
native library through JNA and verifies startup, revisions, prop projection,
generated flat-envelope semantic dispatch, snapshot/restore, notification,
buffer ownership, and teardown.

CI composes the Rust fixture with the adjacent `package/` Mosaic UI, emits a
strict Compose native distribution with `--runtime-library`, and verifies the
engine landed in the installed app resources. It then copies that exact
generated binding into this console harness and runs the full ABI lifecycle
with only `compose.application.resources.dir` set—never `MOSAIC_APP_LIBRARY`.
The binding itself is deliberately not duplicated here, and no Compose UI
runtime is required for the console round trip.

CI runs the harness across clean, restored, and incompatible-state launches
against one explicit state path.

It also checks UI48 ENV4 (environment reporting, spec §7.4) against the
conformance runtime, which ignores `environmentChanged`: an ignored report keeps
the props and revision, an unchanged one is not resent, and an invalid one is
refused and the identical one held back. With the conformance app's
`failEnvironment` switch on, a report that failed for the app's own reasons is
sent again rather than held back. With `MOSAIC_APP_STATE_PATH` set, an ignored
report does not rewrite the state file and raises no persistence warning, while
an event that cannot persist surfaces one at once.
