# Compose runtime conformance

This JVM console harness compiles the exact `MosaicRuntimeHost.kt` emitted into a
generated Compose Desktop project. It loads the shared `mosaic-app-conformance`
native library through JNA and verifies startup, revisions, prop projection,
semantic dispatch, snapshot/restore, notification, buffer ownership, and teardown.

CI generates the complete TaskApp project, copies its generated binding into
this harness in a temporary workspace, and runs the result. The binding itself
is deliberately not duplicated here, and no Compose UI runtime is required.
