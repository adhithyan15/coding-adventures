# capability-os-sandbox

`capability-os-sandbox` lowers `required_capabilities.json` manifests into
reviewable OS sandbox primitive plans.

The manifest remains the source of truth. This crate translates each declared
capability into a platform-specific defense-in-depth rule and labels the rule's
coverage:

- `direct`: an OS primitive names the exact declared target. Today that is only
  exact-path filesystem rules. Network, process and wildcard grants are
  brokered on every platform, and `ffi` is unsupported everywhere.
- `brokered`: a host broker must mediate the operation.
- `launch_time`: the boundary is applied when the process is spawned.
- `advisory`: the OS primitive narrows the class of behavior but not the exact
  target. Never a grant: no lowering produces it, and a plan containing one
  cannot launch (D18S S-P2).
- `unsupported`: the platform cannot enforce the capability at all, which is a
  launch failure, never a quiet downgrade (S-P3).

Every plan also carries a **base deny** (`SandboxPlan.base`, D18S S-I1a). It is
the deny-all policy for the OS, built without looking at the manifest, so a
manifest with no capabilities yields the strictest plan rather than an empty
one. Grants subtract from the base. `SandboxPlan::launch_preconditions()` says
whether a plan may launch, and `run_with_kernel_sandbox` checks it first. It
re-derives every rule and the base rather than trusting the plan's public
fields, and it refuses the portable target, which has no kernel boundary. The
base is modelled today but not yet installed by the appliers: see the
CHANGELOG.

Supported planning targets are Linux, macOS, Windows, FreeBSD, OpenBSD, and a
portable host-broker fallback.
