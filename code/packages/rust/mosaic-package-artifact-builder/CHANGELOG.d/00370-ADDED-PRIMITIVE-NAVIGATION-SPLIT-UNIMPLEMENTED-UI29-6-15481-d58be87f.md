### Added — `primitive.navigation-split-unimplemented` (UI29-6, #15481)

`HostNavigationSplit` is registered before any backend lowers it, so every
native backend reports it rather than quietly emitting two containers with no
collapse and no pane semantics. This is the `HostSwitch` → `HostProgressRing`
lifecycle: unconditional at registration, then narrowed one backend at a time,
in the same PR that adds each lowering, so the report cannot outlive the gap or
precede the fix.

