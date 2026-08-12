# Compose runtime conformance

This JVM console harness compiles the exact `MosaicRuntimeHost.kt` emitted into a
generated Compose Desktop project. It loads the shared `mosaic-app-conformance`
native library through JNA and verifies startup, revisions, prop projection,
semantic dispatch, snapshot/restore, notification, buffer ownership, and teardown.

CI composes the Rust fixture with the adjacent `package/` Mosaic UI, emits a
strict Compose native distribution with `--runtime-library`, and verifies the
engine landed in the installed app resources. It then copies that exact
generated binding into this console harness and runs the full ABI lifecycle
with only `compose.application.resources.dir` set—never `MOSAIC_APP_LIBRARY`.
The binding itself is deliberately not duplicated here, and no Compose UI
runtime is required for the console round trip.
