# capability-os-sandbox

`capability-os-sandbox` lowers `required_capabilities.json` manifests into
reviewable OS sandbox primitive plans.

The manifest remains the source of truth. This crate translates each declared
capability into a platform-specific defense-in-depth rule and labels the rule's
coverage:

- `direct`: an OS primitive can enforce the boundary.
- `brokered`: a host broker must mediate the operation.
- `launch_time`: the boundary is applied when the process is spawned.
- `advisory`: the OS primitive narrows the class of behavior but not the exact
  target.

Supported planning targets are Linux, macOS, Windows, FreeBSD, OpenBSD, and a
portable host-broker fallback.
