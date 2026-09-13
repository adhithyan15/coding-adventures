---
category: Mosaic compiler pipeline
---

# A host override that replaces a generated file can break the build, not just duplicate it

2026-09-13.

Engram's `host/flutter/mosaic_host.dart` overrode the generated Mosaic host and
defined `load()` but not `loadRequired()`, which the native-complete
`main.dart` calls. `flutter analyze` on the emitted project:

    error - The method 'loadRequired' isn't defined for the type 'MosaicHost'
          - lib/main.dart:9:43 - undefined_method

Nothing caught it because `mosaic/programs/engram-app` was absent from the
Flutter lane's `ACCEPTANCE_PACKAGES`, so no build had ever emitted Engram on
that backend with `--profile native-complete`. The lane was green because it
built task-app.

A package outside a lane's acceptance set is not "covered by the other
entries"; it is untested. When adding a backend override or a host adapter,
check the package is in the acceptance set of the lane that compiles it — and
add a test that fails without the entry.
