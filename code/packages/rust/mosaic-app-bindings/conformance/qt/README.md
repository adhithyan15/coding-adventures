# Qt runtime conformance

This headless Qt Core harness compiles the exact `MosaicHost.h/.cpp` emitted into
a generated Qt/QML project. It loads the shared `mosaic-app-conformance` native
library and verifies startup, revisions, prop projection, semantic dispatch,
snapshot/restore, buffer ownership, and teardown.

CI generates the complete TaskApp project, copies its generated binding into
this harness in a temporary workspace, and runs the result. The binding itself
is deliberately not duplicated here, and no graphical display is required.
